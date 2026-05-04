use crate::EntityUid;
use core::fmt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct GridId {
    value: i32,
}

impl GridId {
    pub const INVALID: Self = Self { value: 0 };

    pub const fn new(value: i32) -> Self {
        Self { value }
    }

    pub fn is_valid(self) -> bool {
        self.value > 0
    }

    pub fn raw(self) -> i32 {
        self.value
    }
}

impl From<GridId> for i32 {
    fn from(value: GridId) -> Self {
        value.value
    }
}

impl From<GridId> for EntityUid {
    fn from(value: GridId) -> Self {
        EntityUid::new(value.value)
    }
}

impl fmt::Display for GridId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}
