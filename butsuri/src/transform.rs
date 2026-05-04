use core::ops::Mul;
use keisan::{Angle, Vector2};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Quaternion2D {
    pub c: f32,
    pub s: f32,
}

impl Quaternion2D {
    pub fn new(angle: f32) -> Self {
        Self {
            c: angle.cos(),
            s: angle.sin(),
        }
    }

    pub fn from_angle(angle: Angle) -> Self {
        Self::new(angle.theta as f32)
    }

    pub fn angle(self) -> f32 {
        self.s.atan2(self.c)
    }

    pub fn rotate_vector(self, vector: Vector2) -> Vector2 {
        Vector2::new(
            self.c * vector.x - self.s * vector.y,
            self.s * vector.x + self.c * vector.y,
        )
    }

    pub fn inverse_rotate_vector(self, vector: Vector2) -> Vector2 {
        Vector2::new(
            self.c * vector.x + self.s * vector.y,
            -self.s * vector.x + self.c * vector.y,
        )
    }
}

impl Mul<Vector2> for Quaternion2D {
    type Output = Vector2;

    fn mul(self, rhs: Vector2) -> Self::Output {
        self.rotate_vector(rhs)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub position: Vector2,
    pub rotation: Quaternion2D,
}

impl Transform {
    pub fn new(position: Vector2, angle: f32) -> Self {
        Self {
            position,
            rotation: Quaternion2D::new(angle),
        }
    }

    pub fn from_angle(angle: f32) -> Self {
        Self::new(Vector2::ZERO, angle)
    }

    pub fn from_angle_type(position: Vector2, angle: Angle) -> Self {
        Self {
            position,
            rotation: Quaternion2D::from_angle(angle),
        }
    }

    pub fn transform_point(self, vector: Vector2) -> Vector2 {
        self.rotation * vector + self.position
    }

    pub fn inverse_transform_point(self, vector: Vector2) -> Vector2 {
        self.rotation.inverse_rotate_vector(vector - self.position)
    }

    pub fn combine_t(a: Self, b: Self) -> Self {
        Self {
            rotation: Quaternion2D {
                s: a.rotation.c * b.rotation.s - a.rotation.s * b.rotation.c,
                c: a.rotation.c * b.rotation.c + a.rotation.s * b.rotation.s,
            },
            position: a.rotation.inverse_rotate_vector(b.position - a.position),
        }
    }
}

impl Mul<Vector2> for Transform {
    type Output = Vector2;

    fn mul(self, rhs: Vector2) -> Self::Output {
        self.transform_point(rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::Transform;
    use keisan::Vector2;

    #[test]
    fn transform_roundtrips_vectors() {
        let transform = Transform::new(Vector2::new(5.0, 0.0), core::f32::consts::FRAC_PI_2);
        let world = transform * Vector2::UNIT_X;
        let local = transform.inverse_transform_point(world);
        assert!((local - Vector2::UNIT_X).length() < 0.0001);
    }
}
