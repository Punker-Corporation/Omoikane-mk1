use core::fmt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct TileRenderFlag(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Tile {
    pub type_id: u16,
    pub flags: TileRenderFlag,
    pub variant: u8,
}

impl Tile {
    pub const EMPTY: Self = Self {
        type_id: 0,
        flags: TileRenderFlag(0),
        variant: 0,
    };

    pub const fn new(type_id: u16, flags: TileRenderFlag, variant: u8) -> Self {
        Self {
            type_id,
            flags,
            variant,
        }
    }

    pub fn is_empty(self) -> bool {
        self.type_id == 0
    }

    pub fn pack(self) -> u32 {
        ((self.type_id as u32) << 16) | ((self.flags.0 as u32) << 8) | self.variant as u32
    }

    pub fn unpack(value: u32) -> Self {
        Self::new((value >> 16) as u16, TileRenderFlag((value >> 8) as u8), value as u8)
    }
}

impl From<Tile> for u32 {
    fn from(value: Tile) -> Self {
        value.pack()
    }
}

impl From<u32> for Tile {
    fn from(value: u32) -> Self {
        Self::unpack(value)
    }
}

impl fmt::Display for Tile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tile {}, {}, {}", self.type_id, self.flags.0, self.variant)
    }
}

#[cfg(test)]
mod tests {
    use super::{Tile, TileRenderFlag};

    #[test]
    fn tile_pack_roundtrips() {
        let tile = Tile::new(42, TileRenderFlag(3), 9);
        assert_eq!(Tile::unpack(tile.pack()), tile);
    }
}
