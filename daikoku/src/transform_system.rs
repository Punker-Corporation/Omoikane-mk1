use crate::ServerEntityManager;
use keisan::{Angle, Vector2, Vector2i};
use sekai::{
    EntityCoordinates, EntityLookupSystem, EntityUid, GridId, MoveEvent, SharedTransformSystem,
};

#[derive(Debug, Default, Clone)]
pub struct TransformSystem {
    shared: SharedTransformSystem,
    lookup: EntityLookupSystem,
}

impl TransformSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn defer_move_event(&mut self, entities: &ServerEntityManager, move_event: MoveEvent) {
        self.shared.defer_move_event(&entities.inner, move_event);
    }

    pub fn process_deferred_moves(
        &mut self,
        entities: &mut ServerEntityManager,
        maps: &mut sekai::MapManager,
    ) -> Vec<MoveEvent> {
        let processed = self.shared.process_deferred_moves(&entities.inner);
        for move_event in &processed {
            if let Some(grid_component) = entities.inner.map_grid_components.get(&move_event.sender)
            {
                if let Some(transform) = entities.inner.transforms.get(&move_event.sender) {
                    maps.mark_grid_moved(transform.map_id, grid_component.grid_index);
                }
                let world_state = entities
                    .inner
                    .transforms
                    .get(&move_event.sender)
                    .map(|transform| transform.get_world_position_rotation_matrix(&entities.inner));
                if let Some(grid) = entities.inner.map_grids.get_mut(&move_event.sender)
                    && let Some((world_pos, world_rot, _)) = world_state
                {
                    grid.world_position = world_pos;
                    grid.world_rotation = world_rot;
                }
            }
            self.lookup
                .update_subtree_bounds(&mut entities.inner, move_event.sender);
        }
        processed
    }

    pub fn get_world_position(
        &self,
        entities: &ServerEntityManager,
        uid: EntityUid,
    ) -> Option<keisan::Vector2> {
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

    pub fn set_local_position(
        &mut self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        position: Vector2,
    ) -> bool {
        let tick = entities.inner.current_tick;
        let Some(transform) = entities.inner.transforms.get_mut(&uid) else {
            return false;
        };
        let Some(event) = transform.set_local_position(position) else {
            return false;
        };
        transform.base.last_modified_tick = tick;
        entities.inner.dirty_entity(uid);
        self.shared.defer_move_event(&entities.inner, event);
        true
    }

    pub fn set_local_rotation(
        &mut self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        rotation: Angle,
    ) -> bool {
        let tick = entities.inner.current_tick;
        let coordinates = {
            let Some(transform) = entities.inner.transforms.get_mut(&uid) else {
                return false;
            };
            let changed = transform.set_local_rotation(rotation).is_some();
            if !changed {
                return false;
            }
            let coordinates = transform.coordinates();
            transform.base.last_modified_tick = tick;
            coordinates
        };

        entities.inner.dirty_entity(uid);
        self.shared.defer_move_event(
            &entities.inner,
            MoveEvent {
                sender: uid,
                old_position: coordinates,
                new_position: coordinates,
            },
        );
        true
    }

    pub fn anchor_entity(
        &mut self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        grid_uid: EntityUid,
        grid_id: GridId,
        tile_indices: Vector2i,
    ) -> bool {
        let Some((map_id, tile_center)) = entities.inner.map_grids.get(&grid_uid).map(|grid| {
            (
                grid.parent_map_id,
                grid.grid_tile_to_local(tile_indices).position,
            )
        }) else {
            return false;
        };

        let tick = entities.inner.current_tick;
        let old = entities
            .inner
            .transforms
            .get(&uid)
            .map(|transform| transform.coordinates());

        if !entities.inner.set_parent(uid, grid_uid) {
            return false;
        }

        let new = {
            let Some(transform) = entities.inner.transforms.get_mut(&uid) else {
                return false;
            };
            transform.map_id = map_id;
            transform.grid_id = grid_id;
            transform.local_position = tile_center;
            transform.anchored = true;
            transform.rebuild_for_manager();
            transform.base.last_modified_tick = tick;
            transform.coordinates()
        };

        entities.inner.dirty_entity(uid);
        self.shared.defer_move_event(
            &entities.inner,
            MoveEvent {
                sender: uid,
                old_position: old.unwrap_or(new),
                new_position: new,
            },
        );
        true
    }

    pub fn unanchor_entity(&mut self, entities: &mut ServerEntityManager, uid: EntityUid) -> bool {
        let tick = entities.inner.current_tick;
        let Some(old) = entities
            .inner
            .transforms
            .get(&uid)
            .filter(|transform| transform.anchored)
            .map(|transform| transform.coordinates())
        else {
            return false;
        };

        let new = {
            let Some(transform) = entities.inner.transforms.get_mut(&uid) else {
                return false;
            };
            transform.anchored = false;
            transform.base.last_modified_tick = tick;
            transform.coordinates()
        };

        entities.inner.dirty_entity(uid);
        self.shared.defer_move_event(
            &entities.inner,
            MoveEvent {
                sender: uid,
                old_position: old,
                new_position: new,
            },
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::TransformSystem;
    use crate::ServerEntityManager;
    use keisan::Vector2i;
    use sekai::{GridId, MapId, MapManager, Tile, TileRenderFlag};

    #[test]
    fn transform_system_reads_world_position_for_entities() {
        let entities = ServerEntityManager::new();
        let mut entities = entities;
        let uid = entities.create_entity(None);
        let system = TransformSystem::new();
        assert_eq!(
            system.get_world_position(&entities, uid),
            Some(keisan::Vector2::ZERO)
        );
    }

    #[test]
    fn transform_system_moves_entities_and_marks_grids() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(2)));
        let grid_id = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(7)), 8);
        let grid_uid = maps.get_grid_euid(grid_id).unwrap();
        let uid = entities.create_entity(None);
        entities.initialize_entity(uid);
        entities.inner.transforms.get_mut(&uid).unwrap().parent = grid_uid;
        entities
            .inner
            .transforms
            .get_mut(&uid)
            .unwrap()
            .rebuild_for_manager();

        let mut system = TransformSystem::new();
        assert!(system.set_local_position(&mut entities, uid, keisan::Vector2::new(1.0, 0.0)));
        let processed = system.process_deferred_moves(&mut entities, &mut maps);
        assert_eq!(processed.len(), 1);

        assert!(system.set_local_position(&mut entities, grid_uid, keisan::Vector2::new(2.0, 0.0)));
        let processed = system.process_deferred_moves(&mut entities, &mut maps);
        assert_eq!(processed.len(), 1);
        assert!(maps.get_moved_grids(map_id).contains(&grid_id));
    }

    #[test]
    fn transform_system_can_anchor_and_unanchor_entities_to_grid_tiles() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(2)));
        let grid_id = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(7)), 8);
        let grid_uid = maps.get_grid_euid(grid_id).unwrap();
        entities
            .inner
            .map_grids
            .get_mut(&grid_uid)
            .unwrap()
            .set_tile(Vector2i::new(0, 0), Tile::new(1, TileRenderFlag(0), 0));

        let uid = entities.create_entity(None);
        entities.initialize_entity(uid);
        let mut system = TransformSystem::new();
        assert!(system.anchor_entity(&mut entities, uid, grid_uid, grid_id, Vector2i::new(0, 0)));
        let _ = system.process_deferred_moves(&mut entities, &mut maps);
        let transform = entities.inner.transforms.get(&uid).unwrap();
        assert!(transform.anchored);
        assert_eq!(transform.parent, grid_uid);
        assert_eq!(transform.local_position, keisan::Vector2::new(0.5, 0.5));
        assert_eq!(
            sekai::EntityLookupSystem.get_entities_intersecting(
                &entities.inner,
                grid_id,
                Vector2i::new(0, 0)
            ),
            vec![uid]
        );

        assert!(system.unanchor_entity(&mut entities, uid));
        let _ = system.process_deferred_moves(&mut entities, &mut maps);
        assert!(!entities.inner.transforms.get(&uid).unwrap().anchored);
    }
}
