use crate::MapId;
use keisan::Vector2;
use core::fmt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct MapCoordinates {
    pub position: Vector2,
    pub map_id: MapId,
}

impl MapCoordinates {
    pub const NULLSPACE: Self = Self {
        position: Vector2::ZERO,
        map_id: MapId::NULLSPACE,
    };

    pub const fn new(position: Vector2, map_id: MapId) -> Self {
        Self { position, map_id }
    }

    pub fn new_xy(x: f32, y: f32, map_id: MapId) -> Self {
        Self::new(Vector2::new(x, y), map_id)
    }

    pub fn x(self) -> f32 {
        self.position.x
    }

    pub fn y(self) -> f32 {
        self.position.y
    }

    pub fn in_range(self, other: Self, range: f32) -> bool {
        if other.map_id != self.map_id {
            return false;
        }
        (other.position - self.position).length_squared() < range * range
    }

    pub fn offset(self, offset: Vector2) -> Self {
        Self::new(self.position + offset, self.map_id)
    }

    pub fn offset_xy(self, x: f32, y: f32) -> Self {
        self.offset(Vector2::new(x, y))
    }
}

impl fmt::Display for MapCoordinates {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Map={}, X={:.2}, Y={:.2}", self.map_id, self.position.x, self.position.y)
    }
}

#[cfg(test)]
mod tests {
    use super::MapCoordinates;
    use crate::MapId;
    use keisan::Vector2;

    #[test]
    fn map_coordinates_range_respects_map_identity() {
        let a = MapCoordinates::new(Vector2::new(0.0, 0.0), MapId::new(1));
        let b = MapCoordinates::new(Vector2::new(3.0, 4.0), MapId::new(1));
        let c = MapCoordinates::new(Vector2::new(3.0, 4.0), MapId::new(2));
        assert!(a.in_range(b, 6.0));
        assert!(!a.in_range(c, 6.0));
    }
}
