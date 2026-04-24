use crate::{
    ChunkDatum, EntityManager, EntityUid, GameStateMapData, GridDatum, GridId, MapCoordinates,
    MapGrid, MapId,
};
use keisan::{Box2, Vector2};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapEventArgs {
    pub map: MapId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GridChangedEventArgs {
    pub grid_id: GridId,
    pub modified: Vec<(keisan::Vector2i, crate::Tile)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileChangedEventArgs {
    pub new_tile: crate::TileRef,
    pub old_tile: crate::Tile,
}

#[derive(Debug, Default, Clone)]
pub struct MapManager {
    pub suppress_on_tile_changed: bool,
    map_entities: HashMap<MapId, EntityUid>,
    grids: HashMap<GridId, EntityUid>,
    moved_grids: HashMap<MapId, HashSet<GridId>>,
    changed_chunks: HashMap<GridId, Vec<(jikan::GameTick, keisan::Vector2i, bool)>>,
    deleted_grids: HashMap<MapId, Vec<(jikan::GameTick, GridId)>>,
    highest_map_id: MapId,
    highest_grid_id: GridId,
    current_tick: jikan::GameTick,
}

impl MapManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn initialize(&mut self) {}

    pub fn set_current_tick(&mut self, current_tick: jikan::GameTick) {
        self.current_tick = current_tick;
    }

    pub fn startup(&mut self, manager: &mut EntityManager) {
        self.ensure_nullspace_exists_and_clear(manager);
    }

    pub fn shutdown(&mut self, manager: &mut EntityManager) {
        let maps: Vec<_> = self.map_entities.keys().copied().collect();
        for map_id in maps {
            if map_id != MapId::NULLSPACE {
                self.delete_map(manager, map_id);
            }
        }
        self.ensure_nullspace_exists_and_clear(manager);
        self.deleted_grids.clear();
    }

    pub fn restart(&mut self, manager: &mut EntityManager) {
        self.shutdown(manager);
    }

    pub fn create_map(&mut self, manager: &mut EntityManager, map_id: Option<MapId>) -> MapId {
        let actual_id = map_id.unwrap_or_else(|| self.next_map_id());
        assert!(!self.map_exists(actual_id), "map already exists: {actual_id}");

        if actual_id.raw() > self.highest_map_id.raw() {
            self.highest_map_id = actual_id;
        }

        if actual_id == MapId::NULLSPACE {
            self.map_entities.insert(actual_id, EntityUid::INVALID);
            return actual_id;
        }

        let new_entity = manager.create_entity_uninitialized_as_map(None, actual_id);
        self.map_entities.insert(actual_id, new_entity);
        actual_id
    }

    pub fn create_uninitialized_map(&mut self, manager: &mut EntityManager, map_id: Option<MapId>) -> MapId {
        let actual_id = self.create_map(manager, map_id);
        if actual_id != MapId::NULLSPACE {
            let _ = self.add_uninitialized_map(manager, actual_id);
        }
        actual_id
    }

    pub fn add_uninitialized_map(&mut self, manager: &mut EntityManager, map_id: MapId) -> bool {
        let map_uid = self.get_map_entity_id(map_id);
        if !map_uid.is_valid() {
            return false;
        }
        manager.set_map_pre_init(map_uid, true)
    }

    pub fn do_map_initialize(&mut self, manager: &mut EntityManager, map_id: MapId) -> bool {
        let map_uid = self.get_map_entity_id(map_id);
        if !map_uid.is_valid() {
            return false;
        }
        manager.set_map_pre_init(map_uid, false)
    }

    pub fn is_map_initialized(&self, manager: &EntityManager, map_id: MapId) -> bool {
        manager.is_map_initialized(map_id)
    }

    pub fn is_map_paused(&self, manager: &EntityManager, map_id: MapId) -> bool {
        manager.is_map_paused(map_id)
    }

    pub fn is_grid_paused(&self, manager: &EntityManager, grid_id: GridId) -> bool {
        self.get_grid(manager, grid_id)
            .map(|grid| self.is_map_paused(manager, grid.parent_map_id))
            .unwrap_or(true)
    }

    pub fn map_exists(&self, map_id: MapId) -> bool {
        self.map_entities.contains_key(&map_id)
    }

    pub fn create_new_map_entity(&mut self, manager: &mut EntityManager, map_id: MapId) -> EntityUid {
        let entity = manager.create_entity_uninitialized_as_map(None, map_id);
        self.set_map_entity(manager, map_id, entity);
        entity
    }

    pub fn set_map_entity(&mut self, manager: &mut EntityManager, map_id: MapId, new_map_entity_id: EntityUid) {
        assert!(self.map_exists(map_id), "map does not exist: {map_id}");

        let previous = self.map_entities.insert(map_id, new_map_entity_id);
        let map_grids = self
            .grids
            .values()
            .filter_map(|uid| manager.map_grids.get(uid).filter(|grid| grid.parent_map_id == map_id).map(|_| *uid))
            .collect::<Vec<_>>();

        let _ = manager.materialize_map_entity(new_map_entity_id, map_id);

        for grid_uid in map_grids {
            let current = manager.transforms.get(&grid_uid).cloned();
            if let Some(current) = current {
                let _ = manager.apply_transform_state(
                    grid_uid,
                    crate::TransformComponentState {
                        local_position: current.local_position,
                        rotation: current.local_rotation,
                        parent_id: new_map_entity_id,
                        map_id: current.map_id,
                        grid_id: current.grid_id,
                        no_local_rotation: current.no_local_rotation,
                        anchored: current.anchored,
                    },
                );
            }
        }

        if let Some(old) = previous {
            if old.is_valid() && old != new_map_entity_id {
                manager.queue_delete_entity(old);
            }
        }

    }

    pub fn get_map_entity_id(&self, map_id: MapId) -> EntityUid {
        self.map_entities.get(&map_id).copied().unwrap_or(EntityUid::INVALID)
    }

    pub fn get_map_entity_id_or_throw(&self, map_id: MapId) -> EntityUid {
        *self.map_entities.get(&map_id).expect("map does not exist")
    }

    pub fn get_all_map_ids(&self) -> Vec<MapId> {
        self.map_entities.keys().copied().collect()
    }

    pub fn delete_map(&mut self, manager: &mut EntityManager, map_id: MapId) {
        if let Some(entity) = self.map_entities.remove(&map_id) {
            let grids: Vec<_> = self
                .grids
                .iter()
                .filter_map(|(grid_id, uid)| {
                    manager
                        .map_grids
                        .get(uid)
                        .filter(|grid| grid.parent_map_id == map_id)
                        .map(|_| *grid_id)
                })
                .collect();

            for grid_id in grids {
                self.delete_grid(manager, grid_id);
            }

            if entity.is_valid() {
                manager.queue_delete_entity(entity);
                manager.flush_queued_deletions();
            }
        }
    }

    pub fn create_grid(
        &mut self,
        manager: &mut EntityManager,
        current_map_id: MapId,
        forced_grid_id: Option<GridId>,
        chunk_size: u16,
    ) -> GridId {
        assert!(self.map_exists(current_map_id), "map does not exist: {current_map_id}");

        let actual_id = forced_grid_id.unwrap_or_else(|| self.next_grid_id());
        assert!(!self.grid_exists(actual_id), "grid already exists: {actual_id}");

        if actual_id.raw() > self.highest_grid_id.raw() {
            self.highest_grid_id = actual_id;
        }

        let grid_entity = manager.create_entity_uninitialized_as_grid(
            None,
            current_map_id,
            self.get_map_entity_id(current_map_id),
            actual_id,
            chunk_size,
            Vector2::ZERO,
            keisan::Angle::ZERO,
        );
        self.grids.insert(actual_id, grid_entity);
        actual_id
    }

    pub fn get_grid<'a>(&self, manager: &'a EntityManager, grid_id: GridId) -> Option<&'a MapGrid> {
        let uid = self.grids.get(&grid_id)?;
        manager.map_grids.get(uid)
    }

    pub fn try_get_grid<'a>(&self, manager: &'a EntityManager, grid_id: GridId) -> Option<&'a MapGrid> {
        self.get_grid(manager, grid_id)
    }

    pub fn try_get_grid_by_entity<'a>(&self, manager: &'a EntityManager, entity: EntityUid) -> Option<&'a MapGrid> {
        manager.map_grids.get(&entity)
    }

    pub fn grid_exists(&self, grid_id: GridId) -> bool {
        grid_id != GridId::INVALID && self.grids.contains_key(&grid_id)
    }

    pub fn grid_exists_entity(&self, manager: &EntityManager, entity: EntityUid) -> bool {
        manager.map_grids.contains_key(&entity)
    }

    pub fn get_all_map_grids<'a>(&self, manager: &'a EntityManager, map_id: MapId) -> Vec<&'a MapGrid> {
        self.grids
            .values()
            .filter_map(|uid| manager.map_grids.get(uid))
            .filter(|grid| grid.parent_map_id == map_id)
            .collect()
    }

    pub fn try_find_grid_at<'a>(&self, manager: &'a EntityManager, map_id: MapId, world_pos: Vector2) -> Option<&'a MapGrid> {
        let point = Box2::centered_around(world_pos, Vector2::ONE);
        self.find_grids_intersecting(manager, map_id, point, true)
            .into_iter()
            .find(|grid| grid.collides_with_grid(grid.world_to_tile(world_pos)))
    }

    pub fn try_find_grid_at_coordinates<'a>(
        &self,
        manager: &'a EntityManager,
        map_coordinates: MapCoordinates,
    ) -> Option<&'a MapGrid> {
        self.try_find_grid_at(manager, map_coordinates.map_id, map_coordinates.position)
    }

    pub fn find_grids_intersecting<'a>(
        &self,
        manager: &'a EntityManager,
        map_id: MapId,
        world_aabb: Box2,
        approx: bool,
    ) -> Vec<&'a MapGrid> {
        let mut grids = Vec::new();

        for uid in self.grids.values() {
            let Some(grid) = manager.map_grids.get(uid) else {
                continue;
            };
            if grid.parent_map_id != map_id {
                continue;
            }

            if !grid.world_bounds().intersects(world_aabb) {
                continue;
            }

            if approx {
                grids.push(grid);
                continue;
            }

            let chunk_hit = grid.get_map_chunks_intersecting(world_aabb).next().is_some();
            if chunk_hit || (grid.chunk_count() == 0 && world_aabb.contains(grid.world_position, true)) {
                grids.push(grid);
            }
        }

        grids
    }

    pub fn delete_grid(&mut self, manager: &mut EntityManager, grid_id: GridId) {
        let Some(entity) = self.grids.remove(&grid_id) else {
            return;
        };
        let map_id = manager
            .map_grids
            .get(&entity)
            .map(|grid| grid.parent_map_id)
            .unwrap_or(MapId::NULLSPACE);
        manager.map_grids.remove(&entity);
        manager.map_grid_components.remove(&entity);
        self.changed_chunks.remove(&grid_id);
        self.note_grid_deleted(map_id, grid_id);
        manager.queue_delete_entity(entity);
        manager.flush_queued_deletions();
    }

    pub fn get_moved_grids(&self, map_id: MapId) -> HashSet<GridId> {
        self.moved_grids.get(&map_id).cloned().unwrap_or_default()
    }

    pub fn mark_grid_moved(&mut self, map_id: MapId, grid_id: GridId) {
        self.moved_grids.entry(map_id).or_default().insert(grid_id);
    }

    pub fn clear_moved_grids(&mut self, map_id: MapId) {
        self.moved_grids.remove(&map_id);
    }

    pub fn set_tile(&mut self, manager: &mut EntityManager, grid_id: GridId, indices: keisan::Vector2i, tile: crate::Tile) -> bool {
        let Some(grid_uid) = self.grids.get(&grid_id).copied() else {
            return false;
        };
        let Some(grid) = manager.map_grids.get_mut(&grid_uid) else {
            return false;
        };
        if !grid.set_tile(indices, tile) {
            return false;
        }
        self.note_chunk_changed(grid_id, grid.grid_tile_to_chunk_indices(indices), false);
        true
    }

    pub fn set_tiles(
        &mut self,
        manager: &mut EntityManager,
        grid_id: GridId,
        tiles: &[(keisan::Vector2i, crate::Tile)],
    ) -> Vec<keisan::Vector2i> {
        let Some(grid_uid) = self.grids.get(&grid_id).copied() else {
            return Vec::new();
        };
        let Some(grid) = manager.map_grids.get_mut(&grid_uid) else {
            return Vec::new();
        };
        let changed = grid.set_tiles(tiles);
        for chunk_index in &changed {
            self.note_chunk_changed(grid_id, *chunk_index, false);
        }
        changed
    }

    pub fn remove_chunk(&mut self, manager: &mut EntityManager, grid_id: GridId, chunk_index: keisan::Vector2i) -> bool {
        let Some(grid_uid) = self.grids.get(&grid_id).copied() else {
            return false;
        };
        let Some(grid) = manager.map_grids.get_mut(&grid_uid) else {
            return false;
        };
        if !grid.remove_chunk(chunk_index) {
            return false;
        }
        self.note_chunk_changed(grid_id, chunk_index, true);
        true
    }

    pub fn get_changed_chunks_since(
        &self,
        grid_id: GridId,
        from_tick: jikan::GameTick,
    ) -> Vec<(keisan::Vector2i, bool)> {
        let mut latest = HashMap::new();
        if let Some(history) = self.changed_chunks.get(&grid_id) {
            for (tick, chunk_index, deleted) in history {
                if *tick > from_tick {
                    latest.insert(*chunk_index, *deleted);
                }
            }
        }
        let mut chunks = latest.into_iter().collect::<Vec<_>>();
        chunks.sort_by(|a, b| a.0.x.cmp(&b.0.x).then(a.0.y.cmp(&b.0.y)));
        chunks
    }

    pub fn get_deleted_grids_since(&self, map_id: MapId, from_tick: jikan::GameTick) -> Vec<GridId> {
        self.deleted_grids
            .get(&map_id)
            .map(|history| {
                history
                    .iter()
                    .filter_map(|(tick, grid_id)| (*tick > from_tick).then_some(*grid_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn collect_game_state_map_data(
        &self,
        manager: &EntityManager,
        visible: &[EntityUid],
        newly_visible: &[EntityUid],
        from_tick: jikan::GameTick,
    ) -> Option<GameStateMapData> {
        let mut grid_data = HashMap::new();
        let mut deleted_grids = Vec::new();
        let all_grids = from_tick == jikan::GameTick::ZERO;
        let visible_grids = visible
            .iter()
            .filter_map(|uid| {
                let grid_id = manager.grid_id_for(*uid);
                grid_id.is_valid().then_some(grid_id)
            })
            .collect::<HashSet<_>>();
        let newly_visible_grids = newly_visible
            .iter()
            .filter_map(|uid| {
                let grid_id = manager.grid_id_for(*uid);
                grid_id.is_valid().then_some(grid_id)
            })
            .collect::<HashSet<_>>();
        let moved = self
            .get_all_map_ids()
            .into_iter()
            .flat_map(|map_id| self.get_moved_grids(map_id).into_iter())
            .collect::<HashSet<_>>();

        for grid in manager.map_grids.values() {
            if all_grids {
                if !visible_grids.contains(&grid.index) {
                    continue;
                }
            } else {
                let moved_match = moved.contains(&grid.index) && visible_grids.contains(&grid.index);
                let newly_visible_match = newly_visible_grids.contains(&grid.index);
                let changed_chunks = self.get_changed_chunks_since(grid.index, from_tick);
                let chunk_changed_match = !changed_chunks.is_empty() && visible_grids.contains(&grid.index);
                if !moved_match && !newly_visible_match && !chunk_changed_match {
                    continue;
                }
            }

            let mut chunk_data = Vec::new();
            if all_grids || newly_visible_grids.contains(&grid.index) {
                for chunk_index in grid.chunk_indices() {
                    let Some(chunk) = grid.try_get_chunk(chunk_index) else {
                        continue;
                    };
                    let mut tile_data = Vec::with_capacity((grid.chunk_size as usize).pow(2));
                    for x in 0..grid.chunk_size {
                        for y in 0..grid.chunk_size {
                            tile_data.push(chunk.get_tile(x, y));
                        }
                    }
                    chunk_data.push(ChunkDatum::create_modified(chunk_index, tile_data));
                }
            } else {
                for (chunk_index, deleted) in self.get_changed_chunks_since(grid.index, from_tick) {
                    if deleted {
                        chunk_data.push(ChunkDatum::create_deleted(chunk_index));
                        continue;
                    }
                    let Some(chunk) = grid.try_get_chunk(chunk_index) else {
                        continue;
                    };
                    let mut tile_data = Vec::with_capacity((grid.chunk_size as usize).pow(2));
                    for x in 0..grid.chunk_size {
                        for y in 0..grid.chunk_size {
                            tile_data.push(chunk.get_tile(x, y));
                        }
                    }
                    chunk_data.push(ChunkDatum::create_modified(chunk_index, tile_data));
                }
            }

            grid_data.insert(
                grid.index,
                GridDatum {
                    coordinates: MapCoordinates::new(grid.world_position, grid.parent_map_id),
                    angle: grid.world_rotation,
                    chunk_data,
                },
            );
        }

        for map_id in self.get_all_map_ids() {
            deleted_grids.extend(self.get_deleted_grids_since(map_id, from_tick));
        }

        if grid_data.is_empty() && deleted_grids.is_empty() {
            None
        } else {
            Some(GameStateMapData {
                grid_data,
                deleted_grids,
            })
        }
    }

    pub fn cull_deleted_grid_history(&mut self, oldest_ack: jikan::GameTick) {
        self.deleted_grids.retain(|_, history| {
            history.retain(|(tick, _)| *tick > oldest_ack);
            !history.is_empty()
        });
        self.changed_chunks.retain(|_, history| {
            history.retain(|(tick, _, _)| *tick > oldest_ack);
            !history.is_empty()
        });
    }

    pub fn has_map_entity(&self, map_id: MapId) -> bool {
        self.map_entities.contains_key(&map_id)
    }

    pub fn is_grid(&self, manager: &EntityManager, uid: EntityUid) -> bool {
        manager.map_grids.contains_key(&uid)
    }

    pub fn is_map(&self, manager: &EntityManager, uid: EntityUid) -> bool {
        manager.map_components.contains_key(&uid)
    }

    pub fn next_map_id(&mut self) -> MapId {
        self.highest_map_id = MapId::new(self.highest_map_id.raw() + 1);
        self.highest_map_id
    }

    pub fn next_grid_id(&mut self) -> GridId {
        self.highest_grid_id = GridId::new(self.highest_grid_id.raw() + 1);
        self.highest_grid_id
    }

    pub fn get_grid_euid(&self, grid_id: GridId) -> Option<EntityUid> {
        self.grids.get(&grid_id).copied()
    }

    fn note_grid_deleted(&mut self, map_id: MapId, grid_id: GridId) {
        self.deleted_grids
            .entry(map_id)
            .or_default()
            .push((self.current_tick, grid_id));
    }

    fn note_chunk_changed(&mut self, grid_id: GridId, chunk_index: keisan::Vector2i, deleted: bool) {
        self.changed_chunks
            .entry(grid_id)
            .or_default()
            .push((self.current_tick, chunk_index, deleted));
    }

    fn ensure_nullspace_exists_and_clear(&mut self, manager: &mut EntityManager) {
        if !self.map_exists(MapId::NULLSPACE) {
            self.create_map(manager, Some(MapId::NULLSPACE));
            return;
        }

        let grids: Vec<_> = self
            .grids
            .iter()
            .filter_map(|(grid_id, uid)| {
                manager
                    .map_grids
                    .get(uid)
                    .filter(|grid| grid.parent_map_id == MapId::NULLSPACE)
                    .map(|_| *grid_id)
            })
            .collect();

        for grid_id in grids {
            self.delete_grid(manager, grid_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MapManager;
    use crate::{EntityManager, MapId, Tile, TileRenderFlag};
    use keisan::{Angle, Box2, Vector2, Vector2i};

    #[test]
    fn map_manager_creates_maps_and_grids_and_can_query_them() {
        let mut entities = EntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities);
        let map_id = maps.create_map(&mut entities, None);
        let grid_id = maps.create_grid(&mut entities, map_id, None, 8);
        let grid_uid = maps.get_grid_euid(grid_id).unwrap();
        let grid = entities.map_grids.get_mut(&grid_uid).unwrap();
        grid.set_tile(Vector2i::new(0, 0), Tile::new(1, TileRenderFlag(0), 0));
        assert!(maps.map_exists(map_id));
        assert!(maps.grid_exists(grid_id));
        assert_eq!(maps.find_grids_intersecting(&entities, map_id, Box2::new(-1.0, -1.0, 1.0, 1.0), false).len(), 1);
        assert!(maps.try_find_grid_at(&entities, map_id, Vector2::ZERO).is_some());
    }

    #[test]
    fn map_manager_tracks_moved_grids() {
        let mut maps = MapManager::new();
        let map = MapId::new(4);
        maps.mark_grid_moved(map, crate::GridId::new(8));
        assert!(maps.get_moved_grids(map).contains(&crate::GridId::new(8)));
        maps.clear_moved_grids(map);
        assert!(maps.get_moved_grids(map).is_empty());
    }

    #[test]
    fn map_manager_tracks_deleted_grids_incrementally() {
        let mut entities = EntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities);
        let map_id = maps.create_map(&mut entities, None);
        maps.set_current_tick(jikan::GameTick::new(5));
        let grid_id = maps.create_grid(&mut entities, map_id, None, 8);
        maps.set_current_tick(jikan::GameTick::new(6));
        maps.delete_grid(&mut entities, grid_id);
        assert_eq!(maps.get_deleted_grids_since(map_id, jikan::GameTick::new(5)), vec![grid_id]);
        maps.cull_deleted_grid_history(jikan::GameTick::new(6));
        assert!(maps.get_deleted_grids_since(map_id, jikan::GameTick::ZERO).is_empty());
    }

    #[test]
    fn map_manager_tracks_changed_chunks_incrementally() {
        let mut entities = EntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities);
        let map_id = maps.create_map(&mut entities, None);
        let grid_id = maps.create_grid(&mut entities, map_id, None, 4);

        maps.set_current_tick(jikan::GameTick::new(2));
        assert!(maps.set_tile(&mut entities, grid_id, Vector2i::new(0, 0), Tile::new(1, TileRenderFlag(0), 0)));
        assert_eq!(maps.get_changed_chunks_since(grid_id, jikan::GameTick::new(1)), vec![(Vector2i::new(0, 0), false)]);

        maps.set_current_tick(jikan::GameTick::new(3));
        assert!(maps.remove_chunk(&mut entities, grid_id, Vector2i::new(0, 0)));
        assert_eq!(maps.get_changed_chunks_since(grid_id, jikan::GameTick::new(2)), vec![(Vector2i::new(0, 0), true)]);

        maps.cull_deleted_grid_history(jikan::GameTick::new(3));
        assert!(maps.get_changed_chunks_since(grid_id, jikan::GameTick::ZERO).is_empty());
    }

    #[test]
    fn map_manager_collects_game_state_map_data_incrementally() {
        let mut entities = EntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities);
        let map_id = maps.create_map(&mut entities, Some(MapId::new(7)));
        let grid_id = maps.create_grid(&mut entities, map_id, Some(crate::GridId::new(12)), 4);
        let grid_uid = maps.get_grid_euid(grid_id).unwrap();

        maps.set_current_tick(jikan::GameTick::new(2));
        assert!(maps.set_tile(
            &mut entities,
            grid_id,
            Vector2i::new(0, 0),
            Tile::new(5, TileRenderFlag(0), 0)
        ));

        let full = maps
            .collect_game_state_map_data(&entities, &[grid_uid], &[grid_uid], jikan::GameTick::ZERO)
            .unwrap();
        assert!(full.grid_data.contains_key(&grid_id));
        assert_eq!(full.grid_data.get(&grid_id).unwrap().chunk_data.len(), 1);
        assert!(full.deleted_grids.is_empty());

        maps.set_current_tick(jikan::GameTick::new(3));
        assert!(maps.remove_chunk(&mut entities, grid_id, Vector2i::new(0, 0)));
        let delta = maps
            .collect_game_state_map_data(&entities, &[grid_uid], &[], jikan::GameTick::new(2))
            .unwrap();
        assert_eq!(delta.grid_data.get(&grid_id).unwrap().chunk_data.len(), 1);
        assert!(delta.grid_data.get(&grid_id).unwrap().chunk_data[0].is_deleted());

        maps.set_current_tick(jikan::GameTick::new(4));
        maps.delete_grid(&mut entities, grid_id);
        let deleted = maps
            .collect_game_state_map_data(&entities, &[], &[], jikan::GameTick::new(3))
            .unwrap();
        assert_eq!(deleted.deleted_grids, vec![grid_id]);
    }

    #[test]
    fn map_manager_reparents_existing_grids_when_map_entity_changes() {
        let mut entities = EntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities);
        let map_id = maps.create_map(&mut entities, Some(MapId::new(9)));
        let original_map_uid = maps.get_map_entity_id(map_id);
        let grid_id = maps.create_grid(&mut entities, map_id, None, 8);
        let grid_uid = maps.get_grid_euid(grid_id).unwrap();

        let replacement = entities.create_entity_uninitialized(None);
        maps.set_map_entity(&mut entities, map_id, replacement);
        entities.flush_queued_deletions();

        let grid_transform = entities.transforms.get(&grid_uid).unwrap();
        assert_eq!(grid_transform.parent, replacement);
        assert_eq!(grid_transform.map_id, map_id);
        assert_eq!(grid_transform.grid_id, grid_id);
        assert!(!entities.entity_exists(original_map_uid));
        assert_eq!(maps.get_map_entity_id(map_id), replacement);
    }

    #[test]
    fn map_manager_create_grid_sets_grid_parent_and_transform_state_atomically() {
        let mut entities = EntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities);
        let map_id = maps.create_map(&mut entities, Some(MapId::new(10)));
        let map_uid = maps.get_map_entity_id(map_id);
        entities.apply_transform_state(
            map_uid,
            crate::TransformComponentState {
                local_position: Vector2::new(3.0, -2.0),
                rotation: Angle::from_degrees(25.0),
                parent_id: crate::EntityUid::INVALID,
                map_id,
                grid_id: crate::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );

        let grid_id = maps.create_grid(&mut entities, map_id, None, 8);
        let grid_uid = maps.get_grid_euid(grid_id).unwrap();
        let transform = entities.transforms.get(&grid_uid).unwrap();
        assert_eq!(transform.parent, map_uid);
        assert_eq!(transform.map_id, map_id);
        assert_eq!(transform.grid_id, grid_id);
        assert_eq!(transform.local_position, Vector2::ZERO);
        assert_eq!(transform.local_rotation, Angle::ZERO);
    }

    #[test]
    fn map_manager_tracks_pre_init_as_pause_and_initialization_state() {
        let mut entities = EntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities);
        let map_id = maps.create_map(&mut entities, Some(MapId::new(11)));
        let map_uid = maps.get_map_entity_id(map_id);
        let child = entities.create_entity_uninitialized_with_transform(
            None,
            crate::TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: map_uid,
                map_id,
                grid_id: crate::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );

        assert!(maps.is_map_initialized(&entities, map_id));
        assert!(!maps.is_map_paused(&entities, map_id));
        assert!(maps.add_uninitialized_map(&mut entities, map_id));
        assert!(!maps.is_map_initialized(&entities, map_id));
        assert!(maps.is_map_paused(&entities, map_id));
        assert!(entities.is_entity_paused(map_uid));
        assert!(entities.is_entity_paused(child));

        assert!(maps.do_map_initialize(&mut entities, map_id));
        assert!(maps.is_map_initialized(&entities, map_id));
        assert!(!maps.is_map_paused(&entities, map_id));
        assert!(!entities.is_entity_paused(map_uid));
        assert!(!entities.is_entity_paused(child));
    }

    #[test]
    fn map_manager_can_create_uninitialized_map_and_report_grid_pause() {
        let mut entities = EntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities);
        let map_id = maps.create_uninitialized_map(&mut entities, Some(MapId::new(12)));
        let grid_id = maps.create_grid(&mut entities, map_id, Some(crate::GridId::new(22)), 8);

        assert!(!maps.is_map_initialized(&entities, map_id));
        assert!(maps.is_map_paused(&entities, map_id));
        assert!(maps.is_grid_paused(&entities, grid_id));

        assert!(maps.do_map_initialize(&mut entities, map_id));
        assert!(maps.is_map_initialized(&entities, map_id));
        assert!(!maps.is_grid_paused(&entities, grid_id));
    }
}
