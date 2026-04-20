use crate::{Component, ComponentStateValue, EntityUid, GridId, MapGrid};
use keisan::Vector2i;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct MapGridComponent {
    pub base: Component,
    pub grid_index: GridId,
    pub chunk_size: u16,
    pub grid: Option<MapGrid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapGridComponentState {
    pub grid_index: GridId,
    pub chunk_size: u16,
}

impl MapGridComponent {
    pub fn new() -> Self {
        Self {
            base: Component::new("MapGridComponent"),
            grid_index: GridId::INVALID,
            chunk_size: 16,
            grid: None,
        }
    }

    pub fn alloc_map_grid(&mut self, owner: EntityUid, parent_map_id: crate::MapId, tile_size: u16) -> &mut MapGrid {
        let mut grid = MapGrid::new(parent_map_id, owner, self.grid_index, self.chunk_size);
        grid.tile_size = tile_size;
        self.grid = Some(grid);
        self.grid.as_mut().unwrap()
    }

    pub fn anchor_entity(&mut self, tile_indices: Vector2i, entity: EntityUid) -> bool {
        let Some(grid) = self.grid.as_mut() else {
            return false;
        };
        grid.add_to_snap_grid_cell(tile_indices, entity)
    }

    pub fn unanchor_entity(&mut self, tile_indices: Vector2i, entity: EntityUid) {
        if let Some(grid) = self.grid.as_mut() {
            grid.remove_from_snap_grid_cell(tile_indices, entity);
        }
    }

    pub fn get_component_state(&self) -> MapGridComponentState {
        MapGridComponentState {
            grid_index: self.grid_index,
            chunk_size: self.chunk_size,
        }
    }

    pub fn handle_map_grid_state(&mut self, state: MapGridComponentState) {
        self.grid_index = state.grid_index;
        self.chunk_size = state.chunk_size;
    }

    pub fn as_state_value(&self) -> ComponentStateValue {
        std::sync::Arc::new(self.get_component_state())
    }
}

#[cfg(test)]
mod tests {
    use super::{MapGridComponent, MapGridComponentState};
    use crate::{EntityUid, GridId, MapId, Tile, TileRenderFlag};
    use keisan::Vector2i;

    #[test]
    fn map_grid_component_allocates_and_anchors() {
        let mut component = MapGridComponent::new();
        component.handle_map_grid_state(MapGridComponentState {
            grid_index: GridId::new(4),
            chunk_size: 8,
        });
        let grid = component.alloc_map_grid(EntityUid::new(20), MapId::new(1), 1);
        grid.set_tile(Vector2i::new(0, 0), Tile::new(1, TileRenderFlag(0), 0));
        assert!(component.anchor_entity(Vector2i::new(0, 0), EntityUid::new(30)));
        assert_eq!(component.grid.as_ref().unwrap().get_anchored_entities(Vector2i::new(0, 0)).len(), 1);
    }
}
