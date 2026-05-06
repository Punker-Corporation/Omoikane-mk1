use crate::{Angle, ApproxEq, MathHelper, Vector2, Vector3};
use core::fmt;
use core::ops::{Index, IndexMut, Mul};

#[derive(Clone, Copy, Default, PartialEq)]
pub struct Matrix3 {
    pub r0c0: f32,
    pub r0c1: f32,
    pub r0c2: f32,
    pub r1c0: f32,
    pub r1c1: f32,
    pub r1c2: f32,
    pub r2c0: f32,
    pub r2c1: f32,
    pub r2c2: f32,
}

impl Matrix3 {
    pub const IDENTITY: Self = Self::new(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0);
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);

    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        r0c0: f32,
        r0c1: f32,
        r0c2: f32,
        r1c0: f32,
        r1c1: f32,
        r1c2: f32,
        r2c0: f32,
        r2c1: f32,
        r2c2: f32,
    ) -> Self {
        Self {
            r0c0,
            r0c1,
            r0c2,
            r1c0,
            r1c1,
            r1c2,
            r2c0,
            r2c1,
            r2c2,
        }
    }

    pub fn from_affine_basis(x: Vector2, y: Vector2, origin: Vector2) -> Self {
        Self::new(x.x, y.x, origin.x, x.y, y.y, origin.y, 0.0, 0.0, 1.0)
    }

    pub fn determinant(self) -> f32 {
        self.r0c0 * self.r1c1 * self.r2c2
            - self.r0c0 * self.r1c2 * self.r2c1
            - self.r0c1 * self.r1c0 * self.r2c2
            + self.r0c2 * self.r1c0 * self.r2c1
            + self.r0c1 * self.r1c2 * self.r2c0
            - self.r0c2 * self.r1c1 * self.r2c0
    }

    pub fn transpose(self) -> Self {
        Self::new(
            self.r0c0, self.r1c0, self.r2c0, self.r0c1, self.r1c1, self.r2c1, self.r0c2, self.r1c2,
            self.r2c2,
        )
    }

    pub fn inverted(self) -> Self {
        let d = self.determinant();
        if MathHelper::close_to_percent(d, 0.0, 0.000_001) {
            panic!("Matrix is singular and cannot be inverted.");
        }

        let invdet = 1.0 / d as f64;
        Self::new(
            ((self.r1c1 * self.r2c2 - self.r2c1 * self.r1c2) as f64 * invdet) as f32,
            ((self.r0c2 * self.r2c1 - self.r0c1 * self.r2c2) as f64 * invdet) as f32,
            ((self.r0c1 * self.r1c2 - self.r0c2 * self.r1c1) as f64 * invdet) as f32,
            ((self.r1c2 * self.r2c0 - self.r1c0 * self.r2c2) as f64 * invdet) as f32,
            ((self.r0c0 * self.r2c2 - self.r0c2 * self.r2c0) as f64 * invdet) as f32,
            ((self.r1c0 * self.r0c2 - self.r0c0 * self.r1c2) as f64 * invdet) as f32,
            ((self.r1c0 * self.r2c1 - self.r2c0 * self.r1c1) as f64 * invdet) as f32,
            ((self.r2c0 * self.r0c1 - self.r0c0 * self.r2c1) as f64 * invdet) as f32,
            ((self.r0c0 * self.r1c1 - self.r1c0 * self.r0c1) as f64 * invdet) as f32,
        )
    }

    pub fn create_translation(x: f32, y: f32) -> Self {
        let mut result = Self::IDENTITY;
        result.r0c2 = x;
        result.r1c2 = y;
        result
    }

    pub fn create_translation_vector(vector: Vector2) -> Self {
        Self::create_translation(vector.x, vector.y)
    }

    pub fn create_rotation(angle: Angle) -> Self {
        let cos = angle.theta.cos() as f32;
        let sin = angle.theta.sin() as f32;
        let mut result = Self::IDENTITY;
        result.r0c0 = cos;
        result.r1c0 = sin;
        result.r0c1 = -sin;
        result.r1c1 = cos;
        result
    }

    pub fn create_scale(x: f32, y: f32) -> Self {
        let mut result = Self::IDENTITY;
        result.r0c0 = x;
        result.r1c1 = y;
        result
    }

    pub fn create_scale_vector(scale: Vector2) -> Self {
        Self::create_scale(scale.x, scale.y)
    }

    pub fn create_transform(
        pos_x: f32,
        pos_y: f32,
        angle: f32,
        scale_x: f32,
        scale_y: f32,
    ) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self {
            r0c0: cos * scale_x,
            r0c1: -sin * scale_y,
            r0c2: pos_x,
            r1c0: sin * scale_x,
            r1c1: cos * scale_y,
            r1c2: pos_y,
            r2c0: 0.0,
            r2c1: 0.0,
            r2c2: 1.0,
        }
    }

    pub fn create_inverse_transform(
        pos_x: f32,
        pos_y: f32,
        angle: f32,
        scale_x: f32,
        scale_y: f32,
    ) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self {
            r0c0: cos / scale_x,
            r0c1: sin / scale_x,
            r0c2: -(pos_x * cos + pos_y * sin) / scale_x,
            r1c0: -sin / scale_y,
            r1c1: cos / scale_y,
            r1c2: (pos_x * sin - pos_y * cos) / scale_y,
            r2c0: 0.0,
            r2c1: 0.0,
            r2c2: 1.0,
        }
    }

    pub fn transform_vector2(self, vector: Vector2) -> Vector2 {
        Vector2::new(
            self.r0c0 * vector.x + self.r0c1 * vector.y + self.r0c2,
            self.r1c0 * vector.x + self.r1c1 * vector.y + self.r1c2,
        )
    }

    pub fn transform_vector3(self, vector: Vector3) -> Vector3 {
        Vector3::new(
            self.r0c0 * vector.x + self.r0c1 * vector.y + self.r0c2 * vector.z,
            self.r1c0 * vector.x + self.r1c1 * vector.y + self.r1c2 * vector.z,
            self.r2c0 * vector.x + self.r2c1 * vector.y + self.r2c2 * vector.z,
        )
    }
}

impl Index<(usize, usize)> for Matrix3 {
    type Output = f32;

    fn index(&self, index: (usize, usize)) -> &Self::Output {
        match index {
            (0, 0) => &self.r0c0,
            (0, 1) => &self.r0c1,
            (0, 2) => &self.r0c2,
            (1, 0) => &self.r1c0,
            (1, 1) => &self.r1c1,
            (1, 2) => &self.r1c2,
            (2, 0) => &self.r2c0,
            (2, 1) => &self.r2c1,
            (2, 2) => &self.r2c2,
            _ => panic!("index out of range"),
        }
    }
}

impl IndexMut<(usize, usize)> for Matrix3 {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        match index {
            (0, 0) => &mut self.r0c0,
            (0, 1) => &mut self.r0c1,
            (0, 2) => &mut self.r0c2,
            (1, 0) => &mut self.r1c0,
            (1, 1) => &mut self.r1c1,
            (1, 2) => &mut self.r1c2,
            (2, 0) => &mut self.r2c0,
            (2, 1) => &mut self.r2c1,
            (2, 2) => &mut self.r2c2,
            _ => panic!("index out of range"),
        }
    }
}

impl Mul for Matrix3 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(
            self.r0c0 * rhs.r0c0 + self.r0c1 * rhs.r1c0 + self.r0c2 * rhs.r2c0,
            self.r0c0 * rhs.r0c1 + self.r0c1 * rhs.r1c1 + self.r0c2 * rhs.r2c1,
            self.r0c0 * rhs.r0c2 + self.r0c1 * rhs.r1c2 + self.r0c2 * rhs.r2c2,
            self.r1c0 * rhs.r0c0 + self.r1c1 * rhs.r1c0 + self.r1c2 * rhs.r2c0,
            self.r1c0 * rhs.r0c1 + self.r1c1 * rhs.r1c1 + self.r1c2 * rhs.r2c1,
            self.r1c0 * rhs.r0c2 + self.r1c1 * rhs.r1c2 + self.r1c2 * rhs.r2c2,
            self.r2c0 * rhs.r0c0 + self.r2c1 * rhs.r1c0 + self.r2c2 * rhs.r2c0,
            self.r2c0 * rhs.r0c1 + self.r2c1 * rhs.r1c1 + self.r2c2 * rhs.r2c1,
            self.r2c0 * rhs.r0c2 + self.r2c1 * rhs.r1c2 + self.r2c2 * rhs.r2c2,
        )
    }
}

impl Mul<Vector2> for Matrix3 {
    type Output = Vector2;

    fn mul(self, rhs: Vector2) -> Self::Output {
        self.transform_vector2(rhs)
    }
}

impl Mul<Vector3> for Matrix3 {
    type Output = Vector3;

    fn mul(self, rhs: Vector3) -> Self::Output {
        self.transform_vector3(rhs)
    }
}

impl ApproxEq for Matrix3 {
    fn approx_eq(&self, other: Self) -> bool {
        self.approx_eq_with_tolerance(other, 0.000_001)
    }

    fn approx_eq_with_tolerance(&self, other: Self, tolerance: f64) -> bool {
        (0..3).all(|row| {
            (0..3).all(|column| {
                MathHelper::close_to_f64(
                    self[(row, column)] as f64,
                    other[(row, column)] as f64,
                    tolerance,
                )
            })
        })
    }
}

impl fmt::Debug for Matrix3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "|{}, {}, {}|\n|{}, {}, {}|\n|{}, {}, {}|\n",
            self.r0c0,
            self.r0c1,
            self.r0c2,
            self.r1c0,
            self.r1c1,
            self.r1c2,
            self.r2c0,
            self.r2c1,
            self.r2c2
        )
    }
}

impl fmt::Display for Matrix3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::Matrix3;
    use crate::{Angle, ApproxEq, Vector2, Vector3};

    #[test]
    fn affine_transform_roundtrips_with_inverse() {
        let transform = Matrix3::create_transform(10.0, -5.0, 0.3, 2.0, 3.0);
        let inverse = Matrix3::create_inverse_transform(10.0, -5.0, 0.3, 2.0, 3.0);
        let point = Vector2::new(4.0, -2.0);
        let world = transform * point;
        let local = inverse * world;
        assert!(local.approx_eq(point));
    }

    #[test]
    fn matrix3_rotation_translation_and_inversion_match_expected_shape() {
        let rotation = Matrix3::create_rotation(Angle::from_degrees(90.0));
        let translated = Matrix3::create_translation(3.0, 4.0) * rotation;
        let rotated = rotation * Vector2::UNIT_X;
        assert!(rotated.approx_eq(Vector2::UNIT_Y));
        let inverted = translated.inverted();
        let origin = inverted * (translated * Vector2::new(2.0, 3.0));
        assert!(origin.approx_eq(Vector2::new(2.0, 3.0)));
        assert_eq!(
            Matrix3::IDENTITY * Vector3::new(1.0, 2.0, 3.0),
            Vector3::new(1.0, 2.0, 3.0)
        );
    }
}
