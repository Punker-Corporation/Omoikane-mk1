use crate::{EntityManager, EntityUid};
use butsuri::{
    AabbShape, BodyType, CircleShape, CollisionRay, ContactManager, ContactManifold,
    ContactManifoldPoint, Fixture, PhysShape, RayCastHit, Transform as PhysicsTransform,
};
use keisan::{Box2, Vector2};
use std::collections::{BTreeSet, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub struct PhysicsQueryHit {
    pub entity: EntityUid,
    pub fixture_id: String,
    pub distance: f32,
    pub hit_pos: Vector2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicsStepState {
    pub linear_velocity: Vector2,
    pub angular_velocity: f32,
    pub auto_clear_forces: bool,
    pub awake_changed: bool,
    pub awake: bool,
    pub state_changed: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SharedPhysicsSystem;

impl SharedPhysicsSystem {
    pub(crate) fn new() -> Self {
        Self
    }

    const SLEEP_LINEAR_TOLERANCE_SQ: f32 = 0.0001;
    const SLEEP_ANGULAR_TOLERANCE: f32 = 0.01;
    const SLEEP_TIME_THRESHOLD: f32 = 0.5;

    pub(crate) fn step_body(
        &self,
        manager: &mut EntityManager,
        uid: EntityUid,
        frame_time: f32,
    ) -> Option<PhysicsStepState> {
        let map_id = manager.transforms.get(&uid)?.map_id;
        let entity_paused = manager.is_entity_paused(uid);
        let map_paused = manager.is_map_paused(map_id);
        let (gravity, auto_clear_forces) = manager
            .map_entity_for(map_id)
            .map(|owner| {
                manager
                    .physics_maps
                    .get(&owner)
                    .map(|map| (map.gravity, map.auto_clear_forces))
                    .unwrap_or((Vector2::ZERO, false))
            })
            .unwrap_or((Vector2::ZERO, false));
        let Some(body) = manager.physics.get_mut(&uid) else {
            return None;
        };
        if (entity_paused || map_paused) && !body.ignore_paused {
            return None;
        }
        if body.body_type == BodyType::Static || !body.awake {
            return None;
        }

        let previous_linear_velocity = body.linear_velocity;
        let previous_angular_velocity = body.angular_velocity;
        let previous_force = body.force;
        let previous_torque = body.torque;
        let effective_gravity = if body.ignore_gravity {
            Vector2::ZERO
        } else {
            gravity
        };
        body.linear_velocity = body.linear_velocity + (effective_gravity + body.force) * frame_time;
        if body.linear_damping > 0.0 {
            let damping = (1.0 - body.linear_damping * frame_time).max(0.0);
            body.linear_velocity = body.linear_velocity * damping;
        }
        if body.fixed_rotation {
            body.angular_velocity = 0.0;
        } else {
            body.angular_velocity += body.torque * frame_time;
            if body.angular_damping > 0.0 {
                let damping = (1.0 - body.angular_damping * frame_time).max(0.0);
                body.angular_velocity *= damping;
            }
        }

        let previous_awake = body.awake;
        if body.sleeping_allowed {
            let idle_linear =
                body.linear_velocity.length_squared() <= Self::SLEEP_LINEAR_TOLERANCE_SQ;
            let idle_angular = body.angular_velocity.abs() <= Self::SLEEP_ANGULAR_TOLERANCE;
            if idle_linear && idle_angular {
                body.sleep_time += frame_time;
                if body.sleep_time >= Self::SLEEP_TIME_THRESHOLD {
                    body.set_awake(false);
                }
            } else {
                body.sleep_time = 0.0;
            }
        } else {
            body.sleep_time = 0.0;
        }

        let state = PhysicsStepState {
            linear_velocity: body.linear_velocity,
            angular_velocity: body.angular_velocity,
            auto_clear_forces,
            awake_changed: previous_awake != body.awake,
            awake: body.awake,
            state_changed: previous_awake != body.awake
                || previous_linear_velocity != body.linear_velocity
                || previous_angular_velocity != body.angular_velocity
                || (auto_clear_forces
                    && (previous_force != Vector2::ZERO || previous_torque != 0.0)),
        };

        if auto_clear_forces {
            body.force = Vector2::ZERO;
            body.torque = 0.0;
        }

        Some(state)
    }

    pub(crate) fn get_world_aabb(&self, manager: &EntityManager, uid: EntityUid) -> Option<Box2> {
        let body = manager.physics.get(&uid)?;
        let xform = manager.transforms.get(&uid)?;
        let fixtures = manager.fixtures.get(&uid)?;
        body.get_aabb(xform, fixtures, manager)
    }

    pub(crate) fn get_map_velocities(
        &self,
        manager: &EntityManager,
        uid: EntityUid,
    ) -> (Vector2, f32) {
        let Some(component) = manager.physics.get(&uid) else {
            return (Vector2::ZERO, 0.0);
        };
        let Some(xform) = manager.transforms.get(&uid) else {
            return (Vector2::ZERO, 0.0);
        };

        let mut parent = xform.parent;
        let mut local_pos = xform.local_position;
        let mut linear_velocity = component.linear_velocity;
        let mut angular_velocity = component.angular_velocity;
        let mut angular_linear_contribution = Vector2::ZERO;

        while parent.is_valid() {
            let Some(parent_xform) = manager.transforms.get(&parent) else {
                break;
            };

            if let Some(body) = manager.physics.get(&parent) {
                angular_velocity += body.angular_velocity;
                linear_velocity = linear_velocity + body.linear_velocity;
                angular_linear_contribution = angular_linear_contribution
                    + Vector2::new(
                        -body.angular_velocity * local_pos.y,
                        body.angular_velocity * local_pos.x,
                    );
                angular_linear_contribution = parent_xform
                    .local_rotation
                    .rotate_vec(angular_linear_contribution);
            }

            local_pos =
                parent_xform.local_position + parent_xform.local_rotation.rotate_vec(local_pos);
            parent = parent_xform.parent;
        }

        (
            linear_velocity + angular_linear_contribution,
            angular_velocity,
        )
    }

    pub(crate) fn sync_broadphase(
        &self,
        manager: &mut EntityManager,
        broadphase_owner: EntityUid,
        bodies: &[EntityUid],
    ) -> usize {
        let mut pending = Vec::new();
        let body_set = bodies.iter().copied().collect::<HashSet<_>>();

        for (uid, body, xform, fixtures) in manager.entity_query3(
            &manager.physics,
            &manager.transforms,
            &manager.fixtures,
            true,
        ) {
            if !body_set.contains(&uid) || !body.can_collide {
                continue;
            }
            let (world_pos, world_rot, _) = xform.get_world_position_rotation_matrix(manager);
            let transform = butsuri::Transform::from_angle_type(world_pos, world_rot);
            pending.extend(
                fixtures
                    .fixtures
                    .values()
                    .cloned()
                    .map(|fixture| (uid.raw(), fixture, transform)),
            );
        }

        let Some(broadphase) = manager.broadphases.get_mut(&broadphase_owner) else {
            return 0;
        };

        broadphase.tree = Default::default();
        for (owner_id, fixture, transform) in &pending {
            broadphase
                .tree
                .insert(*owner_id, fixture.clone(), *transform);
        }

        pending.len()
    }

    pub(crate) fn sync_contacts(
        &self,
        manager: &mut EntityManager,
        map_owner: EntityUid,
        bodies: &[EntityUid],
    ) -> usize {
        let mut contact_manager = ContactManager::new();
        let previous_contact_manager = ContactManager::from_contacts(
            manager
                .physics_maps
                .get(&map_owner)
                .map(|map| map.contacts().to_vec())
                .unwrap_or_default(),
        );
        let previous_contacts = previous_contact_manager.contacts();
        let mut body_fixtures = Vec::new();
        let body_set = bodies.iter().copied().collect::<HashSet<_>>();

        for (uid, body, transform, fixtures) in manager.entity_query3(
            &manager.physics,
            &manager.transforms,
            &manager.fixtures,
            true,
        ) {
            if !body_set.contains(&uid) || !body.can_collide {
                continue;
            }
            let (world_pos, world_rot, _) = transform.get_world_position_rotation_matrix(manager);
            let physics_transform = butsuri::Transform::from_angle_type(world_pos, world_rot);
            let fixture_data = fixtures
                .fixtures
                .values()
                .cloned()
                .map(|fixture| (fixture, physics_transform))
                .collect::<Vec<_>>();
            if fixture_data.is_empty() {
                continue;
            }
            body_fixtures.push((uid, body.body_type, fixture_data));
        }

        for first in 0..body_fixtures.len() {
            let (uid_a, body_type_a, fixtures_a) = &body_fixtures[first];
            for second in first + 1..body_fixtures.len() {
                let (uid_b, body_type_b, fixtures_b) = &body_fixtures[second];
                if *body_type_a == BodyType::Static && *body_type_b == BodyType::Static {
                    continue;
                }
                if self.joint_blocks_collision(manager, *uid_a, *uid_b, *body_type_a, *body_type_b)
                {
                    continue;
                }

                for (fixture_a, transform_a) in fixtures_a {
                    for (fixture_b, transform_b) in fixtures_b {
                        if !Self::fixtures_can_collide(fixture_a, fixture_b) {
                            continue;
                        }
                        if !fixture_a
                            .compute_aabb(*transform_a)
                            .intersects(fixture_b.compute_aabb(*transform_b))
                        {
                            continue;
                        }

                        let fixture_a_key = format!("{}:{}", uid_a.raw(), fixture_a.id);
                        let fixture_b_key = format!("{}:{}", uid_b.raw(), fixture_b.id);
                        let (
                            fixture_a_key,
                            ordered_fixture_a,
                            ordered_transform_a,
                            fixture_b_key,
                            ordered_fixture_b,
                            ordered_transform_b,
                        ) = if fixture_a_key <= fixture_b_key {
                            (
                                fixture_a_key,
                                fixture_a,
                                *transform_a,
                                fixture_b_key,
                                fixture_b,
                                *transform_b,
                            )
                        } else {
                            (
                                fixture_b_key,
                                fixture_b,
                                *transform_b,
                                fixture_a_key,
                                fixture_a,
                                *transform_a,
                            )
                        };

                        let manifold = Self::compute_contact_manifold(
                            ordered_fixture_a,
                            ordered_transform_a,
                            ordered_fixture_b,
                            ordered_transform_b,
                        );
                        if manifold.points.is_empty() {
                            continue;
                        }
                        if let Some(previous) =
                            previous_contact_manager.contact_pair(&fixture_a_key, &fixture_b_key)
                        {
                            let mut contact = previous.clone();
                            contact.retarget_pair(
                                fixture_a_key,
                                ordered_fixture_a,
                                fixture_b_key,
                                ordered_fixture_b,
                            );
                            let (_, status, contact) = contact_manager.insert_refreshed_contact(
                                contact,
                                Self::merge_manifold_impulses(&previous.manifold, manifold),
                            );
                            if let Some(physics_map) = manager.physics_maps.get_mut(&map_owner) {
                                physics_map.queue_contact_event(status, contact);
                            }
                        } else {
                            let Some((_, status, contact)) = contact_manager
                                .add_pair_with_manifold(
                                    &fixture_a_key,
                                    ordered_fixture_a,
                                    &fixture_b_key,
                                    ordered_fixture_b,
                                    manifold,
                                )
                            else {
                                continue;
                            };
                            if let Some(physics_map) = manager.physics_maps.get_mut(&map_owner) {
                                physics_map.queue_contact_event(status, contact);
                            }
                        };
                    }
                }
            }
        }

        let count = contact_manager.contact_count();
        let Some(physics_map) = manager.physics_maps.get_mut(&map_owner) else {
            return 0;
        };
        for previous in previous_contacts {
            let still_present =
                contact_manager.has_contact_pair(&previous.fixture_a, &previous.fixture_b);
            if still_present {
                continue;
            }
            let mut ended = previous.clone();
            let status = ended.end_touching();
            physics_map.queue_contact_event(status, ended);
        }
        physics_map.replace_contacts(contact_manager);
        count
    }

    pub(crate) fn query_aabb_entities(
        &self,
        manager: &EntityManager,
        broadphase_owner: EntityUid,
        aabb: Box2,
    ) -> Vec<EntityUid> {
        let Some(broadphase) = manager.broadphases.get(&broadphase_owner) else {
            return Vec::new();
        };

        let mut entities = BTreeSet::new();
        for hit in broadphase.tree.query_aabb(aabb) {
            entities.insert(EntityUid::new(hit.owner_id));
        }
        entities.into_iter().collect()
    }

    pub(crate) fn intersect_ray(
        &self,
        manager: &EntityManager,
        broadphase_owner: EntityUid,
        ray: CollisionRay,
        max_length: f32,
        return_on_first_hit: bool,
    ) -> Vec<PhysicsQueryHit> {
        let Some(broadphase) = manager.broadphases.get(&broadphase_owner) else {
            return Vec::new();
        };

        broadphase
            .tree
            .query_ray(ray, max_length, return_on_first_hit)
            .into_iter()
            .filter_map(
                |RayCastHit {
                     distance,
                     hit_pos,
                     hit,
                 }| {
                    let entry = broadphase.tree.entry(hit.fixture_index)?;
                    Some(PhysicsQueryHit {
                        entity: EntityUid::new(hit.owner_id),
                        fixture_id: entry.fixture.id.clone(),
                        distance,
                        hit_pos,
                    })
                },
            )
            .collect()
    }

    fn fixtures_can_collide(fixture_a: &Fixture, fixture_b: &Fixture) -> bool {
        let unrestricted = fixture_a.collision_layer == 0
            && fixture_a.collision_mask == 0
            && fixture_b.collision_layer == 0
            && fixture_b.collision_mask == 0;
        unrestricted
            || ((fixture_a.collision_mask & fixture_b.collision_layer) != 0
                && (fixture_b.collision_mask & fixture_a.collision_layer) != 0)
    }

    fn merge_manifold_impulses(
        previous: &ContactManifold,
        mut current: ContactManifold,
    ) -> ContactManifold {
        if previous.points.len() == current.points.len() {
            for (previous_point, current_point) in
                previous.points.iter().zip(current.points.iter_mut())
            {
                current_point.normal_impulse = previous_point.normal_impulse;
                current_point.tangent_impulse = previous_point.tangent_impulse;
            }
        }
        current
    }

    fn compute_contact_manifold(
        fixture_a: &Fixture,
        transform_a: PhysicsTransform,
        fixture_b: &Fixture,
        transform_b: PhysicsTransform,
    ) -> ContactManifold {
        match (fixture_a.shape, fixture_b.shape) {
            (PhysShape::Aabb(shape_a), PhysShape::Aabb(shape_b)) => {
                Self::compute_aabb_aabb_manifold(shape_a, transform_a, shape_b, transform_b)
            }
            (PhysShape::Circle(shape_a), PhysShape::Circle(shape_b)) => {
                Self::compute_circle_circle_manifold(shape_a, transform_a, shape_b, transform_b)
            }
            (PhysShape::Aabb(shape_a), PhysShape::Circle(shape_b)) => {
                Self::compute_aabb_circle_manifold(shape_a, transform_a, shape_b, transform_b)
            }
            (PhysShape::Circle(shape_a), PhysShape::Aabb(shape_b)) => {
                Self::compute_circle_aabb_manifold(shape_a, transform_a, shape_b, transform_b)
            }
        }
    }

    fn compute_aabb_aabb_manifold(
        shape_a: AabbShape,
        transform_a: PhysicsTransform,
        shape_b: AabbShape,
        transform_b: PhysicsTransform,
    ) -> ContactManifold {
        let corners_a = Self::aabb_world_corners(shape_a, transform_a);
        let corners_b = Self::aabb_world_corners(shape_b, transform_b);
        let axes = [
            transform_a.rotation.mul(Vector2::UNIT_X),
            transform_a.rotation.mul(Vector2::UNIT_Y),
            transform_b.rotation.mul(Vector2::UNIT_X),
            transform_b.rotation.mul(Vector2::UNIT_Y),
        ];
        let center_a = transform_a.mul(shape_a.local_bounds.center());
        let center_b = transform_b.mul(shape_b.local_bounds.center());

        let mut best_overlap = f32::MAX;
        let mut best_normal = Vector2::ZERO;

        for axis in axes {
            let axis_length = axis.length();
            if axis_length <= 0.0001 {
                continue;
            }
            let axis = axis / axis_length;
            let (min_a, max_a) = Self::project_points(axis, &corners_a);
            let (min_b, max_b) = Self::project_points(axis, &corners_b);
            let overlap = (max_a + shape_a.radius).min(max_b + shape_b.radius)
                - (min_a - shape_a.radius).max(min_b - shape_b.radius);
            if overlap <= 0.0 {
                return ContactManifold::default();
            }
            if overlap < best_overlap {
                best_overlap = overlap;
                best_normal = if Vector2::dot(center_b - center_a, axis) >= 0.0 {
                    axis
                } else {
                    -axis
                };
            }
        }

        if best_normal == Vector2::ZERO {
            return ContactManifold::default();
        }

        let point_a = Self::support_point(&corners_a, best_normal) + best_normal * shape_a.radius;
        let point_b = Self::support_point(&corners_b, -best_normal) - best_normal * shape_b.radius;
        let world_point = (point_a + point_b) * 0.5;
        ContactManifold {
            normal: best_normal,
            points: vec![ContactManifoldPoint {
                local_point: transform_a.mul_t(world_point),
                normal_impulse: 0.0,
                tangent_impulse: 0.0,
            }],
        }
    }

    fn compute_circle_circle_manifold(
        shape_a: CircleShape,
        transform_a: PhysicsTransform,
        shape_b: CircleShape,
        transform_b: PhysicsTransform,
    ) -> ContactManifold {
        let center_a = transform_a.mul(shape_a.position);
        let center_b = transform_b.mul(shape_b.position);
        let delta = center_b - center_a;
        let distance = delta.length();
        let radius_sum = shape_a.radius + shape_b.radius;
        if distance > radius_sum {
            return ContactManifold::default();
        }

        let normal = if distance <= 0.0001 {
            Vector2::UNIT_X
        } else {
            delta / distance
        };
        let world_point = center_a + normal * shape_a.radius.min(radius_sum * 0.5);
        ContactManifold {
            normal,
            points: vec![ContactManifoldPoint {
                local_point: transform_a.mul_t(world_point),
                normal_impulse: 0.0,
                tangent_impulse: 0.0,
            }],
        }
    }

    fn compute_aabb_circle_manifold(
        shape_aabb: AabbShape,
        transform_aabb: PhysicsTransform,
        shape_circle: CircleShape,
        transform_circle: PhysicsTransform,
    ) -> ContactManifold {
        let center = transform_circle.mul(shape_circle.position);
        let local_center = transform_aabb.mul_t(center);
        let bounds = shape_aabb.local_bounds.enlarged(shape_aabb.radius);
        let local_closest = bounds.closest_point(local_center);
        let delta = local_center - local_closest;
        let distance = delta.length();

        let (normal, world_point) = if distance > 0.0001 {
            if distance > shape_circle.radius {
                return ContactManifold::default();
            }
            let local_normal = delta / distance;
            (
                transform_aabb.rotation.mul(local_normal),
                transform_aabb.mul(local_closest),
            )
        } else {
            let left = (local_center.x - bounds.left).abs();
            let right = (bounds.right - local_center.x).abs();
            let bottom = (local_center.y - bounds.bottom).abs();
            let top = (bounds.top - local_center.y).abs();

            let (local_normal, local_point) = if left <= right && left <= bottom && left <= top {
                (
                    Vector2::new(-1.0, 0.0),
                    Vector2::new(bounds.left, local_center.y),
                )
            } else if right <= bottom && right <= top {
                (
                    Vector2::new(1.0, 0.0),
                    Vector2::new(bounds.right, local_center.y),
                )
            } else if bottom <= top {
                (
                    Vector2::new(0.0, -1.0),
                    Vector2::new(local_center.x, bounds.bottom),
                )
            } else {
                (
                    Vector2::new(0.0, 1.0),
                    Vector2::new(local_center.x, bounds.top),
                )
            };
            (
                transform_aabb.rotation.mul(local_normal),
                transform_aabb.mul(local_point),
            )
        };

        ContactManifold {
            normal,
            points: vec![ContactManifoldPoint {
                local_point: transform_aabb.mul_t(world_point),
                normal_impulse: 0.0,
                tangent_impulse: 0.0,
            }],
        }
    }

    fn compute_circle_aabb_manifold(
        shape_circle: CircleShape,
        transform_circle: PhysicsTransform,
        shape_aabb: AabbShape,
        transform_aabb: PhysicsTransform,
    ) -> ContactManifold {
        let manifold = Self::compute_aabb_circle_manifold(
            shape_aabb,
            transform_aabb,
            shape_circle,
            transform_circle,
        );
        if manifold.points.is_empty() {
            return manifold;
        }

        let center = transform_circle.mul(shape_circle.position);
        let world_point =
            center + (-manifold.normal) * shape_circle.radius.min(shape_circle.radius);
        ContactManifold {
            normal: -manifold.normal,
            points: vec![ContactManifoldPoint {
                local_point: transform_circle.mul_t(world_point),
                normal_impulse: 0.0,
                tangent_impulse: 0.0,
            }],
        }
    }

    fn aabb_world_corners(shape: AabbShape, transform: PhysicsTransform) -> [Vector2; 4] {
        [
            transform.mul(shape.local_bounds.bottom_left()),
            transform.mul(shape.local_bounds.bottom_right()),
            transform.mul(shape.local_bounds.top_right()),
            transform.mul(shape.local_bounds.top_left()),
        ]
    }

    fn project_points(axis: Vector2, points: &[Vector2; 4]) -> (f32, f32) {
        let mut min = Vector2::dot(points[0], axis);
        let mut max = min;
        for point in points.iter().copied().skip(1) {
            let projection = Vector2::dot(point, axis);
            min = min.min(projection);
            max = max.max(projection);
        }
        (min, max)
    }

    fn support_point(points: &[Vector2; 4], direction: Vector2) -> Vector2 {
        let mut best = points[0];
        let mut best_projection = Vector2::dot(best, direction);
        for point in points.iter().copied().skip(1) {
            let projection = Vector2::dot(point, direction);
            if projection > best_projection {
                best = point;
                best_projection = projection;
            }
        }
        best
    }

    fn joint_blocks_collision(
        &self,
        manager: &EntityManager,
        uid_a: EntityUid,
        uid_b: EntityUid,
        body_type_a: BodyType,
        body_type_b: BodyType,
    ) -> bool {
        manager
            .joint_components
            .get(&uid_a)
            .into_iter()
            .flat_map(|component| component.joints.values())
            .chain(
                manager
                    .joint_components
                    .get(&uid_b)
                    .into_iter()
                    .flat_map(|component| component.joints.values()),
            )
            .any(|joint| {
                let matches_pair = (joint.body_a_uid == uid_a.raw()
                    && joint.body_b_uid == uid_b.raw())
                    || (joint.body_a_uid == uid_b.raw() && joint.body_b_uid == uid_a.raw());
                matches_pair && joint.blocks_collisions(body_type_a, body_type_b)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{PhysicsQueryHit, SharedPhysicsSystem};
    use crate::{BroadphaseComponent, EntityManager};
    use butsuri::{
        AabbShape, BodyType, CircleShape, CollisionRay, Fixture, Joint, JointType, PhysShape,
    };
    use keisan::{Box2, Vector2};

    fn configure_dynamic_body(manager: &mut EntityManager, uid: crate::EntityUid) {
        assert!(manager.configure_physics_body(
            uid,
            Some(BodyType::Dynamic),
            None,
            Some(true),
            None,
        ));
    }

    fn set_position(manager: &mut EntityManager, uid: crate::EntityUid, position: Vector2) {
        assert!(manager.mutate_transform_and_reconcile(uid, |transform| {
            transform.local_position = position;
        }));
    }

    fn set_rotation(manager: &mut EntityManager, uid: crate::EntityUid, rotation: keisan::Angle) {
        assert!(manager.mutate_transform_and_reconcile(uid, |transform| {
            transform.local_rotation = rotation;
        }));
    }

    fn mutate_body<F>(manager: &mut EntityManager, uid: crate::EntityUid, mutate: F)
    where
        F: FnOnce(&mut crate::PhysicsComponent),
    {
        assert!(manager.mutate_physics_and_reconcile(uid, mutate));
    }

    #[test]
    fn physics_system_computes_aabb_and_syncs_broadphase() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.initialize_entity(uid);

        configure_dynamic_body(&mut manager, uid);

        manager.ensure_fixtures(uid).insert_fixture(Fixture::new(
            "main",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        ));

        let broadphase_uid = manager.create_entity_uninitialized(None);
        manager
            .broadphases
            .insert(broadphase_uid, BroadphaseComponent::new());

        let system = SharedPhysicsSystem::new();
        let aabb = system.get_world_aabb(&manager, uid).unwrap();
        assert_eq!(aabb, Box2::new(-1.0, -1.0, 1.0, 1.0));
        assert_eq!(
            system.sync_broadphase(&mut manager, broadphase_uid, &[uid]),
            1
        );
        assert_eq!(
            system.query_aabb_entities(&manager, broadphase_uid, Box2::new(-2.0, -2.0, 2.0, 2.0)),
            vec![uid]
        );
        assert_eq!(
            system.intersect_ray(
                &manager,
                broadphase_uid,
                CollisionRay::new(Vector2::new(-5.0, 0.0), Vector2::UNIT_X, -1),
                10.0,
                true,
            ),
            vec![PhysicsQueryHit {
                entity: uid,
                fixture_id: "main".to_string(),
                distance: 4.0,
                hit_pos: Vector2::new(-1.0, 0.0),
            }]
        );
    }

    #[test]
    fn physics_system_intersect_ray_respects_fixture_collision_layers() {
        let mut manager = EntityManager::new();
        let first = manager.create_entity_uninitialized(None);
        manager.initialize_entity(first);
        assert!(manager.configure_physics_body(
            first,
            Some(BodyType::Dynamic),
            None,
            Some(true),
            None,
        ));
        let mut first_fixture = Fixture::new(
            "first",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        );
        first_fixture.collision_layer = 1 << 0;
        let _ = manager.insert_fixture_and_reconcile(first, first_fixture);
        assert!(manager.mutate_transform_and_reconcile(first, |transform| {
            transform.local_position = Vector2::new(5.0, 0.0);
        }));

        let second = manager.create_entity_uninitialized(None);
        manager.initialize_entity(second);
        assert!(manager.configure_physics_body(
            second,
            Some(BodyType::Dynamic),
            None,
            Some(true),
            None,
        ));
        let mut second_fixture = Fixture::new(
            "second",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        );
        second_fixture.collision_layer = 1 << 1;
        let _ = manager.insert_fixture_and_reconcile(second, second_fixture);
        assert!(manager.mutate_transform_and_reconcile(second, |transform| {
            transform.local_position = Vector2::new(8.0, 0.0);
        }));

        let broadphase_uid = manager.create_entity_uninitialized(None);
        manager
            .broadphases
            .insert(broadphase_uid, BroadphaseComponent::new());

        let system = SharedPhysicsSystem::new();
        assert_eq!(
            system.sync_broadphase(&mut manager, broadphase_uid, &[first, second]),
            2
        );
        let hits = system.intersect_ray(
            &manager,
            broadphase_uid,
            CollisionRay::new(Vector2::ZERO, Vector2::UNIT_X, 1 << 1),
            10.0,
            false,
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entity, second);
        assert_eq!(hits[0].fixture_id, "second");
    }

    #[test]
    fn physics_system_intersect_ray_first_hit_returns_closest_fixture() {
        let mut manager = EntityManager::new();
        let far = manager.create_entity_uninitialized(None);
        manager.initialize_entity(far);
        assert!(manager.configure_physics_body(
            far,
            Some(BodyType::Dynamic),
            None,
            Some(true),
            None,
        ));
        let _ = manager.insert_fixture_and_reconcile(
            far,
            Fixture::new(
                "far",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );
        assert!(manager.mutate_transform_and_reconcile(far, |transform| {
            transform.local_position = Vector2::new(8.0, 0.0);
        }));

        let near = manager.create_entity_uninitialized(None);
        manager.initialize_entity(near);
        assert!(manager.configure_physics_body(
            near,
            Some(BodyType::Dynamic),
            None,
            Some(true),
            None,
        ));
        let _ = manager.insert_fixture_and_reconcile(
            near,
            Fixture::new(
                "near",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );
        assert!(manager.mutate_transform_and_reconcile(near, |transform| {
            transform.local_position = Vector2::new(5.0, 0.0);
        }));

        let broadphase_uid = manager.create_entity_uninitialized(None);
        manager
            .broadphases
            .insert(broadphase_uid, BroadphaseComponent::new());

        let system = SharedPhysicsSystem::new();
        assert_eq!(
            system.sync_broadphase(&mut manager, broadphase_uid, &[far, near]),
            2
        );
        let hits = system.intersect_ray(
            &manager,
            broadphase_uid,
            CollisionRay::new(Vector2::ZERO, Vector2::UNIT_X, -1),
            10.0,
            true,
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entity, near);
        assert_eq!(hits[0].fixture_id, "near");
    }

    #[test]
    fn physics_system_intersect_ray_ignores_circle_aabb_false_positives() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.initialize_entity(uid);
        assert!(manager.configure_physics_body(
            uid,
            Some(BodyType::Dynamic),
            None,
            Some(true),
            None,
        ));
        let _ = manager.insert_fixture_and_reconcile(
            uid,
            Fixture::new(
                "circle",
                PhysShape::Circle(CircleShape::new(Vector2::ZERO, 1.0)),
            ),
        );

        let broadphase_uid = manager.create_entity_uninitialized(None);
        manager
            .broadphases
            .insert(broadphase_uid, BroadphaseComponent::new());

        let system = SharedPhysicsSystem::new();
        assert_eq!(
            system.sync_broadphase(&mut manager, broadphase_uid, &[uid]),
            1
        );
        let hits = system.intersect_ray(
            &manager,
            broadphase_uid,
            CollisionRay::new(Vector2::new(-2.0, 1.1), Vector2::UNIT_X, -1),
            10.0,
            false,
        );
        assert!(hits.is_empty());
    }

    #[test]
    fn physics_system_syncs_contacts_per_map_and_respects_joint_collision_filters() {
        let mut manager = EntityManager::new();
        let map_owner = manager.create_entity_uninitialized(None);
        manager.initialize_entity(map_owner);
        manager.ensure_physics_map(map_owner);

        let first = manager.create_entity_uninitialized(None);
        manager.initialize_entity(first);
        assert!(manager.configure_physics_body(
            first,
            Some(BodyType::Dynamic),
            None,
            Some(true),
            None,
        ));
        let _ = manager.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );

        let second = manager.create_entity_uninitialized(None);
        manager.initialize_entity(second);
        assert!(manager.configure_physics_body(
            second,
            Some(BodyType::Dynamic),
            None,
            Some(true),
            None,
        ));
        let _ = manager.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let system = SharedPhysicsSystem::new();
        assert_eq!(
            system.sync_contacts(&mut manager, map_owner, &[first, second]),
            1
        );
        let contacts = manager.owner_contacts_snapshot(map_owner);
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].fixture_a, format!("{}:main", first.raw()));
        assert_eq!(contacts[0].fixture_b, format!("{}:main", second.raw()));
        assert!(contacts[0].is_touching);

        let mut joint = Joint::new(first.raw(), second.raw(), JointType::Distance);
        joint.id = "rope".to_string();
        joint.collide_connected = false;
        assert!(manager.add_joint_between(joint));
        assert_eq!(
            system.sync_contacts(&mut manager, map_owner, &[first, second]),
            0
        );
        assert_eq!(manager.owner_contact_count(map_owner), 0);
    }

    #[test]
    fn physics_system_syncs_aabb_contact_manifolds() {
        let mut manager = EntityManager::new();
        let map_owner = manager.create_entity_uninitialized(None);
        manager.initialize_entity(map_owner);
        manager.ensure_physics_map(map_owner);

        let first = manager.create_entity_uninitialized(None);
        manager.initialize_entity(first);
        assert!(manager.configure_physics_body(
            first,
            Some(BodyType::Dynamic),
            None,
            Some(true),
            None,
        ));
        let _ = manager.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "first",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );

        let second = manager.create_entity_uninitialized(None);
        manager.initialize_entity(second);
        assert!(manager.configure_physics_body(
            second,
            Some(BodyType::Dynamic),
            None,
            Some(true),
            None,
        ));
        let _ = manager.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "second",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );
        assert!(manager.mutate_transform_and_reconcile(second, |transform| {
            transform.local_position = Vector2::new(1.5, 0.0);
        }));

        let system = SharedPhysicsSystem::new();
        assert_eq!(
            system.sync_contacts(&mut manager, map_owner, &[first, second]),
            1
        );
        let contacts = manager.owner_contacts_snapshot(map_owner);
        let contact = &contacts[0];
        assert_eq!(contact.contact_type, butsuri::ContactType::Aabb);
        assert_eq!(contact.manifold.normal, Vector2::UNIT_X);
        assert_eq!(contact.manifold.points.len(), 1);
        assert!(contact.manifold.points[0].local_point.x >= 0.0);
    }

    #[test]
    fn physics_system_syncs_circle_contact_manifolds() {
        let mut manager = EntityManager::new();
        let map_owner = manager.create_entity_uninitialized(None);
        manager.initialize_entity(map_owner);
        manager.ensure_physics_map(map_owner);

        let first = manager.create_entity_uninitialized(None);
        manager.initialize_entity(first);
        configure_dynamic_body(&mut manager, first);
        let _ = manager.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "first",
                PhysShape::Circle(CircleShape::new(Vector2::ZERO, 1.0)),
            ),
        );

        let second = manager.create_entity_uninitialized(None);
        manager.initialize_entity(second);
        configure_dynamic_body(&mut manager, second);
        let _ = manager.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "second",
                PhysShape::Circle(CircleShape::new(Vector2::ZERO, 1.0)),
            ),
        );
        set_position(&mut manager, second, Vector2::new(1.5, 0.0));

        let system = SharedPhysicsSystem::new();
        assert_eq!(
            system.sync_contacts(&mut manager, map_owner, &[first, second]),
            1
        );
        let contacts = manager.owner_contacts_snapshot(map_owner);
        let contact = &contacts[0];
        assert_eq!(contact.contact_type, butsuri::ContactType::Circle);
        assert_eq!(contact.manifold.normal, Vector2::UNIT_X);
        assert_eq!(contact.manifold.points.len(), 1);
        assert!(contact.manifold.points[0].local_point.x > 0.0);
    }

    #[test]
    fn physics_system_syncs_mixed_contact_manifolds() {
        let mut manager = EntityManager::new();
        let map_owner = manager.create_entity_uninitialized(None);
        manager.initialize_entity(map_owner);
        manager.ensure_physics_map(map_owner);

        let first = manager.create_entity_uninitialized(None);
        manager.initialize_entity(first);
        configure_dynamic_body(&mut manager, first);
        let _ = manager.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "aabb",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );

        let second = manager.create_entity_uninitialized(None);
        manager.initialize_entity(second);
        configure_dynamic_body(&mut manager, second);
        let _ = manager.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "circle",
                PhysShape::Circle(CircleShape::new(Vector2::ZERO, 1.0)),
            ),
        );
        set_position(&mut manager, second, Vector2::new(1.5, 0.0));

        let system = SharedPhysicsSystem::new();
        assert_eq!(
            system.sync_contacts(&mut manager, map_owner, &[first, second]),
            1
        );
        let contacts = manager.owner_contacts_snapshot(map_owner);
        let contact = &contacts[0];
        assert_eq!(contact.contact_type, butsuri::ContactType::Mixed);
        assert_eq!(contact.manifold.points.len(), 1);
        assert!(contact.manifold.normal.x.abs() > 0.9);
    }

    #[test]
    fn physics_system_sync_contacts_ignores_circle_aabb_false_positives() {
        let mut manager = EntityManager::new();
        let map_owner = manager.create_entity_uninitialized(None);
        manager.initialize_entity(map_owner);
        manager.ensure_physics_map(map_owner);

        let first = manager.create_entity_uninitialized(None);
        manager.initialize_entity(first);
        configure_dynamic_body(&mut manager, first);
        let _ = manager.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "box",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );

        let second = manager.create_entity_uninitialized(None);
        manager.initialize_entity(second);
        configure_dynamic_body(&mut manager, second);
        let _ = manager.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "circle",
                PhysShape::Circle(CircleShape::new(Vector2::ZERO, 0.5)),
            ),
        );
        set_position(&mut manager, second, Vector2::new(1.4, 1.4));

        let system = SharedPhysicsSystem::new();
        assert_eq!(
            system.sync_contacts(&mut manager, map_owner, &[first, second]),
            0
        );
        assert_eq!(manager.owner_contact_count(map_owner), 0);
    }

    #[test]
    fn physics_system_syncs_rotated_aabb_circle_contact_manifolds_exactly() {
        let mut manager = EntityManager::new();
        let map_owner = manager.create_entity_uninitialized(None);
        manager.initialize_entity(map_owner);
        manager.ensure_physics_map(map_owner);

        let first = manager.create_entity_uninitialized(None);
        manager.initialize_entity(first);
        configure_dynamic_body(&mut manager, first);
        let _ = manager.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "box",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );
        set_rotation(&mut manager, first, keisan::Angle::from_degrees(45.0));

        let second = manager.create_entity_uninitialized(None);
        manager.initialize_entity(second);
        configure_dynamic_body(&mut manager, second);
        let _ = manager.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "circle",
                PhysShape::Circle(CircleShape::new(Vector2::ZERO, 0.5)),
            ),
        );
        set_position(&mut manager, second, Vector2::new(1.2, 0.0));

        let system = SharedPhysicsSystem::new();
        assert_eq!(
            system.sync_contacts(&mut manager, map_owner, &[first, second]),
            1
        );
        let contacts = manager.owner_contacts_snapshot(map_owner);
        let contact = &contacts[0];
        assert_eq!(contact.contact_type, butsuri::ContactType::Mixed);
        assert_eq!(contact.manifold.points.len(), 1);
        assert!(contact.manifold.normal.x.abs() > 0.5);
    }

    #[test]
    fn physics_system_sync_contacts_ignores_rotated_aabb_false_positives() {
        let mut manager = EntityManager::new();
        let map_owner = manager.create_entity_uninitialized(None);
        manager.initialize_entity(map_owner);
        manager.ensure_physics_map(map_owner);

        let first = manager.create_entity_uninitialized(None);
        manager.initialize_entity(first);
        configure_dynamic_body(&mut manager, first);
        let _ = manager.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "first",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -0.25, 1.0, 0.25), 0.0)),
            ),
        );
        set_rotation(&mut manager, first, keisan::Angle::from_degrees(45.0));

        let second = manager.create_entity_uninitialized(None);
        manager.initialize_entity(second);
        configure_dynamic_body(&mut manager, second);
        let _ = manager.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "second",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -0.25, 1.0, 0.25), 0.0)),
            ),
        );
        assert!(manager.mutate_transform_and_reconcile(second, |transform| {
            transform.local_position = Vector2::new(-1.7, -1.7);
            transform.local_rotation = keisan::Angle::from_degrees(45.0);
        }));

        let system = SharedPhysicsSystem::new();
        assert_eq!(
            system.sync_contacts(&mut manager, map_owner, &[first, second]),
            0
        );
        assert_eq!(manager.owner_contact_count(map_owner), 0);
    }

    #[test]
    fn physics_system_preserves_contact_touching_state_and_impulses_across_syncs() {
        let mut manager = EntityManager::new();
        let map_owner = manager.create_entity_uninitialized(None);
        manager.initialize_entity(map_owner);
        manager.ensure_physics_map(map_owner);

        let first = manager.create_entity_uninitialized(None);
        manager.initialize_entity(first);
        configure_dynamic_body(&mut manager, first);
        let _ = manager.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "first",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );

        let second = manager.create_entity_uninitialized(None);
        manager.initialize_entity(second);
        configure_dynamic_body(&mut manager, second);
        let _ = manager.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "second",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );
        set_position(&mut manager, second, Vector2::new(1.5, 0.0));

        let system = SharedPhysicsSystem::new();
        assert_eq!(
            system.sync_contacts(&mut manager, map_owner, &[first, second]),
            1
        );
        {
            let physics_map = manager.physics_maps.get_mut(&map_owner).unwrap();
            assert!(physics_map.set_contact_point_impulse(0, 0, 3.5, 1.25));
        }

        assert_eq!(
            system.sync_contacts(&mut manager, map_owner, &[first, second]),
            1
        );
        let contacts = manager.owner_contacts_snapshot(map_owner);
        let contact = &contacts[0];
        assert!(contact.is_touching);
        assert_eq!(contact.manifold.points[0].normal_impulse, 3.5);
        assert_eq!(contact.manifold.points[0].tangent_impulse, 1.25);
    }

    #[test]
    fn physics_system_queues_start_and_end_touching_events() {
        let mut manager = EntityManager::new();
        let map_owner = manager.create_entity_uninitialized(None);
        manager.initialize_entity(map_owner);
        manager.ensure_physics_map(map_owner);

        let first = manager.create_entity_uninitialized(None);
        manager.initialize_entity(first);
        configure_dynamic_body(&mut manager, first);
        let _ = manager.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "first",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );

        let second = manager.create_entity_uninitialized(None);
        manager.initialize_entity(second);
        configure_dynamic_body(&mut manager, second);
        let _ = manager.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "second",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );
        set_position(&mut manager, second, Vector2::new(1.5, 0.0));

        let system = SharedPhysicsSystem::new();
        assert_eq!(
            system.sync_contacts(&mut manager, map_owner, &[first, second]),
            1
        );
        let events = manager.drain_owner_contact_events(map_owner);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, butsuri::ContactStatus::StartTouching);

        manager.physics.get_mut(&second).unwrap().can_collide = false;
        assert_eq!(
            system.sync_contacts(&mut manager, map_owner, &[first, second]),
            0
        );
        let events = manager.drain_owner_contact_events(map_owner);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, butsuri::ContactStatus::EndTouching);
    }

    #[test]
    fn physics_system_step_body_puts_idle_dynamic_bodies_to_sleep_when_allowed() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized(None);
        manager.initialize_entity(map);
        manager.ensure_map(crate::MapId::new(30), map);
        manager.transforms.get_mut(&map).unwrap().map_id = crate::MapId::new(30);
        manager.ensure_physics_map(map);

        let uid = manager.create_entity_uninitialized(None);
        manager.initialize_entity(uid);
        manager.apply_transform_state(
            uid,
            crate::TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: keisan::Angle::ZERO,
                parent_id: crate::EntityUid::INVALID,
                map_id: crate::MapId::new(30),
                grid_id: crate::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(manager.configure_physics_body(
            uid,
            Some(BodyType::Dynamic),
            Some(true),
            None,
            None,
        ));
        mutate_body(&mut manager, uid, |body| body.sleeping_allowed = true);

        let system = SharedPhysicsSystem::new();
        let state = system.step_body(&mut manager, uid, 0.6).unwrap();
        assert!(!state.awake);
        assert!(state.awake_changed);
        assert!(state.state_changed);
        assert!(!manager.physics.get(&uid).unwrap().awake);
        assert_eq!(manager.physics.get(&uid).unwrap().sleep_time, 0.0);
    }

    #[test]
    fn physics_system_step_body_keeps_idle_dynamic_bodies_awake_when_sleeping_disabled() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized(None);
        manager.initialize_entity(map);
        manager.ensure_map(crate::MapId::new(31), map);
        manager.transforms.get_mut(&map).unwrap().map_id = crate::MapId::new(31);
        manager.ensure_physics_map(map);

        let uid = manager.create_entity_uninitialized(None);
        manager.initialize_entity(uid);
        manager.apply_transform_state(
            uid,
            crate::TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: keisan::Angle::ZERO,
                parent_id: crate::EntityUid::INVALID,
                map_id: crate::MapId::new(31),
                grid_id: crate::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(manager.configure_physics_body(
            uid,
            Some(BodyType::Dynamic),
            Some(true),
            None,
            None,
        ));
        mutate_body(&mut manager, uid, |body| body.sleeping_allowed = false);

        let system = SharedPhysicsSystem::new();
        let state = system.step_body(&mut manager, uid, 0.6).unwrap();
        assert!(state.awake);
        assert!(!state.awake_changed);
        assert!(!state.state_changed);
        assert!(manager.physics.get(&uid).unwrap().awake);
        assert_eq!(manager.physics.get(&uid).unwrap().sleep_time, 0.0);
    }

    #[test]
    fn physics_system_step_body_respects_ignore_gravity_and_damping() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized(None);
        manager.initialize_entity(map);
        manager.ensure_map(crate::MapId::new(36), map);
        manager.transforms.get_mut(&map).unwrap().map_id = crate::MapId::new(36);
        let physics_map = manager.ensure_physics_map(map);
        physics_map.gravity = Vector2::new(0.0, -10.0);

        let uid = manager.create_entity_uninitialized(None);
        manager.initialize_entity(uid);
        manager.apply_transform_state(
            uid,
            crate::TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: keisan::Angle::ZERO,
                parent_id: crate::EntityUid::INVALID,
                map_id: crate::MapId::new(36),
                grid_id: crate::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(manager.configure_physics_body(
            uid,
            Some(BodyType::Dynamic),
            Some(true),
            None,
            None,
        ));
        mutate_body(&mut manager, uid, |body| {
            body.ignore_gravity = true;
            body.linear_damping = 0.5;
            body.angular_damping = 0.5;
            body.linear_velocity = Vector2::new(4.0, 0.0);
            body.angular_velocity = 4.0;
        });

        let system = SharedPhysicsSystem::new();
        let state = system.step_body(&mut manager, uid, 0.5).unwrap();
        assert_eq!(state.linear_velocity, Vector2::new(3.0, 0.0));
        assert_eq!(state.angular_velocity, 3.0);
    }

    #[test]
    fn physics_system_respects_entity_and_map_pause_unless_ignored() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized(None);
        manager.initialize_entity(map);
        manager.ensure_map(crate::MapId::new(60), map);
        manager.transforms.get_mut(&map).unwrap().map_id = crate::MapId::new(60);
        manager.ensure_physics_map(map);

        let uid = manager.create_entity_uninitialized(None);
        manager.initialize_entity(uid);
        manager.apply_transform_state(
            uid,
            crate::TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: keisan::Angle::ZERO,
                parent_id: crate::EntityUid::INVALID,
                map_id: crate::MapId::new(60),
                grid_id: crate::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(manager.configure_physics_body(
            uid,
            Some(BodyType::Dynamic),
            Some(true),
            None,
            None,
        ));
        mutate_body(&mut manager, uid, |body| {
            body.linear_velocity = Vector2::new(2.0, 0.0)
        });

        let system = SharedPhysicsSystem::new();
        assert!(manager.set_entity_paused(uid, true));
        assert!(system.step_body(&mut manager, uid, 0.1).is_none());

        assert!(manager.set_entity_paused(uid, false));
        assert!(manager.set_map_paused(map, true));
        assert!(system.step_body(&mut manager, uid, 0.1).is_none());

        manager.physics.get_mut(&uid).unwrap().ignore_paused = true;
        assert!(system.step_body(&mut manager, uid, 0.1).is_some());
    }

    #[test]
    fn physics_system_gets_map_velocities_from_parent_chain() {
        let mut manager = EntityManager::new();

        let parent = manager.create_entity_uninitialized(None);
        manager.initialize_entity(parent);
        manager.apply_transform_state(
            parent,
            crate::TransformComponentState {
                local_position: Vector2::new(5.0, 0.0),
                rotation: keisan::Angle::from_degrees(90.0),
                parent_id: crate::EntityUid::INVALID,
                map_id: crate::MapId::new(37),
                grid_id: crate::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(manager.configure_physics_body(parent, Some(BodyType::Dynamic), None, None, None,));
        mutate_body(&mut manager, parent, |body| {
            body.linear_velocity = Vector2::new(2.0, 0.0);
            body.angular_velocity = 1.0;
        });

        let child = manager.create_entity_uninitialized(None);
        manager.initialize_entity(child);
        manager.apply_transform_state(
            child,
            crate::TransformComponentState {
                local_position: Vector2::new(1.0, 0.0),
                rotation: keisan::Angle::ZERO,
                parent_id: parent,
                map_id: crate::MapId::new(37),
                grid_id: crate::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(manager.configure_physics_body(child, Some(BodyType::Dynamic), None, None, None,));
        mutate_body(&mut manager, child, |body| {
            body.linear_velocity = Vector2::new(1.0, 0.0);
            body.angular_velocity = 2.0;
        });

        let system = SharedPhysicsSystem::new();
        let (linear, angular) = system.get_map_velocities(&manager, child);
        assert_eq!(angular, 3.0);
        assert!((linear.x - 2.0).abs() < 0.0001);
        assert!(linear.y.abs() < 0.0001);
        let linear_only = system.get_map_velocities(&manager, child).0;
        assert!((linear_only.x - linear.x).abs() < 0.0001);
        assert!((linear_only.y - linear.y).abs() < 0.0001);
        assert_eq!(system.get_map_velocities(&manager, child).1, angular);
    }

    #[test]
    fn physics_system_is_plain_utility_surface() {
        let _first = SharedPhysicsSystem::new();
        let _second = SharedPhysicsSystem::default();
    }
}
