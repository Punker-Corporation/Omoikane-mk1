use crate::{GridId, MapId, Tile};
use core::fmt;
use keisan::Vector2i;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TileRef {
    pub map_index: MapId,
    pub grid_index: GridId,
    pub grid_indices: Vector2i,
    pub tile: Tile,
}

impl TileRef {
    pub const ZERO: Self = Self {
        map_index: MapId::NULLSPACE,
        grid_index: GridId::INVALID,
        grid_indices: Vector2i::ZERO,
        tile: Tile::EMPTY,
    };

    pub const fn new(
        map_index: MapId,
        grid_index: GridId,
        grid_indices: Vector2i,
        tile: Tile,
    ) -> Self {
        Self {
            map_index,
            grid_index,
            grid_indices,
            tile,
        }
    }

    pub fn new_xy(
        map_index: MapId,
        grid_index: GridId,
        x_index: i32,
        y_index: i32,
        tile: Tile,
    ) -> Self {
        Self::new(map_index, grid_index, Vector2i::new(x_index, y_index), tile)
    }

    pub fn x(self) -> i32 {
        self.grid_indices.x
    }

    pub fn y(self) -> i32 {
        self.grid_indices.y
    }
}

impl fmt::Display for TileRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TileRef: {},{} ({})", self.x(), self.y(), self.tile)
    }
}
