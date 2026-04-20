use core::fmt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct MapId {
    value: i32,
}

impl MapId {
    pub const NULLSPACE: Self = Self { value: 0 };

    pub const fn new(value: i32) -> Self {
        Self { value }
    }

    pub fn raw(self) -> i32 {
        self.value
    }
}

impl From<MapId> for i32 {
    fn from(value: MapId) -> Self {
        value.value
    }
}

impl fmt::Display for MapId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}
