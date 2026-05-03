use crate::ClientEntityManager;
use butsuri::{BodyType, CollisionRay};
use keisan::Box2;
use sekai::{EntityLookupSystem, MapId, PhysicsQueryHit, SharedPhysicsSystem};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct PhysicsSystem {
    shared: SharedPhysicsSystem,
    pub prediction_enabled: bool,
    suppressed_once: HashSet<sekai::EntityUid>,
}

impl Default for PhysicsSystem {
    fn default() -> Self {
        Self {
            shared: SharedPhysicsSystem,
            prediction_enabled: true,
            suppressed_once: HashSet::new(),
        }
    }
}

impl PhysicsSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn suppress_prediction_once(&mut self, uid: sekai::EntityUid) {
        self.suppressed_once.insert(uid);
    }

    pub fn update(&mut self, entities: &mut ClientEntityManager, frame_time: f32) {
        let suppressed = std::mem::take(&mut self.suppressed_once);
        let bodies: Vec<_> = entities.inner.physics.keys().copied().collect();
        let lookup = EntityLookupSystem;
        let mut affected_maps = HashSet::new();
        for uid in bodies {
            if suppressed.contains(&uid) {
                continue;
            }
            let Some(body) = entities.inner.physics.get(&uid).cloned() else {
                continue;
            };
            if !body.awake || body.body_type == BodyType::Static || !body.predict {
                continue;
            }

            if let Some(transform) = entities.inner.transforms.get_mut(&uid) {
                transform.local_position =
                    transform.local_position + body.linear_velocity * frame_time;
                transform.rebuild_for_manager();
                affected_maps.insert(transform.map_id);
            }
            lookup.update_subtree_bounds(&mut entities.inner, uid);
        }

        for map_id in affected_maps {
            self.sync_predicted_map(entities, map_id);
        }
    }

    pub fn frame_update(&mut self, entities: &mut ClientEntityManager, frame_time: f32) {
        if self.prediction_enabled {
            self.update(entities, frame_time);
        }
    }

    pub fn world_aabb(
        &self,
        entities: &ClientEntityManager,
        uid: sekai::EntityUid,
    ) -> Option<keisan::Box2> {
        self.shared.get_world_aabb(&entities.inner, uid)
    }

    pub fn query_aabb_entities(
        &self,
        entities: &ClientEntityManager,
        broadphase_owner: sekai::EntityUid,
        aabb: Box2,
    ) -> Vec<sekai::EntityUid> {
        self.shared
            .query_aabb_entities(&entities.inner, broadphase_owner, aabb)
    }

    pub fn intersect_ray(
        &self,
        entities: &ClientEntityManager,
        broadphase_owner: sekai::EntityUid,
        ray: CollisionRay,
        max_length: f32,
        return_on_first_hit: bool,
    ) -> Vec<PhysicsQueryHit> {
        self.shared.intersect_ray(
            &entities.inner,
            broadphase_owner,
            ray,
            max_length,
            return_on_first_hit,
        )
    }

    fn sync_predicted_map(&self, entities: &mut ClientEntityManager, map_id: MapId) {
        entities.sync_map_physics_runtime(map_id);
    }
}

#[cfg(test)]
mod tests {
    use super::PhysicsSystem;
    use crate::ClientEntityManager;
    use butsuri::{AabbShape, BodyType, CircleShape, CollisionRay, Fixture, PhysShape};
    use keisan::{Box2, Vector2};
    use sekai::EntityUid;

    #[test]
    fn physics_system_integrates_dynamic_bodies() {
        let mut entities = ClientEntityManager::new();
        let uid = entities.create_entity(None, EntityUid::new(5));
        entities.initialize_entity(uid);
        let body = entities.inner.ensure_physics(uid);
        body.set_body_type(BodyType::Dynamic);
        body.awake = true;
        body.can_collide = true;
        body.linear_velocity = keisan::Vector2::new(2.0, 0.0);
        entities
            .inner
            .ensure_fixtures(uid)
            .insert_fixture(Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ));
        let broadphase_uid = entities.create_entity(None, EntityUid::new(50));
        entities
            .inner
            .broadphases
            .insert(broadphase_uid, sekai::BroadphaseComponent::new());
        let mut system = PhysicsSystem::new();
        system.update(&mut entities, 0.5);
        sekai::SharedPhysicsSystem.sync_broadphase(&mut entities.inner, broadphase_uid, &[uid]);
        assert_eq!(
            entities
                .inner
                .transforms
                .get(&uid)
                .unwrap()
                .local_position
                .x,
            1.0
        );
        assert!(system.world_aabb(&entities, uid).is_some());
        assert_eq!(
            system.query_aabb_entities(&entities, broadphase_uid, Box2::new(-1.0, -2.0, 3.0, 2.0)),
            vec![uid]
        );
        assert_eq!(
            system.intersect_ray(
                &entities,
                broadphase_uid,
                CollisionRay::new(Vector2::new(-5.0, 0.0), Vector2::UNIT_X, -1),
                10.0,
                true,
            )[0]
            .entity,
            uid
        );
    }

    #[test]
    fn physics_system_can_skip_prediction_for_one_frame() {
        let mut entities = ClientEntityManager::new();
        let uid = entities.create_entity(None, EntityUid::new(6));
        entities.initialize_entity(uid);
        let body = entities.inner.ensure_physics(uid);
        body.set_body_type(BodyType::Dynamic);
        body.awake = true;
        body.predict = true;
        body.linear_velocity = keisan::Vector2::new(2.0, 0.0);

        let mut system = PhysicsSystem::new();
        system.suppress_prediction_once(uid);
        system.update(&mut entities, 0.5);
        assert_eq!(
            entities.inner.transforms.get(&uid).unwrap().local_position,
            Vector2::ZERO
        );
        system.update(&mut entities, 0.5);
        assert_eq!(
            entities.inner.transforms.get(&uid).unwrap().local_position,
            Vector2::new(1.0, 0.0)
        );
    }

    #[test]
    fn physics_system_keeps_predicted_spatial_runtime_in_sync() {
        let mut entities = ClientEntityManager::new();
        let map_owner = entities.ensure_map_entity(sekai::MapId::new(2));
        entities
            .inner
            .broadphases
            .insert(map_owner, sekai::BroadphaseComponent::new());

        let uid = entities.create_entity(None, EntityUid::new(7));
        entities.initialize_entity(uid);
        let _ = entities.inner.apply_transform_state(
            uid,
            sekai::TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: keisan::Angle::ZERO,
                parent_id: sekai::EntityUid::INVALID,
                map_id: sekai::MapId::new(2),
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        entities.inner.transforms.get_mut(&uid).unwrap().map_id = sekai::MapId::new(2);
        entities.inner.ensure_lookup(map_owner);
        let body = entities.inner.ensure_physics(uid);
        body.set_body_type(BodyType::Dynamic);
        body.awake = true;
        body.predict = true;
        body.can_collide = true;
        body.linear_velocity = keisan::Vector2::new(2.0, 0.0);
        entities
            .inner
            .ensure_fixtures(uid)
            .insert_fixture(Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ));

        let mut system = PhysicsSystem::new();
        system.update(&mut entities, 0.5);

        assert_eq!(
            system.query_aabb_entities(&entities, map_owner, Box2::new(0.25, -1.0, 2.0, 1.0)),
            vec![uid]
        );
    }

    #[test]
    fn physics_system_keeps_predicted_contact_runtime_in_sync() {
        let mut entities = ClientEntityManager::new();
        let map_owner = entities.ensure_map_entity(sekai::MapId::new(3));
        entities
            .inner
            .broadphases
            .insert(map_owner, sekai::BroadphaseComponent::new());
        entities.inner.ensure_physics_map(map_owner);

        for uid in [EntityUid::new(8), EntityUid::new(9)] {
            entities.create_entity(None, uid);
            entities.initialize_entity(uid);
            let _ = entities.inner.apply_transform_state(
                uid,
                sekai::TransformComponentState {
                    local_position: Vector2::ZERO,
                    rotation: keisan::Angle::ZERO,
                    parent_id: sekai::EntityUid::INVALID,
                    map_id: sekai::MapId::new(3),
                    grid_id: sekai::GridId::INVALID,
                    no_local_rotation: false,
                    anchored: false,
                },
            );
            entities.inner.transforms.get_mut(&uid).unwrap().map_id = sekai::MapId::new(3);
            let body = entities.inner.ensure_physics(uid);
            body.set_body_type(BodyType::Dynamic);
            body.awake = true;
            body.predict = true;
            body.can_collide = true;
            entities
                .inner
                .ensure_fixtures(uid)
                .insert_fixture(Fixture::new(
                    "main",
                    PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
                ));
        }

        let system = PhysicsSystem::new();
        system.sync_predicted_map(&mut entities, sekai::MapId::new(3));

        let physics_map = entities.inner.physics_maps.get(&map_owner).unwrap();
        assert_eq!(physics_map.contact_count(), 1);
        let first_id = format!("{}:main", EntityUid::new(8).raw());
        let second_id = format!("{}:main", EntityUid::new(9).raw());
        assert!(physics_map.contacts().iter().any(|contact| {
            (contact.fixture_a == first_id && contact.fixture_b == second_id)
                || (contact.fixture_a == second_id && contact.fixture_b == first_id)
        }));
        assert_eq!(physics_map.contacts()[0].manifold.points.len(), 1);
        assert_eq!(physics_map.contacts()[0].manifold.normal, Vector2::UNIT_X);
    }

    #[test]
    fn physics_system_keeps_predicted_mixed_contact_manifold_in_sync() {
        let mut entities = ClientEntityManager::new();
        let map_owner = entities.ensure_map_entity(sekai::MapId::new(4));
        entities
            .inner
            .broadphases
            .insert(map_owner, sekai::BroadphaseComponent::new());
        entities.inner.ensure_physics_map(map_owner);

        let first = EntityUid::new(15);
        entities.create_entity(None, first);
        entities.initialize_entity(first);
        let _ = entities.inner.apply_transform_state(
            first,
            sekai::TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: keisan::Angle::ZERO,
                parent_id: sekai::EntityUid::INVALID,
                map_id: sekai::MapId::new(4),
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        entities.inner.transforms.get_mut(&first).unwrap().map_id = sekai::MapId::new(4);
        let body = entities.inner.ensure_physics(first);
        body.set_body_type(BodyType::Dynamic);
        body.awake = true;
        body.predict = true;
        body.can_collide = true;
        entities
            .inner
            .ensure_fixtures(first)
            .insert_fixture(Fixture::new(
                "aabb",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ));

        let second = EntityUid::new(16);
        entities.create_entity(None, second);
        entities.initialize_entity(second);
        let _ = entities.inner.apply_transform_state(
            second,
            sekai::TransformComponentState {
                local_position: Vector2::new(1.5, 0.0),
                rotation: keisan::Angle::ZERO,
                parent_id: sekai::EntityUid::INVALID,
                map_id: sekai::MapId::new(4),
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        entities.inner.transforms.get_mut(&second).unwrap().map_id = sekai::MapId::new(4);
        let body = entities.inner.ensure_physics(second);
        body.set_body_type(BodyType::Dynamic);
        body.awake = true;
        body.predict = true;
        body.can_collide = true;
        entities
            .inner
            .ensure_fixtures(second)
            .insert_fixture(Fixture::new(
                "circle",
                PhysShape::Circle(CircleShape::new(Vector2::ZERO, 1.0)),
            ));

        let system = PhysicsSystem::new();
        system.sync_predicted_map(&mut entities, sekai::MapId::new(4));

        let physics_map = entities.inner.physics_maps.get(&map_owner).unwrap();
        assert_eq!(physics_map.contact_count(), 1);
        assert_eq!(
            physics_map.contacts()[0].contact_type,
            butsuri::ContactType::Mixed
        );
        assert_eq!(physics_map.contacts()[0].manifold.points.len(), 1);
        assert!(physics_map.contacts()[0].manifold.normal.x.abs() > 0.9);
    }

    #[test]
    fn physics_system_intersect_ray_first_hit_returns_closest_entity() {
        let mut entities = ClientEntityManager::new();
        let broadphase_uid = entities.create_entity(None, EntityUid::new(60));
        entities.initialize_entity(broadphase_uid);
        entities
            .inner
            .broadphases
            .insert(broadphase_uid, sekai::BroadphaseComponent::new());

        let far = entities.create_entity(None, EntityUid::new(61));
        entities.initialize_entity(far);
        let _ = entities.inner.apply_transform_state(
            far,
            sekai::TransformComponentState {
                local_position: Vector2::new(8.0, 0.0),
                rotation: keisan::Angle::ZERO,
                parent_id: sekai::EntityUid::INVALID,
                map_id: sekai::MapId::NULLSPACE,
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(far);
        body.set_body_type(BodyType::Dynamic);
        body.awake = true;
        body.can_collide = true;
        entities
            .inner
            .ensure_fixtures(far)
            .insert_fixture(Fixture::new(
                "far",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ));

        let near = entities.create_entity(None, EntityUid::new(62));
        entities.initialize_entity(near);
        let _ = entities.inner.apply_transform_state(
            near,
            sekai::TransformComponentState {
                local_position: Vector2::new(5.0, 0.0),
                rotation: keisan::Angle::ZERO,
                parent_id: sekai::EntityUid::INVALID,
                map_id: sekai::MapId::NULLSPACE,
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(near);
        body.set_body_type(BodyType::Dynamic);
        body.awake = true;
        body.can_collide = true;
        entities
            .inner
            .ensure_fixtures(near)
            .insert_fixture(Fixture::new(
                "near",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ));

        let system = PhysicsSystem::new();
        sekai::SharedPhysicsSystem.sync_broadphase(
            &mut entities.inner,
            broadphase_uid,
            &[far, near],
        );
        let hits = system.intersect_ray(
            &entities,
            broadphase_uid,
            CollisionRay::new(Vector2::ZERO, Vector2::UNIT_X, -1),
            10.0,
            true,
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entity, near);
        assert_eq!(hits[0].fixture_id, "near");
    }
}
