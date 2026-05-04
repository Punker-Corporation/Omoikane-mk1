use crate::{ApproxEq, Direction, MathHelper, Vector2, Vector2i};
use core::fmt;
use core::ops::{Add, Mul, Neg, Rem, Sub};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Angle {
    pub theta: f64,
}

impl Angle {
    pub const ZERO: Self = Self { theta: 0.0 };

    pub const fn new(theta: f64) -> Self {
        Self { theta }
    }

    pub fn from_vec(dir: Vector2) -> Self {
        let dir = dir.normalized();
        Self::new((dir.y as f64).atan2(dir.x as f64))
    }

    pub fn from_world_vec(dir: Vector2) -> Self {
        Self::from_vec(dir) + MathHelper::PI_OVER_2 as f64
    }

    pub fn from_degrees(degrees: f64) -> Self {
        Self::new(MathHelper::degrees_to_radians_f64(degrees))
    }

    pub fn degrees(self) -> f64 {
        MathHelper::radians_to_degrees_f64(self.theta)
    }

    pub fn to_vec(self) -> Vector2 {
        Vector2::new(self.theta.cos() as f32, self.theta.sin() as f32)
    }

    pub fn to_world_vec(self) -> Vector2 {
        (self - MathHelper::PI_OVER_2 as f64).to_vec()
    }

    pub fn get_dir(self) -> Direction {
        const SEGMENT: f64 = 2.0 * core::f64::consts::PI / 8.0;
        const OFFSET: f64 = SEGMENT / 2.0;
        let mut ang = self.theta % (2.0 * core::f64::consts::PI);
        if ang < 0.0 {
            ang += 2.0 * core::f64::consts::PI;
        }
        Direction::from_index(((ang + OFFSET) / SEGMENT).floor() as i32 % 8)
    }

    pub fn get_cardinal_dir(self) -> Direction {
        const SEGMENT: f64 = 2.0 * core::f64::consts::PI / 4.0;
        const OFFSET: f64 = SEGMENT / 2.0;
        let mut ang = self.theta % (2.0 * core::f64::consts::PI);
        if ang < 0.0 {
            ang += 2.0 * core::f64::consts::PI;
        }
        Direction::from_index((((ang + OFFSET) / SEGMENT).floor() as i32 * 2) % 8)
    }

    pub fn rotate_vec(self, vec: Vector2) -> Vector2 {
        if self.theta == 0.0 {
            return vec;
        }
        let cos = self.theta.cos() as f32;
        let sin = self.theta.sin() as f32;
        Vector2::new(cos * vec.x - sin * vec.y, sin * vec.x + cos * vec.y)
    }

    pub fn reduced(self) -> Self {
        Self::new(Self::reduce(self.theta))
    }

    pub fn opposite(self) -> Self {
        Self::new(Self::flip_positive_raw(self.theta - core::f64::consts::PI))
    }

    pub fn flip_positive(self) -> Self {
        Self::new(Self::flip_positive_raw(self.theta))
    }

    pub fn lerp(a: Self, b: Self, factor: f32) -> Self {
        a + Self::shortest_distance(a, b) * factor as f64
    }

    pub fn shortest_distance(a: Self, b: Self) -> Self {
        let delta = (b - a) % core::f64::consts::TAU;
        2.0 * delta % core::f64::consts::TAU - delta
    }

    fn reduce(theta: f64) -> f64 {
        let turns = (theta / (2.0 * core::f64::consts::PI)) as i32;
        theta - turns as f64 * (2.0 * core::f64::consts::PI)
    }

    fn flip_positive_raw(theta: f64) -> f64 {
        if theta >= 0.0 {
            theta
        } else {
            theta + 2.0 * core::f64::consts::PI
        }
    }
}

impl ApproxEq for Angle {
    fn approx_eq(&self, other: Self) -> bool {
        let a = Self::flip_positive_raw(Self::reduce(self.theta));
        let b = Self::flip_positive_raw(Self::reduce(other.theta));
        MathHelper::close_to_percent_f64(a, b, 0.00001)
            || MathHelper::close_to_percent_f64(a + MathHelper::TWO_PI as f64, b, 0.00001)
            || MathHelper::close_to_percent_f64(a, b + MathHelper::TWO_PI as f64, 0.00001)
    }

    fn approx_eq_with_tolerance(&self, other: Self, tolerance: f64) -> bool {
        let a = Self::flip_positive_raw(Self::reduce(self.theta));
        let b = Self::flip_positive_raw(Self::reduce(other.theta));
        MathHelper::close_to_percent_f64(a, b, tolerance)
            || MathHelper::close_to_percent_f64(a + MathHelper::TWO_PI as f64, b, tolerance)
            || MathHelper::close_to_percent_f64(a, b + MathHelper::TWO_PI as f64, tolerance)
    }
}

impl From<f64> for Angle {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl From<f32> for Angle {
    fn from(value: f32) -> Self {
        Self::new(value as f64)
    }
}

impl From<Vector2> for Angle {
    fn from(value: Vector2) -> Self {
        Self::from_vec(value)
    }
}

impl From<Vector2i> for Angle {
    fn from(value: Vector2i) -> Self {
        Self::from_vec(value.into())
    }
}

impl fmt::Display for Angle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} rad", self.theta)
    }
}

impl Add for Angle {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.theta + rhs.theta)
    }
}

impl Add<f64> for Angle {
    type Output = Self;

    fn add(self, rhs: f64) -> Self::Output {
        Self::new(self.theta + rhs)
    }
}

impl Sub for Angle {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.theta - rhs.theta)
    }
}

impl Sub<f64> for Angle {
    type Output = Self;

    fn sub(self, rhs: f64) -> Self::Output {
        Self::new(self.theta - rhs)
    }
}

impl Neg for Angle {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.theta)
    }
}

impl Mul<f64> for Angle {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self::new(self.theta * rhs)
    }
}

impl Mul<Angle> for f64 {
    type Output = Angle;

    fn mul(self, rhs: Angle) -> Self::Output {
        Angle::new(self * rhs.theta)
    }
}

impl Rem<f64> for Angle {
    type Output = Self;

    fn rem(self, rhs: f64) -> Self::Output {
        Self::new(self.theta % rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::Angle;
    use crate::{ApproxEq, Direction, Vector2};

    #[test]
    fn direction_mapping_matches_expected_octants() {
        assert_eq!(Angle::ZERO.get_dir(), Direction::South);
        assert_eq!(
            Angle::from_world_vec(Vector2::new(0.0, -1.0)).get_dir(),
            Direction::South
        );
    }

    #[test]
    fn approximate_equality_wraps_around_two_pi() {
        let a = Angle::from(0.0_f64);
        let b = Angle::from(core::f64::consts::TAU);
        assert!(a.approx_eq(b));
    }
}
