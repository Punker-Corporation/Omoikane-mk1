use crate::ServerEntityManager;
use butsuri::Joint;
use sekai::EntityUid;

#[derive(Debug, Default, Clone)]
pub struct JointSystem;

impl JointSystem {
    pub fn new() -> Self {
        Self
    }

    pub fn add_joint(&self, entities: &mut ServerEntityManager, joint: Joint) -> bool {
        let body_a = EntityUid::new(joint.body_a_uid);
        let body_b = EntityUid::new(joint.body_b_uid);

        if body_a == body_b {
            return false;
        }

        if entities.inner.add_joint_between(joint) {
            let physics = crate::PhysicsSystem::new();
            let _ = physics.set_awake(entities, body_a, true);
            let _ = physics.set_awake(entities, body_b, true);
            return true;
        }
        false
    }

    pub fn remove_joint(&self, entities: &mut ServerEntityManager, body_a: EntityUid, body_b: EntityUid, id: &str) -> bool {
        entities.inner.remove_joint_between(body_a, body_b, id)
    }
}

#[cfg(test)]
mod tests {
    use super::JointSystem;
    use crate::ServerEntityManager;
    use butsuri::{AabbShape, BodyType, Fixture, Joint, JointType, PhysShape};
    use keisan::{Box2, Vector2};
    use sekai::MapId;

    #[test]
    fn joint_system_adds_and_removes_joint_components() {
        let mut entities = ServerEntityManager::new();
        entities.inner.create_entity_uninitialized_as_map(None, sekai::MapId::new(99));
        let a = entities.create_entity(None);
        let b = entities.create_entity(None);
        let _ = entities.inner.apply_transform_state(
            a,
            sekai::TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: keisan::Angle::ZERO,
                parent_id: sekai::EntityUid::INVALID,
                map_id: sekai::MapId::new(99),
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let _ = entities.inner.apply_transform_state(
            b,
            sekai::TransformComponentState {
                local_position: Vector2::new(1.0, 0.0),
                rotation: keisan::Angle::ZERO,
                parent_id: sekai::EntityUid::INVALID,
                map_id: sekai::MapId::new(99),
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        entities.inner.ensure_physics(a).set_body_type(BodyType::Dynamic);
        entities.inner.ensure_physics(b).set_body_type(BodyType::Dynamic);
        let mut joint = Joint::new(a.raw(), b.raw(), JointType::Distance);
        joint.id = "ab".to_string();
        let system = JointSystem::new();
        assert!(system.add_joint(&mut entities, joint));
        assert_eq!(entities.inner.ensure_joints(a).joint_count(), 1);
        assert_eq!(
            entities.inner.drain_map_runtime_events(sekai::MapId::new(99)),
            vec![sekai::PhysicsRuntimeEvent::JointAdded(sekai::JointAddedEvent {
                body_a: a,
                body_b: b,
                joint_id: "ab".to_string(),
            })]
        );
        assert!(system.remove_joint(&mut entities, a, b, "ab"));
        assert_eq!(
            entities.inner.drain_map_runtime_events(sekai::MapId::new(99)),
            vec![sekai::PhysicsRuntimeEvent::JointRemoved(sekai::JointRemovedEvent {
                body_a: a,
                body_b: b,
                joint_id: "ab".to_string(),
            })]
        );
    }

    #[test]
    fn joint_system_refreshes_contacts_immediately_when_joint_changes_collision_filter() {
        let mut entities = ServerEntityManager::new();
        let map = entities.create_entity(None);
        entities.initialize_entity(map);
        entities.inner.ensure_map(MapId::new(1), map);
        entities.inner.transforms.get_mut(&map).unwrap().map_id = MapId::new(1);
        entities.inner.ensure_broadphase(map);
        entities.inner.ensure_physics_map(map);

        let a = entities.create_entity(None);
        entities.initialize_entity(a);
        let _ = entities.inner.apply_transform_state(
            a,
            sekai::TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: keisan::Angle::ZERO,
                parent_id: sekai::EntityUid::INVALID,
                map_id: MapId::new(1),
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(a);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        body.awake = true;
        let _ = entities.inner.insert_fixture_and_reconcile(
            a,
            Fixture::new(
                "a",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let b = entities.create_entity(None);
        entities.initialize_entity(b);
        let _ = entities.inner.apply_transform_state(
            b,
            sekai::TransformComponentState {
                local_position: Vector2::new(0.5, 0.0),
                rotation: keisan::Angle::ZERO,
                parent_id: sekai::EntityUid::INVALID,
                map_id: MapId::new(1),
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(b);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        body.awake = true;
        let _ = entities.inner.insert_fixture_and_reconcile(
            b,
            Fixture::new(
                "b",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        entities.inner.refresh_map_physics_runtime(MapId::new(1));
        assert_eq!(entities.inner.map_contact_count(MapId::new(1)), 1);
        let mut joint = Joint::new(a.raw(), b.raw(), JointType::Distance);
        joint.id = "ab".to_string();
        joint.collide_connected = false;
        let system = JointSystem::new();
        let _ = entities.inner.drain_map_contact_events(MapId::new(1));
        assert!(system.add_joint(&mut entities, joint));
        assert_eq!(entities.inner.map_contact_count(MapId::new(1)), 0);
        let events = entities.inner.drain_map_contact_events(MapId::new(1));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, butsuri::ContactStatus::EndTouching);

        assert!(system.remove_joint(&mut entities, a, b, "ab"));
        assert_eq!(entities.inner.map_contact_count(MapId::new(1)), 1);
        let events = entities.inner.drain_map_contact_events(MapId::new(1));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, butsuri::ContactStatus::StartTouching);
    }
}
