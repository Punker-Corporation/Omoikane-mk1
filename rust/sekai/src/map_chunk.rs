use crate::{EntityUid, Tile};
use jikan::GameTick;
use keisan::{Box2i, Vector2i};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileModifiedEvent {
    pub chunk_indices: Vector2i,
    pub tile_indices: Vector2i,
    pub new_tile: Tile,
    pub old_tile: Tile,
    pub chunk_shape_changed: bool,
}

#[derive(Debug, Clone)]
pub struct MapChunk {
    indices: Vector2i,
    chunk_size: u16,
    tiles: Vec<Tile>,
    snap_grid: Vec<Vec<EntityUid>>,
    pub filled_tiles: i32,
    pub cached_bounds: Box2i,
    pub last_tile_modified_tick: GameTick,
    pub suppress_collision_regeneration: bool,
}

impl MapChunk {
    pub fn new(x: i32, y: i32, chunk_size: u16) -> Self {
        let cell_count = chunk_size as usize * chunk_size as usize;
        Self {
            indices: Vector2i::new(x, y),
            chunk_size,
            tiles: vec![Tile::EMPTY; cell_count],
            snap_grid: vec![Vec::new(); cell_count],
            filled_tiles: 0,
            cached_bounds: Box2i::default(),
            last_tile_modified_tick: GameTick::ZERO,
            suppress_collision_regeneration: false,
        }
    }

    pub fn chunk_size(&self) -> u16 {
        self.chunk_size
    }

    pub fn x(&self) -> i32 {
        self.indices.x
    }

    pub fn y(&self) -> i32 {
        self.indices.y
    }

    pub fn indices(&self) -> Vector2i {
        self.indices
    }

    fn offset(&self, x_index: u16, y_index: u16) -> usize {
        assert!(x_index < self.chunk_size, "tile index out of bounds");
        assert!(y_index < self.chunk_size, "tile index out of bounds");
        x_index as usize * self.chunk_size as usize + y_index as usize
    }

    pub fn get_tile(&self, x_index: u16, y_index: u16) -> Tile {
        self.tiles[self.offset(x_index, y_index)]
    }

    pub fn set_tile(&mut self, x_index: u16, y_index: u16, tile: Tile) -> Option<TileModifiedEvent> {
        let offset = self.offset(x_index, y_index);
        let old_tile = self.tiles[offset];
        if old_tile == tile {
            return None;
        }

        let old_filled = self.filled_tiles;
        if old_tile.is_empty() != tile.is_empty() {
            if old_tile.is_empty() {
                self.filled_tiles += 1;
            } else {
                self.filled_tiles -= 1;
            }
        }

        self.tiles[offset] = tile;
        Some(TileModifiedEvent {
            chunk_indices: self.indices,
            tile_indices: Vector2i::new(x_index as i32, y_index as i32),
            new_tile: tile,
            old_tile,
            chunk_shape_changed: old_filled != self.filled_tiles,
        })
    }

    pub fn grid_tile_to_chunk_tile(&self, grid_tile: Vector2i) -> Vector2i {
        let modulus = |value: i32| ((value % self.chunk_size as i32) + self.chunk_size as i32) % self.chunk_size as i32;
        Vector2i::new(
            modulus(grid_tile.x),
            modulus(grid_tile.y),
        )
    }

    pub fn chunk_tile_to_grid_tile(&self, chunk_tile: Vector2i) -> Vector2i {
        chunk_tile + self.indices * self.chunk_size as i32
    }

    pub fn get_snap_grid_cell(&self, x_cell: u16, y_cell: u16) -> &[EntityUid] {
        &self.snap_grid[self.offset(x_cell, y_cell)]
    }

    pub fn add_to_snap_grid_cell(&mut self, x_cell: u16, y_cell: u16, entity: EntityUid) {
        let offset = self.offset(x_cell, y_cell);
        let cell = &mut self.snap_grid[offset];
        assert!(!cell.contains(&entity), "entity already anchored in cell");
        cell.push(entity);
    }

    pub fn remove_from_snap_grid_cell(&mut self, x_cell: u16, y_cell: u16, entity: EntityUid) {
        let offset = self.offset(x_cell, y_cell);
        let cell = &mut self.snap_grid[offset];
        if let Some(index) = cell.iter().position(|current| *current == entity) {
            cell.remove(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MapChunk;
    use crate::{EntityUid, Tile, TileRenderFlag};
    use keisan::Vector2i;

    #[test]
    fn map_chunk_tracks_tiles_and_snap_grid() {
        let mut chunk = MapChunk::new(1, -2, 4);
        let tile = Tile::new(5, TileRenderFlag(0), 0);
        let event = chunk.set_tile(1, 2, tile).unwrap();
        assert_eq!(event.tile_indices, Vector2i::new(1, 2));
        assert_eq!(chunk.filled_tiles, 1);
        assert_eq!(chunk.grid_tile_to_chunk_tile(Vector2i::new(5, -6)), Vector2i::new(1, 2));
        chunk.add_to_snap_grid_cell(1, 2, EntityUid::new(7));
        assert_eq!(chunk.get_snap_grid_cell(1, 2), &[EntityUid::new(7)]);
        chunk.remove_from_snap_grid_cell(1, 2, EntityUid::new(7));
        assert!(chunk.get_snap_grid_cell(1, 2).is_empty());
    }
}
