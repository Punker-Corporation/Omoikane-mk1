use crate::ServerEntityManager;
use sekai::{EntityUid, GridId, MapId, MapManager};

#[derive(Debug, Clone)]
pub struct MapSystem {
    pub delete_empty_grids: bool,
}

impl MapSystem {
    pub fn new() -> Self {
        Self {
            delete_empty_grids: false,
        }
    }

    pub fn set_grid_deletion(&mut self, value: bool, maps: &mut MapManager, entities: &mut ServerEntityManager) {
        self.delete_empty_grids = value;
        if !value {
            return;
        }
        let empty: Vec<_> = entities
            .inner
            .map_grids
            .values()
            .filter(|grid| self.grid_empty(grid))
            .map(|grid| grid.index)
            .collect();
        for grid_id in empty {
            maps.delete_grid(&mut entities.inner, grid_id);
        }
    }

    pub fn on_grid_empty(&self, maps: &mut MapManager, entities: &mut ServerEntityManager, grid_id: GridId) -> bool {
        if !self.delete_empty_grids {
            return false;
        }
        maps.delete_grid(&mut entities.inner, grid_id);
        true
    }

    pub fn handle_map_created(&self, maps: &mut MapManager, _entities: &mut ServerEntityManager, map: MapId) -> Option<EntityUid> {
        (map != MapId::NULLSPACE).then(|| maps.get_map_entity_id_or_throw(map))
    }

    fn grid_empty(&self, grid: &sekai::MapGrid) -> bool {
        !grid.get_all_tiles(true).into_iter().any(|tile| !tile.tile.is_empty())
    }
}

impl Default for MapSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::MapSystem;
    use crate::ServerEntityManager;
    use sekai::{GridId, MapId, MapManager};

    #[test]
    fn map_system_can_delete_empty_grids_when_enabled() {
        let mut maps = MapManager::new();
        let mut entities = ServerEntityManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(2)));
        let grid_id = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(5)), 8);
        let mut system = MapSystem::new();
        system.set_grid_deletion(true, &mut maps, &mut entities);
        assert!(!maps.grid_exists(grid_id));
    }
}
