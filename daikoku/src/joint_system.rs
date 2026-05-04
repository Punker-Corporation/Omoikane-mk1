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

        let id = joint.id.clone();
        let inserted_a = entities
            .inner
            .ensure_joints(body_a)
            .add_joint(joint.clone());
        let inserted_b = entities.inner.ensure_joints(body_b).add_joint(joint);

        if inserted_a && inserted_b {
            let tick = entities.inner.current_tick;
            let physics = crate::PhysicsSystem::new();
            let _ = physics.set_awake(entities, body_a, true);
            let _ = physics.set_awake(entities, body_b, true);
            if let Some(component) = entities.inner.joint_components.get_mut(&body_a) {
                component.base.last_modified_tick = tick;
            }
            if let Some(component) = entities.inner.joint_components.get_mut(&body_b) {
                component.base.last_modified_tick = tick;
            }
            entities.inner.dirty_entity(body_a);
            entities.inner.dirty_entity(body_b);
            return true;
        }

        let _ = entities.inner.ensure_joints(body_a).remove_joint(&id);
        let _ = entities.inner.ensure_joints(body_b).remove_joint(&id);
        false
    }

    pub fn remove_joint(
        &self,
        entities: &mut ServerEntityManager,
        body_a: EntityUid,
        body_b: EntityUid,
        id: &str,
    ) -> bool {
        let removed_a = entities
            .inner
            .joint_components
            .get_mut(&body_a)
            .and_then(|component| component.remove_joint(id))
            .is_some();
        let removed_b = entities
            .inner
            .joint_components
            .get_mut(&body_b)
            .and_then(|component| component.remove_joint(id))
            .is_some();
        let tick = entities.inner.current_tick;
        if removed_a {
            if let Some(component) = entities.inner.joint_components.get_mut(&body_a) {
                component.base.last_modified_tick = tick;
            }
            entities.inner.dirty_entity(body_a);
        }
        if removed_b {
            if let Some(component) = entities.inner.joint_components.get_mut(&body_b) {
                component.base.last_modified_tick = tick;
            }
            entities.inner.dirty_entity(body_b);
        }
        removed_a || removed_b
    }
}

#[cfg(test)]
mod tests {
    use super::JointSystem;
    use crate::ServerEntityManager;
    use butsuri::{BodyType, Joint, JointType};

    #[test]
    fn joint_system_adds_and_removes_joint_components() {
        let mut entities = ServerEntityManager::new();
        let a = entities.create_entity(None);
        let b = entities.create_entity(None);
        entities
            .inner
            .ensure_physics(a)
            .set_body_type(BodyType::Dynamic);
        entities
            .inner
            .ensure_physics(b)
            .set_body_type(BodyType::Dynamic);
        let mut joint = Joint::new(a.raw(), b.raw(), JointType::Distance);
        joint.id = "ab".to_string();
        let system = JointSystem::new();
        assert!(system.add_joint(&mut entities, joint));
        assert_eq!(entities.inner.ensure_joints(a).joint_count(), 1);
        assert!(system.remove_joint(&mut entities, a, b, "ab"));
    }
}
