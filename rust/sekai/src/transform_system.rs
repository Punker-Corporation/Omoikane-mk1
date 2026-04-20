use crate::{map_grid::MapGridLike, EntityCoordinates, EntityManager, EntityUid, MapId, MoveEvent, TransformComponent, TransformResolver};
use keisan::{Angle, Matrix3, Vector2};
use std::collections::VecDeque;

#[derive(Debug, Default, Clone)]
pub struct SharedTransformSystem {
    grid_moves: VecDeque<MoveEvent>,
    other_moves: VecDeque<MoveEvent>,
    pub updates_outside_prediction: bool,
}

impl SharedTransformSystem {
    pub fn new() -> Self {
        Self {
            grid_moves: VecDeque::new(),
            other_moves: VecDeque::new(),
            updates_outside_prediction: true,
        }
    }

    pub fn defer_move_event(&mut self, manager: &EntityManager, move_event: MoveEvent) {
        if manager.map_grids.contains_key(&move_event.sender) {
            self.grid_moves.push_back(move_event);
        } else {
            self.other_moves.push_back(move_event);
        }
    }

    pub fn process_deferred_moves(&mut self, manager: &EntityManager) -> Vec<MoveEvent> {
        let mut processed = Vec::new();
        Self::process_queue(&mut self.grid_moves, manager, &mut processed);
        Self::process_queue(&mut self.other_moves, manager, &mut processed);
        processed
    }

    pub fn get_world_matrix(&self, manager: &EntityManager, uid: EntityUid) -> Option<Matrix3> {
        manager.world_transform(uid).map(|x| x.world_matrix)
    }

    pub fn get_world_position(&self, manager: &EntityManager, uid: EntityUid) -> Option<Vector2> {
        manager.world_transform(uid).map(|x| x.world_position)
    }

    pub fn get_world_rotation(&self, manager: &EntityManager, uid: EntityUid) -> Option<Angle> {
        manager.world_transform(uid).map(|x| x.world_rotation)
    }

    pub fn get_inv_world_matrix(&self, manager: &EntityManager, uid: EntityUid) -> Option<Matrix3> {
        manager.world_transform(uid).map(|x| x.inv_world_matrix)
    }

    pub fn get_map_id(&self, manager: &EntityManager, uid: Option<EntityUid>) -> MapId {
        uid.and_then(|entity| manager.transforms.get(&entity).map(|x| x.map_id))
            .unwrap_or(MapId::NULLSPACE)
    }

    pub fn get_mover_coordinates(&self, manager: &EntityManager, xform: &TransformComponent) -> EntityCoordinates {
        if let Some(grid) = manager.map_grids.values().find(|grid| grid.index == xform.grid_id) {
            if grid.grid_entity_id == xform.parent {
                return xform.coordinates();
            }

            let world = xform.map_position(manager);
            let grid_pos = grid.inv_world_matrix() * world.position;
            return EntityCoordinates::new(grid.grid_entity_id, grid_pos);
        }

        let map_uid = manager
            .metadata
            .iter()
            .find_map(|(uid, meta)| (meta.map_id == xform.map_id).then_some(*uid))
            .unwrap_or(xform.parent);

        if xform.parent == map_uid {
            return xform.coordinates();
        }

        EntityCoordinates::new(map_uid, xform.map_position(manager).position)
    }

    fn process_queue(queue: &mut VecDeque<MoveEvent>, manager: &EntityManager, processed: &mut Vec<MoveEvent>) {
        while let Some(event) = queue.pop_front() {
            if manager.deleted(event.sender) || !event.new_position.is_valid(manager) {
                continue;
            }
            processed.push(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SharedTransformSystem;
    use crate::{EntityManager, EntityUid};
    use keisan::{Matrix3, Vector2};

    #[test]
    fn transform_system_reads_world_transforms_from_manager() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.transforms.get_mut(&uid).unwrap().base.owner = uid;
        let system = SharedTransformSystem::new();
        let matrix = system.get_world_matrix(&manager, uid).unwrap();
        assert_eq!(matrix, Matrix3::IDENTITY);
        assert_eq!(system.get_world_position(&manager, uid).unwrap(), Vector2::ZERO);
        assert_eq!(system.get_map_id(&manager, Some(EntityUid::INVALID)), crate::MapId::NULLSPACE);
    }
}
