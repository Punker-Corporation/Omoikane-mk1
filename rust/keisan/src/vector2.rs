use crate::{ApproxEq, MathHelper, Vector2i};
use core::fmt;
use core::ops::{Add, Div, Mul, Neg, Sub};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

impl Vector2 {
    pub const ZERO: Self = Self::new(0.0, 0.0);
    pub const ONE: Self = Self::new(1.0, 1.0);
    pub const UNIT_X: Self = Self::new(1.0, 0.0);
    pub const UNIT_Y: Self = Self::new(0.0, 1.0);
    pub const INFINITY: Self = Self::new(f32::INFINITY, f32::INFINITY);
    pub const NAN: Self = Self::new(f32::NAN, f32::NAN);

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn normalized(self) -> Self {
        let length = self.length();
        Self::new(self.x / length, self.y / length)
    }

    pub fn rotated_90_degrees_clockwise_world(self) -> Self {
        Self::new(self.y, -self.x)
    }

    pub fn rotated_90_degrees_anticlockwise_world(self) -> Self {
        Self::new(-self.y, self.x)
    }

    pub fn rounded(self) -> Self {
        Self::new(self.x.round(), self.y.round())
    }

    pub fn floored(self) -> Vector2i {
        Vector2i::new(self.x.floor() as i32, self.y.floor() as i32)
    }

    pub fn ceiled(self) -> Vector2i {
        Vector2i::new(self.x.ceil() as i32, self.y.ceil() as i32)
    }

    pub fn component_min(a: Self, b: Self) -> Self {
        Self::new(a.x.min(b.x), a.y.min(b.y))
    }

    pub fn component_max(a: Self, b: Self) -> Self {
        Self::new(a.x.max(b.x), a.y.max(b.y))
    }

    pub fn magnitude_min(a: Self, b: Self) -> Self {
        if a.length_squared() < b.length_squared() {
            a
        } else {
            b
        }
    }

    pub fn magnitude_max(a: Self, b: Self) -> Self {
        if a.length_squared() >= b.length_squared() {
            a
        } else {
            b
        }
    }

    pub fn clamp(vector: Self, min: Self, max: Self) -> Self {
        Self::new(
            MathHelper::clamp(vector.x, min.x, max.x),
            MathHelper::clamp(vector.y, min.y, max.y),
        )
    }

    pub fn abs(a: Self) -> Self {
        Self::new(a.x.abs(), a.y.abs())
    }

    pub fn dot(a: Self, b: Self) -> f32 {
        a.x * b.x + a.y * b.y
    }

    pub fn cross(a: Self, b: Self) -> f32 {
        a.x * b.y - a.y * b.x
    }

    pub fn cross_vector_scalar(a: Self, s: f32) -> Self {
        Self::new(s * a.y, -s * a.x)
    }

    pub fn cross_scalar_vector(s: f32, a: Self) -> Self {
        Self::new(-s * a.y, s * a.x)
    }

    pub fn lerp(a: Self, b: Self, factor: f32) -> Self {
        Self::new(
            MathHelper::lerp(a.x, b.x, factor),
            MathHelper::lerp(a.y, b.y, factor),
        )
    }

    pub fn lerp_clamped(a: Self, b: Self, factor: f32) -> Self {
        if factor <= 0.0 {
            a
        } else if factor >= 1.0 {
            b
        } else {
            Self::lerp(a, b, factor)
        }
    }

    pub fn interpolate_cubic(pre_a: Self, a: Self, b: Self, post_b: Self, t: f32) -> Self {
        a + (b - pre_a
            + (pre_a * 2.0 - a * 5.0 + b * 4.0 - post_b + ((a - b) * 3.0 + post_b - pre_a) * t) * t)
            * t
            * 0.5
    }
}

impl PartialEq for Vector2 {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl fmt::Debug for Vector2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl fmt::Display for Vector2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl Add for Vector2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Add<f32> for Vector2 {
    type Output = Self;

    fn add(self, rhs: f32) -> Self::Output {
        Self::new(self.x + rhs, self.y + rhs)
    }
}

impl Sub for Vector2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Sub<f32> for Vector2 {
    type Output = Self;

    fn sub(self, rhs: f32) -> Self::Output {
        Self::new(self.x - rhs, self.y - rhs)
    }
}

impl Neg for Vector2 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

impl Mul<f32> for Vector2 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl Mul for Vector2 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.x * rhs.x, self.y * rhs.y)
    }
}

impl Div<f32> for Vector2 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl Div for Vector2 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.x / rhs.x, self.y / rhs.y)
    }
}

impl From<(f32, f32)> for Vector2 {
    fn from(value: (f32, f32)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl ApproxEq for Vector2 {
    fn approx_eq(&self, other: Self) -> bool {
        MathHelper::close_to(self.x, other.x, 0.000_000_1)
            && MathHelper::close_to(self.y, other.y, 0.000_000_1)
    }

    fn approx_eq_with_tolerance(&self, other: Self, tolerance: f64) -> bool {
        MathHelper::close_to_f64(self.x as f64, other.x as f64, tolerance)
            && MathHelper::close_to_f64(self.y as f64, other.y as f64, tolerance)
    }
}

#[cfg(test)]
mod tests {
    use super::Vector2;

    #[test]
    fn vector_arithmetic_matches_expected_shape() {
        let a = Vector2::new(3.0, 4.0);
        let b = Vector2::new(1.0, -2.0);
        assert_eq!(a + b, Vector2::new(4.0, 2.0));
        assert_eq!(a - b, Vector2::new(2.0, 6.0));
        assert_eq!(a * 2.0, Vector2::new(6.0, 8.0));
        assert_eq!(a / 2.0, Vector2::new(1.5, 2.0));
        assert_eq!(a.length(), 5.0);
    }

    #[test]
    fn cross_products_match_csharp_semantics() {
        let a = Vector2::new(2.0, 3.0);
        let b = Vector2::new(5.0, 7.0);
        assert_eq!(Vector2::cross(a, b), -1.0);
        assert_eq!(
            Vector2::cross_vector_scalar(a, 2.0),
            Vector2::new(6.0, -4.0)
        );
        assert_eq!(
            Vector2::cross_scalar_vector(2.0, a),
            Vector2::new(-6.0, 4.0)
        );
    }
}
