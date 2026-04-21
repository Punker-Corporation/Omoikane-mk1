use crate::{ServerEntityManager, TransformSystem};
use butsuri::CollisionRay;
use keisan::Box2;
use sekai::{BodyType, EntityUid, MapManager, PhysicsQueryHit, SharedPhysicsSystem};
use std::collections::HashSet;

#[derive(Debug, Default, Clone, Copy)]
pub struct PhysicsSystem {
    shared: SharedPhysicsSystem,
    pub metrics_enabled: bool,
}

impl PhysicsSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_metric_flag(&mut self, enabled: bool) {
        self.metrics_enabled = enabled;
    }

    pub fn handle_grid_init(&self, entities: &mut ServerEntityManager, uid: EntityUid) -> bool {
        if !entities.inner.entity_exists(uid) {
            return false;
        }
        let tick = entities.inner.current_tick;
        let body = entities.inner.ensure_physics(uid);
        body.set_body_type(BodyType::Static);
        body.base.last_modified_tick = tick;
        entities.inner.dirty_entity(uid);
        let _ = self.set_can_collide(entities, uid, true);
        true
    }

    pub fn handle_dynamic_init(&self, entities: &mut ServerEntityManager, uid: EntityUid) -> bool {
        if !entities.inner.entity_exists(uid) {
            return false;
        }
        let tick = entities.inner.current_tick;
        let body = entities.inner.ensure_physics(uid);
        body.set_body_type(BodyType::Dynamic);
        body.base.last_modified_tick = tick;
        entities.inner.dirty_entity(uid);
        let _ = self.set_can_collide(entities, uid, true);
        let _ = self.set_awake(entities, uid, true);
        true
    }

    pub fn set_body_type(
        &self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        body_type: BodyType,
    ) -> bool {
        let tick = entities.inner.current_tick;
        let Some(body) = entities.inner.physics.get_mut(&uid) else {
            return false;
        };
        if body.body_type == body_type {
            return false;
        }
        body.set_body_type(body_type);
        body.base.last_modified_tick = tick;
        entities.inner.dirty_entity(uid);
        true
    }

    pub fn set_linear_velocity(
        &self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        velocity: keisan::Vector2,
    ) -> bool {
        let tick = entities.inner.current_tick;
        let Some(body) = entities.inner.physics.get_mut(&uid) else {
            return false;
        };
        let previous = body.linear_velocity;
        body.set_linear_velocity(velocity);
        if body.linear_velocity == previous {
            return false;
        }
        body.base.last_modified_tick = tick;
        entities.inner.dirty_entity(uid);
        true
    }

    pub fn set_awake(
        &self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        awake: bool,
    ) -> bool {
        let tick = entities.inner.current_tick;
        let Some(body) = entities.inner.physics.get_mut(&uid) else {
            return false;
        };
        let previous = body.awake;
        body.set_awake(awake);
        if body.awake == previous {
            return false;
        }
        body.base.last_modified_tick = tick;
        entities.inner.dirty_entity(uid);
        true
    }

    pub fn set_can_collide(
        &self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        can_collide: bool,
    ) -> bool {
        let tick = entities.inner.current_tick;
        let Some(body) = entities.inner.physics.get_mut(&uid) else {
            return false;
        };
        if body.can_collide == can_collide {
            return false;
        }
        body.can_collide = can_collide;
        body.base.last_modified_tick = tick;
        entities.inner.dirty_entity(uid);
        true
    }

    pub fn update(&self, entities: &mut ServerEntityManager, broadphase_owner: EntityUid) -> usize {
        let bodies: Vec<_> = entities.inner.physics.keys().copied().collect();
        self.shared
            .sync_broadphase(&mut entities.inner, broadphase_owner, &bodies)
    }

    pub fn step_simulation(
        &self,
        entities: &mut ServerEntityManager,
        transforms: &mut TransformSystem,
        frame_time: f32,
    ) -> usize {
        let bodies: Vec<_> = entities.inner.physics.keys().copied().collect();
        let mut moved = 0;

        for uid in bodies {
            let Some(body) = entities.inner.physics.get(&uid).cloned() else {
                continue;
            };
            if !body.awake || body.body_type == BodyType::Static || body.linear_velocity == keisan::Vector2::ZERO {
                continue;
            }

            let current = entities
                .inner
                .transforms
                .get(&uid)
                .map(|transform| transform.local_position)
                .unwrap_or(keisan::Vector2::ZERO);

            if transforms.set_local_position(entities, uid, current + body.linear_velocity * frame_time) {
                moved += 1;
            }
        }

        moved
    }

    pub fn sync_map_physics(
        &self,
        entities: &mut ServerEntityManager,
        maps: &MapManager,
    ) -> usize {
        let mut total = 0;

        for map_id in maps.get_all_map_ids() {
            let owner = maps.get_map_entity_id(map_id);
            if !owner.is_valid() || !entities.inner.entity_exists(owner) {
                continue;
            }

            let broadphase_created = !entities.inner.broadphases.contains_key(&owner);
            let physics_map_created = !entities.inner.physics_maps.contains_key(&owner);
            let previous_entries = entities
                .inner
                .broadphases
                .get(&owner)
                .map(|component| component.tree.snapshot())
                .unwrap_or_default();
            let previous_bodies = entities
                .inner
                .physics_maps
                .get(&owner)
                .map(|component| component.bodies.clone())
                .unwrap_or_default();
            let previous_awake = entities
                .inner
                .physics_maps
                .get(&owner)
                .map(|component| component.awake_bodies.clone())
                .unwrap_or_default();

            let bodies: Vec<_> = entities
                .inner
                .physics
                .iter()
                .filter_map(|(uid, body)| {
                    if !body.can_collide {
                        return None;
                    }
                    let transform = entities.inner.transforms.get(uid)?;
                    let fixtures = entities.inner.fixtures.get(uid)?;
                    if transform.map_id != map_id || fixtures.fixtures.is_empty() {
                        return None;
                    }
                    Some(*uid)
                })
                .collect();

            entities.inner.ensure_broadphase(owner);
            total += self.shared.sync_broadphase(&mut entities.inner, owner, &bodies);

            let body_set: HashSet<_> = bodies.iter().copied().collect();
            let awake_set: HashSet<_> = bodies
                .iter()
                .copied()
                .filter(|uid| entities.inner.physics.get(uid).is_some_and(|body| body.awake))
                .collect();
            let physics_map = entities.inner.ensure_physics_map(owner);
            physics_map.bodies = body_set.clone();
            physics_map.awake_bodies = awake_set.clone();
            let _ = self.shared.sync_contacts(&mut entities.inner, owner, &bodies);

            let broadphase_changed = broadphase_created
                || entities
                    .inner
                    .broadphases
                    .get(&owner)
                    .map(|component| component.tree.snapshot())
                    .unwrap_or_default()
                    != previous_entries;
            if broadphase_changed {
                if let Some(component) = entities.inner.broadphases.get_mut(&owner) {
                    component.base.last_modified_tick = entities.inner.current_tick;
                }
                entities.inner.dirty_entity(owner);
            }

            let physics_map_changed =
                physics_map_created || body_set != previous_bodies || awake_set != previous_awake;
            if physics_map_changed {
                if let Some(component) = entities.inner.physics_maps.get_mut(&owner) {
                    component.base.last_modified_tick = entities.inner.current_tick;
                }
                entities.inner.dirty_entity(owner);
            }
        }

        total
    }

    pub fn query_aabb_entities(
        &self,
        entities: &ServerEntityManager,
        broadphase_owner: EntityUid,
        aabb: Box2,
    ) -> Vec<EntityUid> {
        self.shared.query_aabb_entities(&entities.inner, broadphase_owner, aabb)
    }

    pub fn intersect_ray(
        &self,
        entities: &ServerEntityManager,
        broadphase_owner: EntityUid,
        ray: CollisionRay,
        max_length: f32,
        return_on_first_hit: bool,
    ) -> Vec<PhysicsQueryHit> {
        self.shared
            .intersect_ray(&entities.inner, broadphase_owner, ray, max_length, return_on_first_hit)
    }
}

#[cfg(test)]
mod tests {
    use super::PhysicsSystem;
    use crate::{ServerEntityManager, TransformSystem};
    use butsuri::{AabbShape, BodyType, CircleShape, CollisionRay, Fixture, PhysShape};
    use keisan::{Box2, Vector2};
    use sekai::{BroadphaseComponent, MapId, MapManager};

    #[test]
    fn physics_system_initializes_grid_bodies_and_syncs_broadphase() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.create_entity(None);
        let broadphase_uid = entities.create_entity(None);
        entities
            .inner
            .broadphases
            .insert(broadphase_uid, BroadphaseComponent::new());
        let system = PhysicsSystem::new();
        assert!(system.handle_grid_init(&mut entities, uid));
        entities.set_current_tick(jikan::GameTick::new(2));
        let body = entities.inner.ensure_physics(uid);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        entities
            .inner
            .ensure_fixtures(uid)
            .insert_fixture(Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ));
        assert_eq!(system.update(&mut entities, broadphase_uid), 1);
        assert_eq!(
            system.query_aabb_entities(&entities, broadphase_uid, Box2::new(-2.0, -2.0, 2.0, 2.0)),
            vec![uid]
        );
        assert_eq!(
            system
                .intersect_ray(
                    &entities,
                    broadphase_uid,
                    CollisionRay::new(Vector2::new(-5.0, 0.0), Vector2::UNIT_X, -1),
                    10.0,
                    true,
                )[0]
                .entity,
            uid
        );
        assert!(system.set_linear_velocity(&mut entities, uid, Vector2::new(2.0, 0.0)));
        assert_eq!(
            entities.inner.physics.get(&uid).unwrap().base.last_modified_tick,
            jikan::GameTick::new(2)
        );
    }

    #[test]
    fn physics_system_steps_dynamic_bodies_into_transform_positions() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.create_entity(None);
        entities.initialize_entity(uid);
        entities.set_current_tick(jikan::GameTick::new(2));
        let system = PhysicsSystem::new();
        assert!(system.handle_dynamic_init(&mut entities, uid));
        assert!(system.set_linear_velocity(&mut entities, uid, Vector2::new(2.0, 0.0)));

        let mut transforms = TransformSystem::new();
        assert_eq!(system.step_simulation(&mut entities, &mut transforms, 0.5), 1);
        assert_eq!(entities.inner.transforms.get(&uid).unwrap().local_position, Vector2::new(1.0, 0.0));
    }

    #[test]
    fn physics_system_can_toggle_awake_and_collision_flags() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.create_entity(None);
        entities.initialize_entity(uid);
        let system = PhysicsSystem::new();
        assert!(system.handle_dynamic_init(&mut entities, uid));
        assert!(system.set_awake(&mut entities, uid, false));
        assert!(!entities.inner.physics.get(&uid).unwrap().awake);
        assert!(system.set_can_collide(&mut entities, uid, false));
        assert!(!entities.inner.physics.get(&uid).unwrap().can_collide);
    }

    #[test]
    fn physics_system_syncs_broadphase_and_physics_map_per_map() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(2)));
        let map_owner = maps.get_map_entity_id(map_id);

        let uid = entities.create_entity(None);
        entities.initialize_entity(uid);
        {
            let transform = entities.inner.transforms.get_mut(&uid).unwrap();
            transform.map_id = map_id;
            transform.rebuild_for_manager();
        }
        let body = entities.inner.ensure_physics(uid);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        body.awake = true;
        entities
            .inner
            .ensure_fixtures(uid)
            .insert_fixture(Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ));

        let system = PhysicsSystem::new();
        assert_eq!(system.sync_map_physics(&mut entities, &maps), 1);
        assert_eq!(
            system.query_aabb_entities(&entities, map_owner, Box2::new(-2.0, -2.0, 2.0, 2.0)),
            vec![uid]
        );
        let physics_map = entities.inner.physics_maps.get(&map_owner).unwrap();
        assert!(physics_map.bodies.contains(&uid));
        assert!(physics_map.awake_bodies.contains(&uid));
        assert_eq!(physics_map.contact_count(), 0);
    }

    #[test]
    fn physics_system_syncs_runtime_contacts_per_map() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(3)));
        let map_owner = maps.get_map_entity_id(map_id);

        let first = entities.create_entity(None);
        entities.initialize_entity(first);
        {
            let transform = entities.inner.transforms.get_mut(&first).unwrap();
            transform.map_id = map_id;
            transform.rebuild_for_manager();
        }
        let body = entities.inner.ensure_physics(first);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        entities.inner.ensure_fixtures(first).insert_fixture(Fixture::new(
            "main",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        ));

        let second = entities.create_entity(None);
        entities.initialize_entity(second);
        {
            let transform = entities.inner.transforms.get_mut(&second).unwrap();
            transform.map_id = map_id;
            transform.rebuild_for_manager();
        }
        let body = entities.inner.ensure_physics(second);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        entities.inner.ensure_fixtures(second).insert_fixture(Fixture::new(
            "main",
            PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
        ));

        let system = PhysicsSystem::new();
        let _ = system.sync_map_physics(&mut entities, &maps);

        let physics_map = entities.inner.physics_maps.get(&map_owner).unwrap();
        assert_eq!(physics_map.contact_count(), 1);
        let first_id = format!("{}:main", first.raw());
        let second_id = format!("{}:main", second.raw());
        assert!(physics_map.contacts().iter().any(|contact| {
            (contact.fixture_a == first_id && contact.fixture_b == second_id)
                || (contact.fixture_a == second_id && contact.fixture_b == first_id)
        }));
        assert_eq!(physics_map.contacts()[0].manifold.points.len(), 1);
        assert_eq!(physics_map.contacts()[0].manifold.normal, Vector2::UNIT_X);
    }

    #[test]
    fn physics_system_syncs_mixed_contact_manifolds_per_map() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(4)));
        let map_owner = maps.get_map_entity_id(map_id);

        let first = entities.create_entity(None);
        entities.initialize_entity(first);
        {
            let transform = entities.inner.transforms.get_mut(&first).unwrap();
            transform.map_id = map_id;
            transform.rebuild_for_manager();
        }
        let body = entities.inner.ensure_physics(first);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        entities.inner.ensure_fixtures(first).insert_fixture(Fixture::new(
            "aabb",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        ));

        let second = entities.create_entity(None);
        entities.initialize_entity(second);
        {
            let transform = entities.inner.transforms.get_mut(&second).unwrap();
            transform.map_id = map_id;
            transform.local_position = Vector2::new(1.5, 0.0);
            transform.rebuild_for_manager();
        }
        let body = entities.inner.ensure_physics(second);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        entities.inner.ensure_fixtures(second).insert_fixture(Fixture::new(
            "circle",
            PhysShape::Circle(CircleShape::new(Vector2::ZERO, 1.0)),
        ));

        let system = PhysicsSystem::new();
        let _ = system.sync_map_physics(&mut entities, &maps);

        let physics_map = entities.inner.physics_maps.get(&map_owner).unwrap();
        assert_eq!(physics_map.contact_count(), 1);
        assert_eq!(physics_map.contacts()[0].contact_type, butsuri::ContactType::Mixed);
        assert_eq!(physics_map.contacts()[0].manifold.points.len(), 1);
        assert!(physics_map.contacts()[0].manifold.normal.x.abs() > 0.9);
    }

    #[test]
    fn physics_system_intersect_ray_first_hit_returns_closest_entity() {
        let mut entities = ServerEntityManager::new();
        let map_owner = entities.create_entity(None);
        entities.initialize_entity(map_owner);
        entities
            .inner
            .broadphases
            .insert(map_owner, BroadphaseComponent::new());

        let far = entities.create_entity(None);
        entities.initialize_entity(far);
        {
            let transform = entities.inner.transforms.get_mut(&far).unwrap();
            transform.local_position = Vector2::new(8.0, 0.0);
            transform.rebuild_for_manager();
        }
        let body = entities.inner.ensure_physics(far);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        entities.inner.ensure_fixtures(far).insert_fixture(Fixture::new(
            "far",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        ));

        let near = entities.create_entity(None);
        entities.initialize_entity(near);
        {
            let transform = entities.inner.transforms.get_mut(&near).unwrap();
            transform.local_position = Vector2::new(5.0, 0.0);
            transform.rebuild_for_manager();
        }
        let body = entities.inner.ensure_physics(near);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        entities.inner.ensure_fixtures(near).insert_fixture(Fixture::new(
            "near",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        ));

        let system = PhysicsSystem::new();
        assert_eq!(system.update(&mut entities, map_owner), 2);
        let hits = system.intersect_ray(
            &entities,
            map_owner,
            CollisionRay::new(Vector2::ZERO, Vector2::UNIT_X, -1),
            10.0,
            true,
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entity, near);
        assert_eq!(hits[0].fixture_id, "near");
    }
}
