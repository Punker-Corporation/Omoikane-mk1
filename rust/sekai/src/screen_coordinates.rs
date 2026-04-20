use crate::WindowId;
use core::fmt;
use keisan::Vector2;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScreenCoordinates {
    pub position: Vector2,
    pub window: WindowId,
}

impl ScreenCoordinates {
    pub const fn new(position: Vector2, window: WindowId) -> Self {
        Self { position, window }
    }

    pub fn new_xy(x: f32, y: f32, window: WindowId) -> Self {
        Self::new(Vector2::new(x, y), window)
    }

    pub fn x(self) -> f32 {
        self.position.x
    }

    pub fn y(self) -> f32 {
        self.position.y
    }

    pub fn is_valid(self) -> bool {
        self.window != WindowId::INVALID
    }
}

impl fmt::Display for ScreenCoordinates {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, W{})", self.position.x, self.position.y, self.window.raw())
    }
}
