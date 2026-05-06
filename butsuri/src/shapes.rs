use crate::{Ray, Transform};
use keisan::{Box2, Vector2};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShapeType {
    Aabb,
    Circle,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AabbShape {
    pub local_bounds: Box2,
    pub radius: f32,
}

impl AabbShape {
    pub fn new(local_bounds: Box2, radius: f32) -> Self {
        Self {
            local_bounds,
            radius,
        }
    }

    pub fn compute_aabb(self, transform: Transform) -> Box2 {
        let points = [
            transform.transform_point(self.local_bounds.bottom_left()),
            transform.transform_point(self.local_bounds.bottom_right()),
            transform.transform_point(self.local_bounds.top_left()),
            transform.transform_point(self.local_bounds.top_right()),
        ];
        points
            .into_iter()
            .fold(Box2::from_corners(points[0], points[0]), |acc, point| {
                acc.extend_to_contain(point)
            })
            .enlarged(self.radius)
    }

    pub fn ray_cast(self, transform: Transform, ray: Ray) -> Option<(f32, Vector2)> {
        let local_ray = Ray::new(
            transform.mul_t(ray.position),
            transform.rotation.mul_t(ray.direction),
        );
        let local_bounds = self.local_bounds.enlarged(self.radius);
        let (distance, local_hit) = local_ray.intersects(local_bounds)?;
        Some((distance, transform.transform_point(local_hit)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CircleShape {
    pub position: Vector2,
    pub radius: f32,
}

impl CircleShape {
    pub fn new(position: Vector2, radius: f32) -> Self {
        Self { position, radius }
    }

    pub fn compute_aabb(self, transform: Transform) -> Box2 {
        let world = transform.transform_point(self.position);
        Box2::new(
            world.x - self.radius,
            world.y - self.radius,
            world.x + self.radius,
            world.y + self.radius,
        )
    }

    pub fn ray_cast(self, transform: Transform, ray: Ray) -> Option<(f32, Vector2)> {
        let local_position = transform.mul_t(ray.position);
        let local_direction = transform.rotation.mul_t(ray.direction);
        let offset = local_position - self.position;
        let b = Vector2::dot(offset, local_direction);
        let c = Vector2::dot(offset, offset) - self.radius * self.radius;
        let discriminant = b * b - c;
        if discriminant < 0.0 {
            return None;
        }

        let sqrt_discriminant = discriminant.sqrt();
        let mut distance = -b - sqrt_discriminant;
        if distance < 0.0 {
            distance = -b + sqrt_discriminant;
        }
        if distance < 0.0 {
            return None;
        }

        let local_hit = local_position + local_direction * distance;
        Some((distance, transform.transform_point(local_hit)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PhysShape {
    Aabb(AabbShape),
    Circle(CircleShape),
}

impl PhysShape {
    pub fn shape_type(self) -> ShapeType {
        match self {
            Self::Aabb(_) => ShapeType::Aabb,
            Self::Circle(_) => ShapeType::Circle,
        }
    }

    pub fn compute_aabb(self, transform: Transform) -> Box2 {
        match self {
            Self::Aabb(shape) => shape.compute_aabb(transform),
            Self::Circle(shape) => shape.compute_aabb(transform),
        }
    }

    pub fn ray_cast(self, transform: Transform, ray: Ray) -> Option<(f32, Vector2)> {
        match self {
            Self::Aabb(shape) => shape.ray_cast(transform, ray),
            Self::Circle(shape) => shape.ray_cast(transform, ray),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AabbShape, CircleShape, PhysShape};
    use crate::{Ray, Transform};
    use keisan::{Box2, Vector2};

    #[test]
    fn shapes_compute_aabb_in_world_space() {
        let transform = Transform::new(Vector2::new(5.0, 5.0), 0.0);
        let aabb = PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.1));
        let circle = PhysShape::Circle(CircleShape::new(Vector2::new(2.0, 0.0), 1.0));
        assert_eq!(aabb.compute_aabb(transform), Box2::new(3.9, 3.9, 6.1, 6.1));
        assert_eq!(
            circle.compute_aabb(transform),
            Box2::new(6.0, 4.0, 8.0, 6.0)
        );
    }

    #[test]
    fn shapes_ray_cast_rotated_aabb_and_circle() {
        let rotated = PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0));
        let rotated_hit = rotated
            .ray_cast(
                Transform::new(Vector2::ZERO, core::f32::consts::FRAC_PI_4),
                Ray::new(Vector2::new(-5.0, 0.0), Vector2::UNIT_X),
            )
            .unwrap();
        assert!((rotated_hit.0 - (5.0 - core::f32::consts::SQRT_2)).abs() < 0.0001);
        assert!((rotated_hit.1.x + core::f32::consts::SQRT_2).abs() < 0.0001);

        let circle = PhysShape::Circle(CircleShape::new(Vector2::ZERO, 1.0));
        assert!(
            circle
                .ray_cast(
                    Transform::new(Vector2::ZERO, 0.0),
                    Ray::new(Vector2::new(-2.0, 1.1), Vector2::UNIT_X),
                )
                .is_none()
        );
    }
}
