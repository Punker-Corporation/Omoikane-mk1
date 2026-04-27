use core::fmt;
use core::ops::{Add, Sub};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct GameTick {
    pub value: u32,
}

impl GameTick {
    pub const ZERO: Self = Self { value: 0 };
    pub const FIRST: Self = Self { value: 1 };
    pub const MAX_VALUE: Self = Self { value: u32::MAX };

    pub const fn new(value: u32) -> Self {
        Self { value }
    }
}

impl Add<u32> for GameTick {
    type Output = Self;

    fn add(self, rhs: u32) -> Self::Output {
        Self::new(self.value + rhs)
    }
}

impl Sub<u32> for GameTick {
    type Output = Self;

    fn sub(self, rhs: u32) -> Self::Output {
        Self::new(self.value - rhs)
    }
}

impl fmt::Display for GameTick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}
