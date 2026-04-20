use keisan::{Angle, Vector2};

#[derive(Debug, Clone, Copy, PartialEq)]
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

    pub fn mul(self, vector: Vector2) -> Vector2 {
        Vector2::new(
            self.c * vector.x - self.s * vector.y,
            self.s * vector.x + self.c * vector.y,
        )
    }

    pub fn mul_t(self, vector: Vector2) -> Vector2 {
        Vector2::new(
            self.c * vector.x + self.s * vector.y,
            -self.s * vector.x + self.c * vector.y,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

    pub fn mul(self, vector: Vector2) -> Vector2 {
        self.rotation.mul(vector) + self.position
    }

    pub fn mul_t(self, vector: Vector2) -> Vector2 {
        self.rotation.mul_t(vector - self.position)
    }

    pub fn combine_t(a: Self, b: Self) -> Self {
        Self {
            rotation: Quaternion2D {
                s: a.rotation.c * b.rotation.s - a.rotation.s * b.rotation.c,
                c: a.rotation.c * b.rotation.c + a.rotation.s * b.rotation.s,
            },
            position: a.rotation.mul_t(b.position - a.position),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Transform;
    use keisan::Vector2;

    #[test]
    fn transform_roundtrips_vectors() {
        let transform = Transform::new(Vector2::new(5.0, 0.0), core::f32::consts::FRAC_PI_2);
        let world = transform.mul(Vector2::UNIT_X);
        let local = transform.mul_t(world);
        assert!((local - Vector2::UNIT_X).length() < 0.0001);
    }
}
