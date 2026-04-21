use crate::{EntityManager, EntityUid};
use butsuri::{AabbShape, BodyType, CircleShape, CollisionRay, ContactManager, ContactManifold, ContactManifoldPoint, Fixture, PhysShape, RayCastHit, Transform as PhysicsTransform};
use keisan::{Box2, Vector2};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq)]
pub struct PhysicsQueryHit {
    pub entity: EntityUid,
    pub fixture_id: String,
    pub distance: f32,
    pub hit_pos: Vector2,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SharedPhysicsSystem;

impl SharedPhysicsSystem {
    pub fn set_linear_velocity(&self, manager: &mut EntityManager, uid: EntityUid, velocity: Vector2) -> bool {
        let Some(body) = manager.physics.get_mut(&uid) else {
            return false;
        };
        body.set_linear_velocity(velocity);
        true
    }

    pub fn get_world_aabb(&self, manager: &EntityManager, uid: EntityUid) -> Option<Box2> {
        let body = manager.physics.get(&uid)?;
        let xform = manager.transforms.get(&uid)?;
        let fixtures = manager.fixtures.get(&uid)?;
        body.get_aabb(xform, fixtures, manager)
    }

    pub fn get_hard_aabb(&self, manager: &EntityManager, uid: EntityUid) -> Option<Box2> {
        let body = manager.physics.get(&uid)?;
        let xform = manager.transforms.get(&uid)?;
        let fixtures = manager.fixtures.get(&uid)?;
        body.get_hard_aabb(xform, fixtures, manager)
    }

    pub fn sync_broadphase(&self, manager: &mut EntityManager, broadphase_owner: EntityUid, bodies: &[EntityUid]) -> usize {
        let mut pending = Vec::new();

        for uid in bodies {
            let Some(body) = manager.physics.get(uid) else {
                continue;
            };
            if !body.can_collide {
                continue;
            }

            let Some(xform) = manager.transforms.get(uid) else {
                continue;
            };
            let Some(fixtures) = manager.fixtures.get(uid) else {
                continue;
            };
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
            broadphase.tree.insert(*owner_id, fixture.clone(), *transform);
        }

        pending.len()
    }

    pub fn sync_contacts(&self, manager: &mut EntityManager, map_owner: EntityUid, bodies: &[EntityUid]) -> usize {
        let mut contact_manager = ContactManager::new();
        let previous_contacts = manager
            .physics_maps
            .get(&map_owner)
            .map(|map| map.contacts().to_vec())
            .unwrap_or_default();
        let mut body_fixtures = Vec::new();

        for uid in bodies {
            let Some(body) = manager.physics.get(uid) else {
                continue;
            };
            if !body.can_collide {
                continue;
            }
            let Some(transform) = manager.transforms.get(uid) else {
                continue;
            };
            let Some(fixtures) = manager.fixtures.get(uid) else {
                continue;
            };
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
            body_fixtures.push((*uid, body.body_type, fixture_data));
        }

        for first in 0..body_fixtures.len() {
            let (uid_a, body_type_a, fixtures_a) = &body_fixtures[first];
            for second in first + 1..body_fixtures.len() {
                let (uid_b, body_type_b, fixtures_b) = &body_fixtures[second];
                if *body_type_a == BodyType::Static && *body_type_b == BodyType::Static {
                    continue;
                }
                if self.joint_blocks_collision(manager, *uid_a, *uid_b, *body_type_a, *body_type_b) {
                    continue;
                }

                for (fixture_a, transform_a) in fixtures_a {
                    for (fixture_b, transform_b) in fixtures_b {
                        if !Self::fixtures_can_collide(fixture_a, fixture_b) {
                            continue;
                        }
                        if !fixture_a.compute_aabb(*transform_a).intersects(fixture_b.compute_aabb(*transform_b)) {
                            continue;
                        }

                        let fixture_a_key = format!("{}:{}", uid_a.raw(), fixture_a.id);
                        let fixture_b_key = format!("{}:{}", uid_b.raw(), fixture_b.id);
                        let (fixture_a_key, ordered_fixture_a, ordered_transform_a, fixture_b_key, ordered_fixture_b, ordered_transform_b) =
                            if fixture_a_key <= fixture_b_key {
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
                        let contact_type = match (&ordered_fixture_a.shape, &ordered_fixture_b.shape) {
                            (PhysShape::Aabb(_), PhysShape::Aabb(_)) => butsuri::ContactType::Aabb,
                            (PhysShape::Circle(_), PhysShape::Circle(_)) => butsuri::ContactType::Circle,
                            _ => butsuri::ContactType::Mixed,
                        };

                        let index = if let Some(previous) =
                            Self::find_previous_contact(&previous_contacts, &fixture_a_key, &fixture_b_key)
                        {
                            let mut contact = previous.clone();
                            contact.fixture_a = fixture_a_key;
                            contact.fixture_b = fixture_b_key;
                            contact.contact_type = contact_type;
                            contact.reset_friction(ordered_fixture_a.friction, ordered_fixture_b.friction);
                            contact.reset_restitution(ordered_fixture_a.restitution, ordered_fixture_b.restitution);
                            contact.manifold =
                                Self::merge_manifold_impulses(&previous.manifold, manifold);
                            let status = contact.update_touching(true);
                            let cloned = contact.clone();
                            let index = contact_manager.insert_contact(contact);
                            if status != butsuri::ContactStatus::NoContact {
                                if let Some(physics_map) = manager.physics_maps.get_mut(&map_owner) {
                                    physics_map.queue_contact_event(status, cloned);
                                }
                            }
                            index
                        } else {
                            let Some(index) = contact_manager.add_pair_with_keys(
                                &fixture_a_key,
                                ordered_fixture_a,
                                &fixture_b_key,
                                ordered_fixture_b,
                            ) else {
                                continue;
                            };
                            if let Some(contact) = contact_manager.contact_mut(index) {
                                contact.manifold = manifold;
                            }
                            if let Some(status) = contact_manager.update_touching(index, true) {
                                if status != butsuri::ContactStatus::NoContact {
                                    if let Some(contact) = contact_manager.contact_mut(index).cloned() {
                                        if let Some(physics_map) = manager.physics_maps.get_mut(&map_owner) {
                                            physics_map.queue_contact_event(status, contact);
                                        }
                                    }
                                }
                            }
                            index
                        };

                        if let Some(contact) = contact_manager.contact_mut(index) {
                            contact.enabled = true;
                        }
                    }
                }
            }
        }

        let count = contact_manager.contact_count();
        let Some(physics_map) = manager.physics_maps.get_mut(&map_owner) else {
            return 0;
        };
        for previous in &previous_contacts {
            let still_present = contact_manager.contacts().iter().any(|current| {
                current.fixture_a == previous.fixture_a && current.fixture_b == previous.fixture_b
            });
            if still_present {
                continue;
            }
            let mut ended = previous.clone();
            let status = ended.update_touching(false);
            physics_map.queue_contact_event(status, ended);
        }
        physics_map.replace_contacts(contact_manager);
        count
    }

    pub fn query_aabb_entities(&self, manager: &EntityManager, broadphase_owner: EntityUid, aabb: Box2) -> Vec<EntityUid> {
        let Some(broadphase) = manager.broadphases.get(&broadphase_owner) else {
            return Vec::new();
        };

        let mut entities = BTreeSet::new();
        for hit in broadphase.tree.query_aabb(aabb) {
            entities.insert(EntityUid::new(hit.owner_id));
        }
        entities.into_iter().collect()
    }

    pub fn intersect_ray(
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
            .filter_map(|RayCastHit { distance, hit_pos, hit }| {
                let entry = broadphase.tree.entry(hit.fixture_index)?;
                Some(PhysicsQueryHit {
                    entity: EntityUid::new(hit.owner_id),
                    fixture_id: entry.fixture.id.clone(),
                    distance,
                    hit_pos,
                })
            })
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

    fn merge_manifold_impulses(previous: &ContactManifold, mut current: ContactManifold) -> ContactManifold {
        if previous.points.len() == current.points.len() {
            for (previous_point, current_point) in previous.points.iter().zip(current.points.iter_mut()) {
                current_point.normal_impulse = previous_point.normal_impulse;
                current_point.tangent_impulse = previous_point.tangent_impulse;
            }
        }
        current
    }

    fn find_previous_contact<'a>(
        contacts: &'a [butsuri::Contact],
        fixture_a: &str,
        fixture_b: &str,
    ) -> Option<&'a butsuri::Contact> {
        contacts.iter().find(|contact| {
            (contact.fixture_a == fixture_a && contact.fixture_b == fixture_b)
                || (contact.fixture_a == fixture_b && contact.fixture_b == fixture_a)
        })
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
        let bounds_a = shape_a.compute_aabb(transform_a);
        let bounds_b = shape_b.compute_aabb(transform_b);
        let intersection = bounds_a.intersect(bounds_b);
        if intersection.is_empty() {
            return ContactManifold::default();
        }

        let overlap_x = intersection.width();
        let overlap_y = intersection.height();
        let center_delta = bounds_b.center() - bounds_a.center();
        let normal = if overlap_x <= overlap_y {
            Vector2::new(if center_delta.x >= 0.0 { 1.0 } else { -1.0 }, 0.0)
        } else {
            Vector2::new(0.0, if center_delta.y >= 0.0 { 1.0 } else { -1.0 })
        };
        let world_point = intersection.center();
        ContactManifold {
            normal,
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
        let bounds = shape_aabb.compute_aabb(transform_aabb);
        let center = transform_circle.mul(shape_circle.position);
        let closest = bounds.closest_point(center);
        let delta = center - closest;
        let distance = delta.length();

        let (normal, world_point) = if distance > 0.0001 {
            if distance > shape_circle.radius {
                return ContactManifold::default();
            }
            (delta / distance, closest)
        } else {
            let left = (center.x - bounds.left).abs();
            let right = (bounds.right - center.x).abs();
            let bottom = (center.y - bounds.bottom).abs();
            let top = (bounds.top - center.y).abs();

            if left <= right && left <= bottom && left <= top {
                (Vector2::new(-1.0, 0.0), Vector2::new(bounds.left, center.y))
            } else if right <= bottom && right <= top {
                (Vector2::new(1.0, 0.0), Vector2::new(bounds.right, center.y))
            } else if bottom <= top {
                (Vector2::new(0.0, -1.0), Vector2::new(center.x, bounds.bottom))
            } else {
                (Vector2::new(0.0, 1.0), Vector2::new(center.x, bounds.top))
            }
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
        let manifold =
            Self::compute_aabb_circle_manifold(shape_aabb, transform_aabb, shape_circle, transform_circle);
        if manifold.points.is_empty() {
            return manifold;
        }

        let center = transform_circle.mul(shape_circle.position);
        let world_point = center + (-manifold.normal) * shape_circle.radius.min(shape_circle.radius);
        ContactManifold {
            normal: -manifold.normal,
            points: vec![ContactManifoldPoint {
                local_point: transform_circle.mul_t(world_point),
                normal_impulse: 0.0,
                tangent_impulse: 0.0,
            }],
        }
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
                let matches_pair = (joint.body_a_uid == uid_a.raw() && joint.body_b_uid == uid_b.raw())
                    || (joint.body_a_uid == uid_b.raw() && joint.body_b_uid == uid_a.raw());
                matches_pair && joint.blocks_collisions(body_type_a, body_type_b)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{PhysicsQueryHit, SharedPhysicsSystem};
    use crate::{BroadphaseComponent, EntityManager};
    use butsuri::{AabbShape, BodyType, CircleShape, CollisionRay, Fixture, Joint, JointType, PhysShape};
    use keisan::{Box2, Vector2};

    #[test]
    fn physics_system_computes_aabb_and_syncs_broadphase() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.initialize_entity(uid);

        let body = manager.ensure_physics(uid);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);

        manager
            .ensure_fixtures(uid)
            .insert_fixture(Fixture::new("main", PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0))));

        let broadphase_uid = manager.create_entity_uninitialized(None);
        manager.broadphases.insert(broadphase_uid, BroadphaseComponent::new());

        let system = SharedPhysicsSystem;
        let aabb = system.get_world_aabb(&manager, uid).unwrap();
        assert_eq!(aabb, Box2::new(-1.0, -1.0, 1.0, 1.0));
        assert_eq!(system.sync_broadphase(&mut manager, broadphase_uid, &[uid]), 1);
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
        let body = manager.ensure_physics(first);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        let mut first_fixture =
            Fixture::new("first", PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)));
        first_fixture.collision_layer = 1 << 0;
        manager.ensure_fixtures(first).insert_fixture(first_fixture);
        manager.transforms.get_mut(&first).unwrap().local_position = Vector2::new(5.0, 0.0);
        manager.transforms.get_mut(&first).unwrap().rebuild_for_manager();

        let second = manager.create_entity_uninitialized(None);
        manager.initialize_entity(second);
        let body = manager.ensure_physics(second);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        let mut second_fixture =
            Fixture::new("second", PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)));
        second_fixture.collision_layer = 1 << 1;
        manager.ensure_fixtures(second).insert_fixture(second_fixture);
        manager.transforms.get_mut(&second).unwrap().local_position = Vector2::new(8.0, 0.0);
        manager.transforms.get_mut(&second).unwrap().rebuild_for_manager();

        let broadphase_uid = manager.create_entity_uninitialized(None);
        manager.broadphases.insert(broadphase_uid, BroadphaseComponent::new());

        let system = SharedPhysicsSystem;
        assert_eq!(system.sync_broadphase(&mut manager, broadphase_uid, &[first, second]), 2);
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
        let body = manager.ensure_physics(far);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        manager.ensure_fixtures(far).insert_fixture(Fixture::new(
            "far",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        ));
        manager.transforms.get_mut(&far).unwrap().local_position = Vector2::new(8.0, 0.0);
        manager.transforms.get_mut(&far).unwrap().rebuild_for_manager();

        let near = manager.create_entity_uninitialized(None);
        manager.initialize_entity(near);
        let body = manager.ensure_physics(near);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        manager.ensure_fixtures(near).insert_fixture(Fixture::new(
            "near",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        ));
        manager.transforms.get_mut(&near).unwrap().local_position = Vector2::new(5.0, 0.0);
        manager.transforms.get_mut(&near).unwrap().rebuild_for_manager();

        let broadphase_uid = manager.create_entity_uninitialized(None);
        manager.broadphases.insert(broadphase_uid, BroadphaseComponent::new());

        let system = SharedPhysicsSystem;
        assert_eq!(system.sync_broadphase(&mut manager, broadphase_uid, &[far, near]), 2);
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
    fn physics_system_syncs_contacts_per_map_and_respects_joint_collision_filters() {
        let mut manager = EntityManager::new();
        let map_owner = manager.create_entity_uninitialized(None);
        manager.initialize_entity(map_owner);
        manager.ensure_physics_map(map_owner);

        let first = manager.create_entity_uninitialized(None);
        manager.initialize_entity(first);
        let body = manager.ensure_physics(first);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        manager.ensure_fixtures(first).insert_fixture(Fixture::new(
            "main",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        ));

        let second = manager.create_entity_uninitialized(None);
        manager.initialize_entity(second);
        let body = manager.ensure_physics(second);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        manager.ensure_fixtures(second).insert_fixture(Fixture::new(
            "main",
            PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
        ));

        let system = SharedPhysicsSystem;
        assert_eq!(system.sync_contacts(&mut manager, map_owner, &[first, second]), 1);
        let contacts = manager.physics_maps.get(&map_owner).unwrap().contacts();
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].fixture_a, format!("{}:main", first.raw()));
        assert_eq!(contacts[0].fixture_b, format!("{}:main", second.raw()));
        assert!(contacts[0].is_touching);

        let mut joint = Joint::new(first.raw(), second.raw(), JointType::Distance);
        joint.id = "rope".to_string();
        joint.collide_connected = false;
        manager.ensure_joints(first).add_joint(joint);
        assert_eq!(system.sync_contacts(&mut manager, map_owner, &[first, second]), 0);
        assert_eq!(manager.physics_maps.get(&map_owner).unwrap().contact_count(), 0);
    }

    #[test]
    fn physics_system_syncs_aabb_contact_manifolds() {
        let mut manager = EntityManager::new();
        let map_owner = manager.create_entity_uninitialized(None);
        manager.initialize_entity(map_owner);
        manager.ensure_physics_map(map_owner);

        let first = manager.create_entity_uninitialized(None);
        manager.initialize_entity(first);
        let body = manager.ensure_physics(first);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        manager.ensure_fixtures(first).insert_fixture(Fixture::new(
            "first",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        ));

        let second = manager.create_entity_uninitialized(None);
        manager.initialize_entity(second);
        let body = manager.ensure_physics(second);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        manager.ensure_fixtures(second).insert_fixture(Fixture::new(
            "second",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        ));
        manager.transforms.get_mut(&second).unwrap().local_position = Vector2::new(1.5, 0.0);
        manager.transforms.get_mut(&second).unwrap().rebuild_for_manager();

        let system = SharedPhysicsSystem;
        assert_eq!(system.sync_contacts(&mut manager, map_owner, &[first, second]), 1);
        let contact = &manager.physics_maps.get(&map_owner).unwrap().contacts()[0];
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
        let body = manager.ensure_physics(first);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        manager.ensure_fixtures(first).insert_fixture(Fixture::new(
            "first",
            PhysShape::Circle(CircleShape::new(Vector2::ZERO, 1.0)),
        ));

        let second = manager.create_entity_uninitialized(None);
        manager.initialize_entity(second);
        let body = manager.ensure_physics(second);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        manager.ensure_fixtures(second).insert_fixture(Fixture::new(
            "second",
            PhysShape::Circle(CircleShape::new(Vector2::ZERO, 1.0)),
        ));
        manager.transforms.get_mut(&second).unwrap().local_position = Vector2::new(1.5, 0.0);
        manager.transforms.get_mut(&second).unwrap().rebuild_for_manager();

        let system = SharedPhysicsSystem;
        assert_eq!(system.sync_contacts(&mut manager, map_owner, &[first, second]), 1);
        let contact = &manager.physics_maps.get(&map_owner).unwrap().contacts()[0];
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
        let body = manager.ensure_physics(first);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        manager.ensure_fixtures(first).insert_fixture(Fixture::new(
            "aabb",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        ));

        let second = manager.create_entity_uninitialized(None);
        manager.initialize_entity(second);
        let body = manager.ensure_physics(second);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        manager.ensure_fixtures(second).insert_fixture(Fixture::new(
            "circle",
            PhysShape::Circle(CircleShape::new(Vector2::ZERO, 1.0)),
        ));
        manager.transforms.get_mut(&second).unwrap().local_position = Vector2::new(1.5, 0.0);
        manager.transforms.get_mut(&second).unwrap().rebuild_for_manager();

        let system = SharedPhysicsSystem;
        assert_eq!(system.sync_contacts(&mut manager, map_owner, &[first, second]), 1);
        let contact = &manager.physics_maps.get(&map_owner).unwrap().contacts()[0];
        assert_eq!(contact.contact_type, butsuri::ContactType::Mixed);
        assert_eq!(contact.manifold.points.len(), 1);
        assert!(contact.manifold.normal.x.abs() > 0.9);
    }

    #[test]
    fn physics_system_preserves_contact_touching_state_and_impulses_across_syncs() {
        let mut manager = EntityManager::new();
        let map_owner = manager.create_entity_uninitialized(None);
        manager.initialize_entity(map_owner);
        manager.ensure_physics_map(map_owner);

        let first = manager.create_entity_uninitialized(None);
        manager.initialize_entity(first);
        let body = manager.ensure_physics(first);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        manager.ensure_fixtures(first).insert_fixture(Fixture::new(
            "first",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        ));

        let second = manager.create_entity_uninitialized(None);
        manager.initialize_entity(second);
        let body = manager.ensure_physics(second);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        manager.ensure_fixtures(second).insert_fixture(Fixture::new(
            "second",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        ));
        manager.transforms.get_mut(&second).unwrap().local_position = Vector2::new(1.5, 0.0);
        manager.transforms.get_mut(&second).unwrap().rebuild_for_manager();

        let system = SharedPhysicsSystem;
        assert_eq!(system.sync_contacts(&mut manager, map_owner, &[first, second]), 1);
        {
            let physics_map = manager.physics_maps.get_mut(&map_owner).unwrap();
            let contact = physics_map.contact_mut(0).unwrap();
            contact.manifold.points[0].normal_impulse = 3.5;
            contact.manifold.points[0].tangent_impulse = 1.25;
        }

        assert_eq!(system.sync_contacts(&mut manager, map_owner, &[first, second]), 1);
        let contact = &manager.physics_maps.get(&map_owner).unwrap().contacts()[0];
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
        let body = manager.ensure_physics(first);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        manager.ensure_fixtures(first).insert_fixture(Fixture::new(
            "first",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        ));

        let second = manager.create_entity_uninitialized(None);
        manager.initialize_entity(second);
        let body = manager.ensure_physics(second);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        manager.ensure_fixtures(second).insert_fixture(Fixture::new(
            "second",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        ));
        manager.transforms.get_mut(&second).unwrap().local_position = Vector2::new(1.5, 0.0);
        manager.transforms.get_mut(&second).unwrap().rebuild_for_manager();

        let system = SharedPhysicsSystem;
        assert_eq!(system.sync_contacts(&mut manager, map_owner, &[first, second]), 1);
        let events = manager.physics_maps.get_mut(&map_owner).unwrap().drain_contact_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, butsuri::ContactStatus::StartTouching);

        manager.physics.get_mut(&second).unwrap().can_collide = false;
        assert_eq!(system.sync_contacts(&mut manager, map_owner, &[first, second]), 0);
        let events = manager.physics_maps.get_mut(&map_owner).unwrap().drain_contact_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, butsuri::ContactStatus::EndTouching);
    }
}
