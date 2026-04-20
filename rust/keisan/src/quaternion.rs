use crate::{Matrix3, Vector3, Vector4};
use core::fmt;
use core::ops::{Add, Mul, Sub};

#[derive(Clone, Copy, Default, PartialEq)]
pub struct Quaternion {
    pub xyz: Vector3,
    pub w: f32,
}

impl Quaternion {
    pub const IDENTITY: Self = Self::new(0.0, 0.0, 0.0, 1.0);

    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { xyz: Vector3::new(x, y, z), w }
    }

    pub const fn from_vector(vector: Vector3, w: f32) -> Self {
        Self { xyz: vector, w }
    }

    pub fn x(self) -> f32 {
        self.xyz.x
    }

    pub fn y(self) -> f32 {
        self.xyz.y
    }

    pub fn z(self) -> f32 {
        self.xyz.z
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn length_squared(self) -> f32 {
        self.w * self.w + self.xyz.length_squared()
    }

    pub fn normalized(self) -> Self {
        let scale = 1.0 / self.length();
        Self::from_vector(self.xyz * scale, self.w * scale)
    }

    pub fn normalize(&mut self) {
        *self = self.normalized();
    }

    pub fn conjugated(self) -> Self {
        Self::from_vector(-self.xyz, self.w)
    }

    pub fn inverted(self) -> Self {
        let length_sq = self.length_squared();
        if length_sq != 0.0 {
            let inv = 1.0 / length_sq;
            Self::from_vector(self.xyz * -inv, self.w * inv)
        } else {
            self
        }
    }

    pub fn to_axis_angle(self) -> Vector4 {
        let q = if self.w.abs() > 1.0 { self.normalized() } else { self };
        let angle = 2.0 * q.w.acos();
        let den = (1.0 - q.w * q.w).sqrt();
        if den > 0.0001 {
            Vector4::from((q.xyz / den, angle))
        } else {
            Vector4::from((Vector3::UNIT_X, angle))
        }
    }

    pub fn from_axis_angle(axis: Vector3, angle: f32) -> Self {
        if axis.length_squared() == 0.0 {
            return Self::IDENTITY;
        }

        let half = angle * 0.5;
        let normalized = axis.normalized();
        Self::from_vector(normalized * half.sin(), half.cos()).normalized()
    }

    pub fn dot(a: Self, b: Self) -> f32 {
        a.x() * b.x() + a.y() * b.y() + a.z() * b.z() + a.w * b.w
    }

    pub fn slerp(q1: Self, mut q2: Self, blend: f32) -> Self {
        if q1.length_squared() == 0.0 {
            return if q2.length_squared() == 0.0 { Self::IDENTITY } else { q2 };
        }

        if q2.length_squared() == 0.0 {
            return q1;
        }

        let mut cos_half_angle = Self::dot(q1, q2);
        if cos_half_angle >= 1.0 || cos_half_angle <= -1.0 {
            return q1;
        }

        if cos_half_angle < 0.0 {
            q2 = Self::from_vector(-q2.xyz, -q2.w);
            cos_half_angle = -cos_half_angle;
        }

        let (blend_a, blend_b) = if cos_half_angle < 0.99 {
            let half_angle = cos_half_angle.acos();
            let sin_half_angle = half_angle.sin();
            let inv = 1.0 / sin_half_angle;
            (((half_angle * (1.0 - blend)).sin()) * inv, ((half_angle * blend).sin()) * inv)
        } else {
            (1.0 - blend, blend)
        };

        let result = Self::from_vector(q1.xyz * blend_a + q2.xyz * blend_b, blend_a * q1.w + blend_b * q2.w);
        if result.length_squared() > 0.0 {
            result.normalized()
        } else {
            Self::IDENTITY
        }
    }

    pub fn angle(a: Self, b: Self) -> f32 {
        let f = Self::dot(a, b);
        f32::acos(f.abs().min(1.0)) * 2.0 * (180.0 / core::f32::consts::PI)
    }

    pub fn rotate_towards(from: Self, to: Self, max_degrees_delta: f32) -> Self {
        let num = Self::angle(from, to);
        if num == 0.0 {
            return to;
        }

        let t = 1.0f32.min(max_degrees_delta / num);
        Self::slerp(from, to, t)
    }

    pub fn look_rotation(mut forward: Vector3, up: Vector3) -> Self {
        forward = forward.normalized();
        let right = Vector3::cross(up, forward).normalized();
        let up = Vector3::cross(forward, right);

        let m00 = right.x;
        let m01 = right.y;
        let m02 = right.z;
        let m10 = up.x;
        let m11 = up.y;
        let m12 = up.z;
        let m20 = forward.x;
        let m21 = forward.y;
        let m22 = forward.z;

        let num8 = m00 + m11 + m22;
        if num8 > 0.0 {
            let num = (num8 + 1.0).sqrt();
            let w = num * 0.5;
            let num = 0.5 / num;
            return Self::new((m12 - m21) * num, (m20 - m02) * num, (m01 - m10) * num, w);
        }

        if m00 >= m11 && m00 >= m22 {
            let num7 = (1.0 + m00 - m11 - m22).sqrt();
            let num4 = 0.5 / num7;
            return Self::new(0.5 * num7, (m01 + m10) * num4, (m02 + m20) * num4, (m12 - m21) * num4);
        }

        if m11 > m22 {
            let num6 = (1.0 + m11 - m00 - m22).sqrt();
            let num3 = 0.5 / num6;
            return Self::new((m10 + m01) * num3, 0.5 * num6, (m21 + m12) * num3, (m20 - m02) * num3);
        }

        let num5 = (1.0 + m22 - m00 - m11).sqrt();
        let num2 = 0.5 / num5;
        Self::new((m20 + m02) * num2, (m21 + m12) * num2, 0.5 * num5, (m01 - m10) * num2)
    }

    pub fn to_euler_rad(rotation: Self) -> Vector3 {
        let sqw = rotation.w * rotation.w;
        let sqx = rotation.x() * rotation.x();
        let sqy = rotation.y() * rotation.y();
        let sqz = rotation.z() * rotation.z();
        let unit = sqx + sqy + sqz + sqw;
        let test = rotation.x() * rotation.w - rotation.y() * rotation.z();

        if test > 0.4995 * unit {
            return normalize_angles(Vector3::new(core::f32::consts::FRAC_PI_2, 2.0 * rotation.y().atan2(rotation.x()), 0.0) * (180.0 / core::f32::consts::PI));
        }

        if test < -0.4995 * unit {
            return normalize_angles(Vector3::new(-core::f32::consts::FRAC_PI_2, -2.0 * rotation.y().atan2(rotation.x()), 0.0) * (180.0 / core::f32::consts::PI));
        }

        let q = Self::new(rotation.w, rotation.z(), rotation.x(), rotation.y());
        let yaw = (2.0 * q.x() * q.w + 2.0 * q.y() * q.z()).atan2(1.0 - 2.0 * (q.z() * q.z() + q.w * q.w));
        let pitch = (2.0 * (q.x() * q.z() - q.w * q.y())).asin();
        let roll = (2.0 * q.x() * q.y() + 2.0 * q.z() * q.w).atan2(1.0 - 2.0 * (q.y() * q.y() + q.z() * q.z()));
        normalize_angles(Vector3::new(pitch, yaw, roll) * (180.0 / core::f32::consts::PI))
    }

    pub fn from_matrix3(matrix: Matrix3) -> Self {
        let scale = (matrix.determinant() as f64).powf(1.0 / 3.0);
        let mut w = ((0.0f64).max(scale + matrix[(0, 0)] as f64 + matrix[(1, 1)] as f64 + matrix[(2, 2)] as f64).sqrt() / 2.0) as f32;
        let mut x = ((0.0f64).max(scale + matrix[(0, 0)] as f64 - matrix[(1, 1)] as f64 - matrix[(2, 2)] as f64).sqrt() / 2.0) as f32;
        let mut y = ((0.0f64).max(scale - matrix[(0, 0)] as f64 + matrix[(1, 1)] as f64 - matrix[(2, 2)] as f64).sqrt() / 2.0) as f32;
        let mut z = ((0.0f64).max(scale - matrix[(0, 0)] as f64 - matrix[(1, 1)] as f64 + matrix[(2, 2)] as f64).sqrt() / 2.0) as f32;
        if matrix[(2, 1)] - matrix[(1, 2)] < 0.0 { x = -x; }
        if matrix[(0, 2)] - matrix[(2, 0)] < 0.0 { y = -y; }
        if matrix[(1, 0)] - matrix[(0, 1)] < 0.0 { z = -z; }
        if w.is_nan() { w = 1.0; }
        Self::new(x, y, z, w)
    }
}

fn normalize_angle(angle: f32) -> f32 {
    angle - (angle * (1.0 / 360.0)).floor() * 360.0
}

fn normalize_angles(angles: Vector3) -> Vector3 {
    Vector3::new(normalize_angle(angles.x), normalize_angle(angles.y), normalize_angle(angles.z))
}

impl Add for Quaternion {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::from_vector(self.xyz + rhs.xyz, self.w + rhs.w)
    }
}

impl Sub for Quaternion {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::from_vector(self.xyz - rhs.xyz, self.w - rhs.w)
    }
}

impl Mul for Quaternion {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::from_vector(
            self.xyz * rhs.w + rhs.xyz * self.w + Vector3::cross(self.xyz, rhs.xyz),
            self.w * rhs.w - Vector3::dot(self.xyz, rhs.xyz),
        )
    }
}

impl Mul<f32> for Quaternion {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x() * rhs, self.y() * rhs, self.z() * rhs, self.w * rhs)
    }
}

impl fmt::Debug for Quaternion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "V: {}, W: {}", self.xyz, self.w)
    }
}

impl fmt::Display for Quaternion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::Quaternion;
    use crate::{ApproxEq, MathHelper, Vector3};

    #[test]
    fn axis_angle_roundtrip_stays_close() {
        let q = Quaternion::from_axis_angle(Vector3::UNIT_Z, core::f32::consts::FRAC_PI_2);
        let axis_angle = q.to_axis_angle();
        assert!(axis_angle.xyz().approx_eq_with_tolerance(Vector3::UNIT_Z, 0.0001));
        assert!(MathHelper::close_to(axis_angle.w, core::f32::consts::FRAC_PI_2, 0.0001));
    }

    #[test]
    fn slerp_and_rotate_towards_progress_toward_target() {
        let a = Quaternion::IDENTITY;
        let b = Quaternion::from_axis_angle(Vector3::UNIT_Y, core::f32::consts::PI);
        let half = Quaternion::slerp(a, b, 0.5);
        let stepped = Quaternion::rotate_towards(a, b, 90.0);
        assert!(Quaternion::angle(a, half) > 0.0);
        assert!(Quaternion::angle(a, stepped) <= 90.0001);
    }
}
