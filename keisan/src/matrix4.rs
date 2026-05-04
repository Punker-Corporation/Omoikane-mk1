use crate::{Matrix3, Quaternion, Vector3, Vector4};
use core::fmt;
use core::ops::Mul;

#[derive(Clone, Copy, Default, PartialEq)]
pub struct Matrix4 {
    pub row0: Vector4,
    pub row1: Vector4,
    pub row2: Vector4,
    pub row3: Vector4,
}

impl Matrix4 {
    pub const IDENTITY: Self = Self::new(
        Vector4::UNIT_X,
        Vector4::UNIT_Y,
        Vector4::UNIT_Z,
        Vector4::UNIT_W,
    );

    pub const fn new(row0: Vector4, row1: Vector4, row2: Vector4, row3: Vector4) -> Self {
        Self {
            row0,
            row1,
            row2,
            row3,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_values(
        m00: f32,
        m01: f32,
        m02: f32,
        m03: f32,
        m10: f32,
        m11: f32,
        m12: f32,
        m13: f32,
        m20: f32,
        m21: f32,
        m22: f32,
        m23: f32,
        m30: f32,
        m31: f32,
        m32: f32,
        m33: f32,
    ) -> Self {
        Self::new(
            Vector4::new(m00, m01, m02, m03),
            Vector4::new(m10, m11, m12, m13),
            Vector4::new(m20, m21, m22, m23),
            Vector4::new(m30, m31, m32, m33),
        )
    }

    pub fn determinant(self) -> f32 {
        self.row0.x * self.row1.y * self.row2.z * self.row3.w
            - self.row0.x * self.row1.y * self.row2.w * self.row3.z
            + self.row0.x * self.row1.z * self.row2.w * self.row3.y
            - self.row0.x * self.row1.z * self.row2.y * self.row3.w
            + self.row0.x * self.row1.w * self.row2.y * self.row3.z
            - self.row0.x * self.row1.w * self.row2.z * self.row3.y
            - self.row0.y * self.row1.z * self.row2.w * self.row3.x
            + self.row0.y * self.row1.z * self.row2.x * self.row3.w
            - self.row0.y * self.row1.w * self.row2.x * self.row3.z
            + self.row0.y * self.row1.w * self.row2.z * self.row3.x
            - self.row0.y * self.row1.x * self.row2.z * self.row3.w
            + self.row0.y * self.row1.x * self.row2.w * self.row3.z
            + self.row0.z * self.row1.w * self.row2.x * self.row3.y
            - self.row0.z * self.row1.w * self.row2.y * self.row3.x
            + self.row0.z * self.row1.x * self.row2.y * self.row3.w
            - self.row0.z * self.row1.x * self.row2.w * self.row3.y
            + self.row0.z * self.row1.y * self.row2.w * self.row3.x
            - self.row0.z * self.row1.y * self.row2.x * self.row3.w
            - self.row0.w * self.row1.x * self.row2.y * self.row3.z
            + self.row0.w * self.row1.x * self.row2.z * self.row3.y
            - self.row0.w * self.row1.y * self.row2.z * self.row3.x
            + self.row0.w * self.row1.y * self.row2.x * self.row3.z
            - self.row0.w * self.row1.z * self.row2.x * self.row3.y
            + self.row0.w * self.row1.z * self.row2.y * self.row3.x
    }

    pub fn column0(self) -> Vector4 {
        Vector4::new(self.row0.x, self.row1.x, self.row2.x, self.row3.x)
    }

    pub fn column1(self) -> Vector4 {
        Vector4::new(self.row0.y, self.row1.y, self.row2.y, self.row3.y)
    }

    pub fn column2(self) -> Vector4 {
        Vector4::new(self.row0.z, self.row1.z, self.row2.z, self.row3.z)
    }

    pub fn column3(self) -> Vector4 {
        Vector4::new(self.row0.w, self.row1.w, self.row2.w, self.row3.w)
    }

    pub fn transposed(self) -> Self {
        Self::new(
            self.column0(),
            self.column1(),
            self.column2(),
            self.column3(),
        )
    }

    pub fn create_from_axis_angle(mut axis: Vector3, angle: f32) -> Self {
        let cos = (-angle).cos();
        let sin = (-angle).sin();
        let t = 1.0 - cos;
        axis.normalize();
        Self::from_values(
            t * axis.x * axis.x + cos,
            t * axis.x * axis.y - sin * axis.z,
            t * axis.x * axis.z + sin * axis.y,
            0.0,
            t * axis.x * axis.y + sin * axis.z,
            t * axis.y * axis.y + cos,
            t * axis.y * axis.z - sin * axis.x,
            0.0,
            t * axis.x * axis.z - sin * axis.y,
            t * axis.y * axis.z + sin * axis.x,
            t * axis.z * axis.z + cos,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
        )
    }

    pub fn create_rotation_x(angle: f32) -> Self {
        let cos = angle.cos();
        let sin = angle.sin();
        Self::new(
            Vector4::UNIT_X,
            Vector4::new(0.0, cos, sin, 0.0),
            Vector4::new(0.0, -sin, cos, 0.0),
            Vector4::UNIT_W,
        )
    }

    pub fn create_rotation_y(angle: f32) -> Self {
        let cos = angle.cos();
        let sin = angle.sin();
        Self::new(
            Vector4::new(cos, 0.0, -sin, 0.0),
            Vector4::UNIT_Y,
            Vector4::new(sin, 0.0, cos, 0.0),
            Vector4::UNIT_W,
        )
    }

    pub fn create_rotation_z(angle: f32) -> Self {
        let cos = angle.cos();
        let sin = angle.sin();
        Self::new(
            Vector4::new(cos, sin, 0.0, 0.0),
            Vector4::new(-sin, cos, 0.0, 0.0),
            Vector4::UNIT_Z,
            Vector4::UNIT_W,
        )
    }

    pub fn create_translation(x: f32, y: f32, z: f32) -> Self {
        let mut result = Self::IDENTITY;
        result.row3 = Vector4::new(x, y, z, 1.0);
        result
    }

    pub fn create_translation_vector(translation: Vector3) -> Self {
        Self::create_translation(translation.x, translation.y, translation.z)
    }

    pub fn scale(x: f32, y: f32, z: f32) -> Self {
        Self::new(
            Vector4::UNIT_X * x,
            Vector4::UNIT_Y * y,
            Vector4::UNIT_Z * z,
            Vector4::UNIT_W,
        )
    }

    pub fn scale_vector(scale: Vector3) -> Self {
        Self::scale(scale.x, scale.y, scale.z)
    }

    pub fn rotate(q: Quaternion) -> Self {
        let axis_angle = q.to_axis_angle();
        Self::create_from_axis_angle(axis_angle.xyz(), axis_angle.w)
    }

    pub fn look_at(eye: Vector3, target: Vector3, up: Vector3) -> Self {
        let z = (eye - target).normalized();
        let x = Vector3::cross(up, z).normalized();
        let y = Vector3::cross(z, x).normalized();

        let rot = Self::new(
            Vector4::new(x.x, y.x, z.x, 0.0),
            Vector4::new(x.y, y.y, z.y, 0.0),
            Vector4::new(x.z, y.z, z.z, 0.0),
            Vector4::UNIT_W,
        );
        Self::create_translation(-eye.x, -eye.y, -eye.z) * rot
    }

    pub fn transform_vector4(self, vec: Vector4) -> Vector4 {
        Vector4::new(
            vec.x * self.row0.x + vec.y * self.row1.x + vec.z * self.row2.x + vec.w * self.row3.x,
            vec.x * self.row0.y + vec.y * self.row1.y + vec.z * self.row2.y + vec.w * self.row3.y,
            vec.x * self.row0.z + vec.y * self.row1.z + vec.z * self.row2.z + vec.w * self.row3.z,
            vec.x * self.row0.w + vec.y * self.row1.w + vec.z * self.row2.w + vec.w * self.row3.w,
        )
    }

    pub fn inverted(self) -> Self {
        let m41 = self.row3.x;
        let m42 = self.row3.y;
        let m43 = self.row3.z;
        let m44 = self.row3.w;

        if m41 == 0.0 && m42 == 0.0 && m43 == 0.0 && m44 == 1.0 {
            return self.invert_affine();
        }

        let d = self.determinant();
        if d == 0.0 {
            panic!("Matrix is singular and cannot be inverted.");
        }

        let d1 = 1.0 / d;
        let m11 = self.row0.x;
        let m12 = self.row0.y;
        let m13 = self.row0.z;
        let m14 = self.row0.w;
        let m21 = self.row1.x;
        let m22 = self.row1.y;
        let m23 = self.row1.z;
        let m24 = self.row1.w;
        let m31 = self.row2.x;
        let m32 = self.row2.y;
        let m33 = self.row2.z;
        let m34 = self.row2.w;

        Self::from_values(
            d1 * (m22 * m33 * m44 + m23 * m34 * m42 + m24 * m32 * m43
                - m22 * m34 * m43
                - m23 * m32 * m44
                - m24 * m33 * m42),
            d1 * (m12 * m34 * m43 + m13 * m32 * m44 + m14 * m33 * m42
                - m12 * m33 * m44
                - m13 * m34 * m42
                - m14 * m32 * m43),
            d1 * (m12 * m23 * m44 + m13 * m24 * m42 + m14 * m22 * m43
                - m12 * m24 * m43
                - m13 * m22 * m44
                - m14 * m23 * m42),
            d1 * (m12 * m24 * m33 + m13 * m22 * m34 + m14 * m23 * m32
                - m12 * m23 * m34
                - m13 * m24 * m32
                - m14 * m22 * m33),
            d1 * (m21 * m34 * m43 + m23 * m31 * m44 + m24 * m33 * m41
                - m21 * m33 * m44
                - m23 * m34 * m41
                - m24 * m31 * m43),
            d1 * (m11 * m33 * m44 + m13 * m34 * m41 + m14 * m31 * m43
                - m11 * m34 * m43
                - m13 * m31 * m44
                - m14 * m33 * m41),
            d1 * (m11 * m24 * m43 + m13 * m21 * m44 + m14 * m23 * m41
                - m11 * m23 * m44
                - m13 * m24 * m41
                - m14 * m21 * m43),
            d1 * (m11 * m23 * m34 + m13 * m24 * m31 + m14 * m21 * m33
                - m11 * m24 * m33
                - m13 * m21 * m34
                - m14 * m23 * m31),
            d1 * (m21 * m32 * m44 + m22 * m34 * m41 + m24 * m31 * m42
                - m21 * m34 * m42
                - m22 * m31 * m44
                - m24 * m32 * m41),
            d1 * (m11 * m34 * m42 + m12 * m31 * m44 + m14 * m32 * m41
                - m11 * m32 * m44
                - m12 * m34 * m41
                - m14 * m31 * m42),
            d1 * (m11 * m22 * m44 + m12 * m24 * m41 + m14 * m21 * m42
                - m11 * m24 * m42
                - m12 * m21 * m44
                - m14 * m22 * m41),
            d1 * (m11 * m24 * m32 + m12 * m21 * m34 + m14 * m22 * m31
                - m11 * m22 * m34
                - m12 * m24 * m31
                - m14 * m21 * m32),
            d1 * (m21 * m33 * m42 + m22 * m31 * m43 + m23 * m32 * m41
                - m21 * m32 * m43
                - m22 * m33 * m41
                - m23 * m31 * m42),
            d1 * (m11 * m32 * m43 + m12 * m33 * m41 + m13 * m31 * m42
                - m11 * m33 * m42
                - m12 * m31 * m43
                - m13 * m32 * m41),
            d1 * (m11 * m23 * m42 + m12 * m21 * m43 + m13 * m22 * m41
                - m11 * m22 * m43
                - m12 * m23 * m41
                - m13 * m21 * m42),
            d1 * (m11 * m22 * m33 + m12 * m23 * m31 + m13 * m21 * m32
                - m11 * m23 * m32
                - m12 * m21 * m33
                - m13 * m22 * m31),
        )
    }

    fn invert_affine(self) -> Self {
        let m11 = self.row0.x;
        let m12 = self.row0.y;
        let m13 = self.row0.z;
        let m14 = self.row0.w;
        let m21 = self.row1.x;
        let m22 = self.row1.y;
        let m23 = self.row1.z;
        let m24 = self.row1.w;
        let m31 = self.row2.x;
        let m32 = self.row2.y;
        let m33 = self.row2.z;
        let m34 = self.row2.w;

        let d = m11 * m22 * m33 + m21 * m32 * m13 + m31 * m12 * m23
            - m11 * m32 * m23
            - m31 * m22 * m13
            - m21 * m12 * m33;
        if d == 0.0 {
            panic!("Matrix is singular and cannot be inverted.");
        }

        let d1 = 1.0 / d;
        let row0 = Vector4::new(
            d1 * (m22 * m33 - m23 * m32),
            d1 * (m13 * m32 - m12 * m33),
            d1 * (m12 * m23 - m13 * m22),
            0.0,
        );
        let row1 = Vector4::new(
            d1 * (m23 * m31 - m21 * m33),
            d1 * (m11 * m33 - m13 * m31),
            d1 * (m13 * m21 - m11 * m23),
            0.0,
        );
        let row2 = Vector4::new(
            d1 * (m21 * m32 - m22 * m31),
            d1 * (m12 * m31 - m11 * m32),
            d1 * (m11 * m22 - m12 * m21),
            0.0,
        );
        let mut result = Self::new(row0, row1, row2, Vector4::new(0.0, 0.0, 0.0, 1.0));
        result.row0.w = -result.row0.x * m14 - result.row0.y * m24 - result.row0.z * m34;
        result.row1.w = -result.row1.x * m14 - result.row1.y * m24 - result.row1.z * m34;
        result.row2.w = -result.row2.x * m14 - result.row2.y * m24 - result.row2.z * m34;
        result
    }

    pub fn upper_left_matrix3(self) -> Matrix3 {
        Matrix3::new(
            self.row0.x,
            self.row0.y,
            self.row0.z,
            self.row1.x,
            self.row1.y,
            self.row1.z,
            self.row2.x,
            self.row2.y,
            self.row2.z,
        )
    }
}

impl Mul for Matrix4 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(
            Vector4::new(
                self.row0.x * rhs.row0.x
                    + self.row0.y * rhs.row1.x
                    + self.row0.z * rhs.row2.x
                    + self.row0.w * rhs.row3.x,
                self.row0.x * rhs.row0.y
                    + self.row0.y * rhs.row1.y
                    + self.row0.z * rhs.row2.y
                    + self.row0.w * rhs.row3.y,
                self.row0.x * rhs.row0.z
                    + self.row0.y * rhs.row1.z
                    + self.row0.z * rhs.row2.z
                    + self.row0.w * rhs.row3.z,
                self.row0.x * rhs.row0.w
                    + self.row0.y * rhs.row1.w
                    + self.row0.z * rhs.row2.w
                    + self.row0.w * rhs.row3.w,
            ),
            Vector4::new(
                self.row1.x * rhs.row0.x
                    + self.row1.y * rhs.row1.x
                    + self.row1.z * rhs.row2.x
                    + self.row1.w * rhs.row3.x,
                self.row1.x * rhs.row0.y
                    + self.row1.y * rhs.row1.y
                    + self.row1.z * rhs.row2.y
                    + self.row1.w * rhs.row3.y,
                self.row1.x * rhs.row0.z
                    + self.row1.y * rhs.row1.z
                    + self.row1.z * rhs.row2.z
                    + self.row1.w * rhs.row3.z,
                self.row1.x * rhs.row0.w
                    + self.row1.y * rhs.row1.w
                    + self.row1.z * rhs.row2.w
                    + self.row1.w * rhs.row3.w,
            ),
            Vector4::new(
                self.row2.x * rhs.row0.x
                    + self.row2.y * rhs.row1.x
                    + self.row2.z * rhs.row2.x
                    + self.row2.w * rhs.row3.x,
                self.row2.x * rhs.row0.y
                    + self.row2.y * rhs.row1.y
                    + self.row2.z * rhs.row2.y
                    + self.row2.w * rhs.row3.y,
                self.row2.x * rhs.row0.z
                    + self.row2.y * rhs.row1.z
                    + self.row2.z * rhs.row2.z
                    + self.row2.w * rhs.row3.z,
                self.row2.x * rhs.row0.w
                    + self.row2.y * rhs.row1.w
                    + self.row2.z * rhs.row2.w
                    + self.row2.w * rhs.row3.w,
            ),
            Vector4::new(
                self.row3.x * rhs.row0.x
                    + self.row3.y * rhs.row1.x
                    + self.row3.z * rhs.row2.x
                    + self.row3.w * rhs.row3.x,
                self.row3.x * rhs.row0.y
                    + self.row3.y * rhs.row1.y
                    + self.row3.z * rhs.row2.y
                    + self.row3.w * rhs.row3.y,
                self.row3.x * rhs.row0.z
                    + self.row3.y * rhs.row1.z
                    + self.row3.z * rhs.row2.z
                    + self.row3.w * rhs.row3.z,
                self.row3.x * rhs.row0.w
                    + self.row3.y * rhs.row1.w
                    + self.row3.z * rhs.row2.w
                    + self.row3.w * rhs.row3.w,
            ),
        )
    }
}

impl fmt::Debug for Matrix4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\n{}\n{}\n{}",
            self.row0, self.row1, self.row2, self.row3
        )
    }
}

impl fmt::Display for Matrix4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::Matrix4;
    use crate::{MathHelper, Quaternion, Vector3, Vector4};

    #[test]
    fn rotation_translation_and_scale_transform_vectors() {
        let translation = Matrix4::create_translation(2.0, 3.0, 4.0);
        let scale = Matrix4::scale(2.0, 3.0, 4.0);
        let combined = scale * translation;
        let transformed = combined.transform_vector4(Vector4::new(1.0, 1.0, 1.0, 1.0));
        assert_eq!(transformed, Vector4::new(4.0, 6.0, 8.0, 1.0));
    }

    #[test]
    fn matrix4_inversion_and_quaternion_rotation_work() {
        let rotation = Matrix4::rotate(Quaternion::from_axis_angle(
            Vector3::UNIT_Z,
            core::f32::consts::FRAC_PI_2,
        ));
        let inverse = rotation.inverted();
        let point = Vector4::new(1.0, 0.0, 0.0, 1.0);
        let rotated = rotation.transform_vector4(point);
        let unrotated = inverse.transform_vector4(rotated);
        assert!(MathHelper::close_to(unrotated.x, point.x, 0.0001));
        assert!(MathHelper::close_to(unrotated.y, point.y, 0.0001));
    }

    #[test]
    fn look_at_builds_non_singular_camera_matrix() {
        let look = Matrix4::look_at(Vector3::new(0.0, 0.0, 5.0), Vector3::ZERO, Vector3::UNIT_Y);
        assert!(look.determinant().abs() > 0.0001);
    }
}
