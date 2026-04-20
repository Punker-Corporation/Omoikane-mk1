use crate::ServerEntityManager;
use sekai::{BodyType, EntityUid, SharedPhysicsSystem};

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
        let body = entities.inner.ensure_physics(uid);
        body.can_collide = true;
        body.set_body_type(BodyType::Static);
        true
    }

    pub fn update(&self, entities: &mut ServerEntityManager, broadphase_owner: EntityUid) -> usize {
        let bodies: Vec<_> = entities.inner.physics.keys().copied().collect();
        self.shared
            .sync_broadphase(&mut entities.inner, broadphase_owner, &bodies)
    }
}

#[cfg(test)]
mod tests {
    use super::PhysicsSystem;
    use crate::ServerEntityManager;
    use sekai::BroadphaseComponent;

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
        assert_eq!(system.update(&mut entities, broadphase_uid), 0);
    }
}
