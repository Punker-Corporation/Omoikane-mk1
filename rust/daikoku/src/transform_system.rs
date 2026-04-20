use crate::ServerEntityManager;
use sekai::{EntityCoordinates, EntityUid, MoveEvent, SharedTransformSystem};

#[derive(Debug, Default, Clone)]
pub struct TransformSystem {
    shared: SharedTransformSystem,
}

impl TransformSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn defer_move_event(&mut self, entities: &ServerEntityManager, move_event: MoveEvent) {
        self.shared.defer_move_event(&entities.inner, move_event);
    }

    pub fn process_deferred_moves(&mut self, entities: &ServerEntityManager) -> Vec<MoveEvent> {
        self.shared.process_deferred_moves(&entities.inner)
    }

    pub fn get_world_position(&self, entities: &ServerEntityManager, uid: EntityUid) -> Option<keisan::Vector2> {
        self.shared.get_world_position(&entities.inner, uid)
    }

    pub fn get_mover_coordinates(
        &self,
        entities: &ServerEntityManager,
        uid: EntityUid,
    ) -> Option<EntityCoordinates> {
        let xform = entities.inner.transforms.get(&uid)?;
        Some(self.shared.get_mover_coordinates(&entities.inner, xform))
    }
}

#[cfg(test)]
mod tests {
    use super::TransformSystem;
    use crate::ServerEntityManager;

    #[test]
    fn transform_system_reads_world_position_for_entities() {
        let entities = ServerEntityManager::new();
        let mut entities = entities;
        let uid = entities.create_entity(None);
        let system = TransformSystem::new();
        assert_eq!(system.get_world_position(&entities, uid), Some(keisan::Vector2::ZERO));
    }
}
