use crate::{
    EntityManager, EntityUid, GridId, MapCoordinates, MapGrid, MapGridComponent, MapId,
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

        let new_entity = manager.create_entity_uninitialized(None);
        let map_component = manager.ensure_map(actual_id, new_entity);
        map_component.world_map = actual_id;
        self.map_entities.insert(actual_id, new_entity);
        actual_id
    }

    pub fn map_exists(&self, map_id: MapId) -> bool {
        self.map_entities.contains_key(&map_id)
    }

    pub fn create_new_map_entity(&mut self, manager: &mut EntityManager, map_id: MapId) -> EntityUid {
        let entity = manager.create_entity_uninitialized(None);
        self.set_map_entity(manager, map_id, entity);
        entity
    }

    pub fn set_map_entity(&mut self, manager: &mut EntityManager, map_id: MapId, new_map_entity_id: EntityUid) {
        assert!(self.map_exists(map_id), "map does not exist: {map_id}");

        if let Some(old) = self.map_entities.insert(map_id, new_map_entity_id) {
            if old.is_valid() && old != new_map_entity_id {
                manager.queue_delete_entity(old);
            }
        }

        let map_component = manager.ensure_map(map_id, new_map_entity_id);
        map_component.world_map = map_id;
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

        let grid_entity = manager.create_entity_uninitialized(None);
        let component = manager.map_grid_components.entry(grid_entity).or_insert_with(|| {
            let mut component = MapGridComponent::new();
            component.base.owner = grid_entity;
            component
        });
        component.grid_index = actual_id;
        component.chunk_size = chunk_size;
        let grid = component.alloc_map_grid(grid_entity, current_map_id, 1).clone();
        manager.map_grids.insert(grid_entity, grid);
        if let Some(transform) = manager.transforms.get_mut(&grid_entity) {
            transform.map_id = current_map_id;
            transform.rebuild_for_manager();
        }
        let _ = manager.set_parent(grid_entity, self.get_map_entity_id(current_map_id));

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
    use keisan::{Box2, Vector2, Vector2i};

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
}
