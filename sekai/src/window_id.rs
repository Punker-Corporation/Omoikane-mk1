use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct WindowId {
    value: i32,
}

impl WindowId {
    pub const INVALID: Self = Self { value: 0 };
    pub const MAIN: Self = Self { value: 1 };

    pub const fn new(value: i32) -> Self {
        Self { value }
    }

    pub fn raw(self) -> i32 {
        self.value
    }
}

impl From<WindowId> for i32 {
    fn from(value: WindowId) -> Self {
        value.value
    }
}

impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Window {}", self.value)
    }
}
