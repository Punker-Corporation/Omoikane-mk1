use crate::{ApproxEq, MathHelper, Vector2, Vector4};
use core::fmt;
use core::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Clone, Copy, Default)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0);
    pub const UNIT_X: Self = Self::new(1.0, 0.0, 0.0);
    pub const UNIT_Y: Self = Self::new(0.0, 1.0, 0.0);
    pub const UNIT_Z: Self = Self::new(0.0, 0.0, 1.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub const fn splat(value: f32) -> Self {
        Self::new(value, value, value)
    }

    pub fn xy(self) -> Vector2 {
        Vector2::new(self.x, self.y)
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn normalized(self) -> Self {
        self / self.length()
    }

    pub fn normalize(&mut self) {
        *self = self.normalized();
    }

    pub fn component_min(a: Self, b: Self) -> Self {
        Self::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z))
    }

    pub fn component_max(a: Self, b: Self) -> Self {
        Self::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z))
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
            MathHelper::clamp(vector.z, min.z, max.z),
        )
    }

    pub fn dot(a: Self, b: Self) -> f32 {
        a.x * b.x + a.y * b.y + a.z * b.z
    }

    pub fn cross(a: Self, b: Self) -> Self {
        Self::new(
            a.y * b.z - a.z * b.y,
            a.z * b.x - a.x * b.z,
            a.x * b.y - a.y * b.x,
        )
    }

    pub fn lerp(a: Self, b: Self, factor: f32) -> Self {
        Self::new(
            MathHelper::lerp(a.x, b.x, factor),
            MathHelper::lerp(a.y, b.y, factor),
            MathHelper::lerp(a.z, b.z, factor),
        )
    }

    pub fn barycentric(a: Self, b: Self, c: Self, u: f32, v: f32) -> Self {
        a + (b - a) * u + (c - a) * v
    }

    pub fn interpolate_cubic(pre_a: Self, a: Self, b: Self, post_b: Self, t: f32) -> Self {
        a + (b - pre_a + (pre_a * 2.0 - a * 5.0 + b * 4.0 - post_b + ((a - b) * 3.0 + post_b - pre_a) * t) * t) * t * 0.5
    }

    pub fn calculate_angle(first: Self, second: Self) -> f32 {
        f32::acos(Self::dot(first, second) / (first.length() * second.length()))
    }
}

impl PartialEq for Vector3 {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z
    }
}

impl fmt::Debug for Vector3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

impl fmt::Display for Vector3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

impl Add for Vector3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Add<f32> for Vector3 {
    type Output = Self;

    fn add(self, rhs: f32) -> Self::Output {
        Self::new(self.x + rhs, self.y + rhs, self.z + rhs)
    }
}

impl Sub for Vector3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Sub<f32> for Vector3 {
    type Output = Self;

    fn sub(self, rhs: f32) -> Self::Output {
        Self::new(self.x - rhs, self.y - rhs, self.z - rhs)
    }
}

impl Neg for Vector3 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y, -self.z)
    }
}

impl Mul<f32> for Vector3 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl Mul for Vector3 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.x * rhs.x, self.y * rhs.y, self.z * rhs.z)
    }
}

impl Div<f32> for Vector3 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

impl Div for Vector3 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.x / rhs.x, self.y / rhs.y, self.z / rhs.z)
    }
}

impl From<(f32, f32, f32)> for Vector3 {
    fn from(value: (f32, f32, f32)) -> Self {
        Self::new(value.0, value.1, value.2)
    }
}

impl From<f32> for Vector3 {
    fn from(value: f32) -> Self {
        Self::splat(value)
    }
}

impl From<Vector2> for Vector3 {
    fn from(value: Vector2) -> Self {
        Self::new(value.x, value.y, 0.0)
    }
}

impl From<Vector4> for Vector3 {
    fn from(value: Vector4) -> Self {
        Self::new(value.x, value.y, value.z)
    }
}

impl ApproxEq for Vector3 {
    fn approx_eq(&self, other: Self) -> bool {
        MathHelper::close_to(self.x, other.x, 0.000_000_1)
            && MathHelper::close_to(self.y, other.y, 0.000_000_1)
            && MathHelper::close_to(self.z, other.z, 0.000_000_1)
    }

    fn approx_eq_with_tolerance(&self, other: Self, tolerance: f64) -> bool {
        MathHelper::close_to_f64(self.x as f64, other.x as f64, tolerance)
            && MathHelper::close_to_f64(self.y as f64, other.y as f64, tolerance)
            && MathHelper::close_to_f64(self.z as f64, other.z as f64, tolerance)
    }
}

#[cfg(test)]
mod tests {
    use super::Vector3;
    use crate::{ApproxEq, Vector2, Vector4};

    #[test]
    fn vector3_arithmetic_and_norms_match_expected_shape() {
        let a = Vector3::new(3.0, 4.0, 12.0);
        let b = Vector3::new(1.0, -2.0, 5.0);
        assert_eq!(a + b, Vector3::new(4.0, 2.0, 17.0));
        assert_eq!(a - b, Vector3::new(2.0, 6.0, 7.0));
        assert_eq!(a * 2.0, Vector3::new(6.0, 8.0, 24.0));
        assert_eq!(a / 2.0, Vector3::new(1.5, 2.0, 6.0));
        assert_eq!(a.length(), 13.0);
    }

    #[test]
    fn vector3_cross_dot_and_lerp_match_csharp_semantics() {
        let a = Vector3::UNIT_X;
        let b = Vector3::UNIT_Y;
        assert_eq!(Vector3::dot(a, b), 0.0);
        assert_eq!(Vector3::cross(a, b), Vector3::UNIT_Z);
        assert_eq!(Vector3::lerp(Vector3::ZERO, Vector3::ONE, 0.25), Vector3::splat(0.25));
    }

    #[test]
    fn vector3_conversions_preserve_components() {
        let from_v2 = Vector3::from(Vector2::new(2.0, 3.0));
        let from_v4 = Vector3::from(Vector4::new(2.0, 3.0, 4.0, 5.0));
        assert_eq!(from_v2, Vector3::new(2.0, 3.0, 0.0));
        assert_eq!(from_v4, Vector3::new(2.0, 3.0, 4.0));
        assert!(Vector3::new(0.0, 3.0, 4.0).normalized().approx_eq(Vector3::new(0.0, 0.6, 0.8)));
    }
}
