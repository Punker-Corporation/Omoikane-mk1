use crate::Vector2;
use core::fmt;
use core::ops::{Add, Div, Mul, Neg, Sub};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Vector2i {
    pub x: i32,
    pub y: i32,
}

impl Vector2i {
    pub const ZERO: Self = Self::new(0, 0);
    pub const ONE: Self = Self::new(1, 1);

    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn component_max(a: Self, b: Self) -> Self {
        Self::new(a.x.max(b.x), a.y.max(b.y))
    }

    pub fn component_min(a: Self, b: Self) -> Self {
        Self::new(a.x.min(b.x), a.y.min(b.y))
    }
}

impl fmt::Debug for Vector2i {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl fmt::Display for Vector2i {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl Add for Vector2i {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Add<i32> for Vector2i {
    type Output = Self;

    fn add(self, rhs: i32) -> Self::Output {
        Self::new(self.x + rhs, self.y + rhs)
    }
}

impl Sub for Vector2i {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Sub<i32> for Vector2i {
    type Output = Self;

    fn sub(self, rhs: i32) -> Self::Output {
        Self::new(self.x - rhs, self.y - rhs)
    }
}

impl Neg for Vector2i {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

impl Mul for Vector2i {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.x * rhs.x, self.y * rhs.y)
    }
}

impl Mul<i32> for Vector2i {
    type Output = Self;

    fn mul(self, rhs: i32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl Mul<f32> for Vector2i {
    type Output = Vector2;

    fn mul(self, rhs: f32) -> Self::Output {
        Vector2::new(self.x as f32 * rhs, self.y as f32 * rhs)
    }
}

impl Div for Vector2i {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.x / rhs.x, self.y / rhs.y)
    }
}

impl Div<i32> for Vector2i {
    type Output = Self;

    fn div(self, rhs: i32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl Div<f32> for Vector2i {
    type Output = Vector2;

    fn div(self, rhs: f32) -> Self::Output {
        Vector2::new(self.x as f32 / rhs, self.y as f32 / rhs)
    }
}

impl From<(i32, i32)> for Vector2i {
    fn from(value: (i32, i32)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl From<Vector2i> for Vector2 {
    fn from(value: Vector2i) -> Self {
        Vector2::new(value.x as f32, value.y as f32)
    }
}

impl From<Vector2> for Vector2i {
    fn from(value: Vector2) -> Self {
        Self::new(value.x as i32, value.y as i32)
    }
}
