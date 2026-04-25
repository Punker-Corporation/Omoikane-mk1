use crate::ClientEntityManager;
use sekai::MapId;
use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct PhysicsSystem {
    pub prediction_enabled: bool,
    suppressed_once: HashSet<sekai::EntityUid>,
}

impl PhysicsSystem {
    pub fn new() -> Self {
        Self {
            prediction_enabled: true,
            suppressed_once: HashSet::new(),
        }
    }

    pub fn suppress_prediction_once(&mut self, uid: sekai::EntityUid) {
        self.suppressed_once.insert(uid);
    }

    pub fn update(&mut self, entities: &mut ClientEntityManager, frame_time: f32) {
        let suppressed = std::mem::take(&mut self.suppressed_once);
        let bodies = entities.inner.physics_body_entities(true);
        let mut affected_maps = HashSet::new();
        for uid in bodies {
            if suppressed.contains(&uid) {
                continue;
            }
            let previous_map = entities
                .inner
                .transforms
                .get(&uid)
                .map(|transform| transform.map_id)
                .unwrap_or(MapId::NULLSPACE);
            let previous_awake = entities
                .inner
                .physics
                .get(&uid)
                .map(|body| body.awake)
                .unwrap_or(false);
            let Some(step) = entities.inner.step_physics_body(uid, frame_time) else {
                continue;
            };
            let predict = entities
                .inner
                .physics
                .get(&uid)
                .map(|body| body.predict)
                .unwrap_or(false);
            if !predict {
                continue;
            }
            if entities.offset_local_transform(
                uid,
                step.linear_velocity * frame_time,
                keisan::Angle::new((step.angular_velocity * frame_time) as f64),
            ) {
                let current_map = entities
                    .inner
                    .transforms
                    .get(&uid)
                    .map(|transform| transform.map_id)
                    .unwrap_or(MapId::NULLSPACE);
                affected_maps.insert(previous_map);
                affected_maps.insert(current_map);
            }
            if step.awake_changed || previous_awake != step.awake {
                affected_maps.insert(previous_map);
            }
        }

        entities.sync_map_physics_runtime_many(affected_maps);
    }

    pub fn frame_update(&mut self, entities: &mut ClientEntityManager, frame_time: f32) {
        if self.prediction_enabled {
            self.update(entities, frame_time);
        }
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
        assert!(entities.inner.configure_physics_body(
            uid,
            Some(BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        entities
            .inner
            .mutate_physics_and_reconcile(uid, |body| body.linear_velocity = keisan::Vector2::new(2.0, 0.0));
        let _ = entities.inner.insert_fixture_and_reconcile(
            uid,
            Fixture::new("main", PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0))),
        );
        let broadphase_uid = entities.create_entity(None, EntityUid::new(50));
        entities.inner.ensure_broadphase(broadphase_uid);
        let mut system = PhysicsSystem::new();
        system.update(&mut entities, 0.5);
        entities.inner.refresh_broadphase_runtime(broadphase_uid, &[uid]);
        assert_eq!(entities.inner.transforms.get(&uid).unwrap().local_position.x, 0.9);
        assert!(entities.inner.entity_world_aabb(uid).is_some());
        assert_eq!(
            entities.inner.query_aabb_entities(broadphase_uid, Box2::new(-1.0, -2.0, 3.0, 2.0)),
            vec![uid]
        );
        assert_eq!(
            entities
                .inner
                .intersect_ray_on(
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
        assert!(entities.configure_predicted_physics(
            uid,
            Some(BodyType::Dynamic),
            Some(true),
            None,
            Some(true),
        ));
        entities
            .mutate_predicted_physics(uid, |body| body.linear_velocity = keisan::Vector2::new(2.0, 0.0));

        let mut system = PhysicsSystem::new();
        system.suppress_prediction_once(uid);
        system.update(&mut entities, 0.5);
        assert_eq!(entities.inner.transforms.get(&uid).unwrap().local_position, Vector2::ZERO);
        system.update(&mut entities, 0.5);
        assert_eq!(entities.inner.transforms.get(&uid).unwrap().local_position, Vector2::new(0.9, 0.0));
    }

    #[test]
    fn physics_system_keeps_predicted_spatial_runtime_in_sync() {
        let mut entities = ClientEntityManager::new();
        let map_owner = entities.ensure_map_entity(sekai::MapId::new(2));
        entities.inner.ensure_map_physics_runtime(sekai::MapId::new(2));

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
        assert!(entities.configure_predicted_physics(
            uid,
            Some(BodyType::Dynamic),
            Some(true),
            Some(true),
            Some(true),
        ));
        entities
            .mutate_predicted_physics(uid, |body| body.linear_velocity = keisan::Vector2::new(2.0, 0.0));
        let _ = entities.inner.insert_fixture_and_reconcile(
            uid,
            Fixture::new("main", PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0))),
        );

        let mut system = PhysicsSystem::new();
        system.update(&mut entities, 0.5);

        assert_eq!(
            entities.inner.query_aabb_entities(map_owner, Box2::new(0.25, -1.0, 2.0, 1.0)),
            vec![uid]
        );
    }

    #[test]
    fn physics_system_keeps_predicted_contact_runtime_in_sync() {
        let mut entities = ClientEntityManager::new();
        let _map_owner = entities.ensure_map_entity(sekai::MapId::new(3));
        entities.inner.ensure_map_physics_runtime(sekai::MapId::new(3));

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
            assert!(entities.configure_predicted_physics(
                uid,
                Some(BodyType::Dynamic),
                Some(true),
                Some(true),
                Some(true),
            ));
            let _ = entities.inner.insert_fixture_and_reconcile(
                uid,
                Fixture::new(
                    "main",
                    PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
                ),
            );
        }

        entities.sync_map_physics_runtime_many([sekai::MapId::new(3)]);

        assert_eq!(entities.inner.map_contact_count(sekai::MapId::new(3)), 1);
        let first_id = format!("{}:main", EntityUid::new(8).raw());
        let second_id = format!("{}:main", EntityUid::new(9).raw());
        let contacts = entities.inner.map_contacts_snapshot(sekai::MapId::new(3));
        assert!(contacts.iter().any(|contact| {
            (contact.fixture_a == first_id && contact.fixture_b == second_id)
                || (contact.fixture_a == second_id && contact.fixture_b == first_id)
        }));
        assert_eq!(contacts[0].manifold.points.len(), 1);
        assert_eq!(contacts[0].manifold.normal, Vector2::UNIT_X);
    }

    #[test]
    fn physics_system_keeps_predicted_mixed_contact_manifold_in_sync() {
        let mut entities = ClientEntityManager::new();
        let _map_owner = entities.ensure_map_entity(sekai::MapId::new(4));
        entities.inner.ensure_map_physics_runtime(sekai::MapId::new(4));

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
        assert!(entities.configure_predicted_physics(
            first,
            Some(BodyType::Dynamic),
            Some(true),
            Some(true),
            Some(true),
        ));
        let _ = entities.inner.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "aabb",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );

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
        assert!(entities.configure_predicted_physics(
            second,
            Some(BodyType::Dynamic),
            Some(true),
            Some(true),
            Some(true),
        ));
        let _ = entities.inner.insert_fixture_and_reconcile(
            second,
            Fixture::new("circle", PhysShape::Circle(CircleShape::new(Vector2::ZERO, 1.0))),
        );

        entities.sync_map_physics_runtime_many([sekai::MapId::new(4)]);

        assert_eq!(entities.inner.map_contact_count(sekai::MapId::new(4)), 1);
        let contacts = entities.inner.map_contacts_snapshot(sekai::MapId::new(4));
        assert_eq!(contacts[0].contact_type, butsuri::ContactType::Mixed);
        assert_eq!(contacts[0].manifold.points.len(), 1);
        assert!(contacts[0].manifold.normal.x.abs() > 0.9);
    }

    #[test]
    fn physics_system_intersect_ray_first_hit_returns_closest_entity() {
        let mut entities = ClientEntityManager::new();
        let broadphase_uid = entities.create_entity(None, EntityUid::new(60));
        entities.initialize_entity(broadphase_uid);
        entities.inner.ensure_broadphase(broadphase_uid);

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
        let _ = entities.inner.insert_fixture_and_reconcile(
            far,
            Fixture::new(
                "far",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );

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
        let _ = entities.inner.insert_fixture_and_reconcile(
            near,
            Fixture::new(
                "near",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );

        entities.inner.refresh_broadphase_runtime(broadphase_uid, &[far, near]);
        let hits = entities.inner.intersect_ray_on(
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
        let mut entities = ClientEntityManager::new();
        let broadphase_uid = entities.create_entity(None, EntityUid::new(63));
        entities.initialize_entity(broadphase_uid);
        entities.inner.ensure_broadphase(broadphase_uid);

        let uid = entities.create_entity(None, EntityUid::new(64));
        entities.initialize_entity(uid);
        let body = entities.inner.ensure_physics(uid);
        body.set_body_type(butsuri::BodyType::Dynamic);
        body.awake = true;
        body.can_collide = true;
        let _ = entities.inner.insert_fixture_and_reconcile(
            uid,
            Fixture::new("circle", PhysShape::Circle(CircleShape::new(Vector2::ZERO, 1.0))),
        );

        entities.inner.refresh_broadphase_runtime(broadphase_uid, &[uid]);
        let hits = entities.inner.intersect_ray_on(
            broadphase_uid,
            CollisionRay::new(Vector2::new(-2.0, 1.1), Vector2::UNIT_X, -1),
            10.0,
            false,
        );
        assert!(hits.is_empty());
    }

    #[test]
    fn physics_system_predicted_runtime_ignores_circle_aabb_false_contacts() {
        let mut entities = ClientEntityManager::new();
        let _map_owner = entities.ensure_map_entity(sekai::MapId::new(10));
        entities.inner.ensure_map_physics_runtime(sekai::MapId::new(10));

        let first = EntityUid::new(71);
        entities.create_entity(None, first);
        entities.initialize_entity(first);
        let _ = entities.inner.apply_transform_state(
            first,
            sekai::TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: keisan::Angle::ZERO,
                parent_id: sekai::EntityUid::INVALID,
                map_id: sekai::MapId::new(10),
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(first);
        body.set_body_type(butsuri::BodyType::Dynamic);
        body.awake = true;
        body.predict = true;
        body.can_collide = true;
        let _ = entities.inner.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "box",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );

        let second = EntityUid::new(72);
        entities.create_entity(None, second);
        entities.initialize_entity(second);
        let _ = entities.inner.apply_transform_state(
            second,
            sekai::TransformComponentState {
                local_position: Vector2::new(1.4, 1.4),
                rotation: keisan::Angle::ZERO,
                parent_id: sekai::EntityUid::INVALID,
                map_id: sekai::MapId::new(10),
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(second);
        body.set_body_type(butsuri::BodyType::Dynamic);
        body.awake = true;
        body.predict = true;
        body.can_collide = true;
        let _ = entities.inner.insert_fixture_and_reconcile(
            second,
            Fixture::new("circle", PhysShape::Circle(CircleShape::new(Vector2::ZERO, 0.5))),
        );

        entities.sync_map_physics_runtime_many([sekai::MapId::new(10)]);
        assert_eq!(entities.inner.map_contact_count(sekai::MapId::new(10)), 0);
    }

    #[test]
    fn physics_system_predicted_runtime_ignores_rotated_aabb_false_contacts() {
        let mut entities = ClientEntityManager::new();
        let _map_owner = entities.ensure_map_entity(sekai::MapId::new(11));
        entities.inner.ensure_map_physics_runtime(sekai::MapId::new(11));

        let first = EntityUid::new(73);
        entities.create_entity(None, first);
        entities.initialize_entity(first);
        let _ = entities.inner.apply_transform_state(
            first,
            sekai::TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: keisan::Angle::from_degrees(45.0),
                parent_id: sekai::EntityUid::INVALID,
                map_id: sekai::MapId::new(11),
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(first);
        body.set_body_type(butsuri::BodyType::Dynamic);
        body.awake = true;
        body.predict = true;
        body.can_collide = true;
        let _ = entities.inner.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "first",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -0.25, 1.0, 0.25), 0.0)),
            ),
        );

        let second = EntityUid::new(74);
        entities.create_entity(None, second);
        entities.initialize_entity(second);
        let _ = entities.inner.apply_transform_state(
            second,
            sekai::TransformComponentState {
                local_position: Vector2::new(-1.7, -1.7),
                rotation: keisan::Angle::from_degrees(45.0),
                parent_id: sekai::EntityUid::INVALID,
                map_id: sekai::MapId::new(11),
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(second);
        body.set_body_type(butsuri::BodyType::Dynamic);
        body.awake = true;
        body.predict = true;
        body.can_collide = true;
        let _ = entities.inner.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "second",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -0.25, 1.0, 0.25), 0.0)),
            ),
        );

        entities.sync_map_physics_runtime_many([sekai::MapId::new(11)]);
        assert_eq!(entities.inner.map_contact_count(sekai::MapId::new(11)), 0);
    }

    #[test]
    fn physics_system_update_applies_gravity_and_auto_clears_forces_for_predicted_bodies() {
        let mut entities = ClientEntityManager::new();
        let _map_owner = entities.ensure_map_entity(sekai::MapId::new(9));
        assert!(entities.inner.set_map_gravity(sekai::MapId::new(9), Vector2::new(0.0, -10.0)));
        assert!(entities.inner.set_map_auto_clear_forces(sekai::MapId::new(9), true));

        let uid = entities.create_entity(None, EntityUid::new(70));
        entities.initialize_entity(uid);
        let _ = entities.inner.apply_transform_state(
            uid,
            sekai::TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: keisan::Angle::ZERO,
                parent_id: sekai::EntityUid::INVALID,
                map_id: sekai::MapId::new(9),
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(uid);
        body.set_body_type(BodyType::Dynamic);
        body.awake = true;
        body.predict = true;
        body.can_collide = true;
        body.force = Vector2::new(2.0, 0.0);
        body.torque = 4.0;
        let _ = entities.inner.insert_fixture_and_reconcile(
            uid,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let mut system = PhysicsSystem::new();
        system.update(&mut entities, 0.5);

        let body = entities.inner.physics.get(&uid).unwrap();
        assert_eq!(body.linear_velocity, Vector2::new(0.9, -4.5));
        assert_eq!(body.angular_velocity, 1.8);
        assert_eq!(body.force, Vector2::ZERO);
        assert_eq!(body.torque, 0.0);
        let transform = entities.inner.transforms.get(&uid).unwrap();
        assert_eq!(transform.local_position, Vector2::new(0.45, -2.25));
        assert!((transform.local_rotation.theta - 0.9).abs() < 0.0001);
    }

    #[test]
    fn physics_system_update_puts_idle_predicted_bodies_to_sleep_and_updates_runtime() {
        let mut entities = ClientEntityManager::new();
        let map_owner = entities.ensure_map_entity(sekai::MapId::new(12));
        entities.inner.ensure_broadphase(map_owner);
        entities.inner.ensure_physics_map(map_owner);

        let uid = entities.create_entity(None, EntityUid::new(80));
        entities.initialize_entity(uid);
        let _ = entities.inner.apply_transform_state(
            uid,
            sekai::TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: keisan::Angle::ZERO,
                parent_id: sekai::EntityUid::INVALID,
                map_id: sekai::MapId::new(12),
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(uid);
        body.set_body_type(BodyType::Dynamic);
        body.awake = true;
        body.predict = true;
        body.can_collide = true;
        body.sleeping_allowed = true;
        let _ = entities.inner.insert_fixture_and_reconcile(
            uid,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );
        entities.sync_map_physics_runtime_many([sekai::MapId::new(12)]);
        assert!(entities.inner.map_contains_awake_body(sekai::MapId::new(12), uid));

        let mut system = PhysicsSystem::new();
        system.update(&mut entities, 0.6);

        let body = entities.inner.physics.get(&uid).unwrap();
        assert!(!body.awake);
        assert!(!entities.inner.map_contains_awake_body(sekai::MapId::new(12), uid));
    }

    #[test]
    fn physics_system_update_keeps_idle_predicted_bodies_awake_when_sleeping_is_disabled() {
        let mut entities = ClientEntityManager::new();
        let uid = entities.create_entity(None, EntityUid::new(81));
        entities.initialize_entity(uid);
        let body = entities.inner.ensure_physics(uid);
        body.set_body_type(BodyType::Dynamic);
        body.awake = true;
        body.predict = true;
        body.sleeping_allowed = false;

        let mut system = PhysicsSystem::new();
        system.update(&mut entities, 0.6);

        let body = entities.inner.physics.get(&uid).unwrap();
        assert!(body.awake);
        assert_eq!(body.sleep_time, 0.0);
    }
}
