use crate::Transform;
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
        Self { local_bounds, radius }
    }

    pub fn compute_aabb(self, transform: Transform) -> Box2 {
        let points = [
            transform.mul(self.local_bounds.bottom_left()),
            transform.mul(self.local_bounds.bottom_right()),
            transform.mul(self.local_bounds.top_left()),
            transform.mul(self.local_bounds.top_right()),
        ];
        points
            .into_iter()
            .fold(Box2::from_corners(points[0], points[0]), |acc, point| acc.extend_to_contain(point))
            .enlarged(self.radius)
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
        let world = transform.mul(self.position);
        Box2::new(
            world.x - self.radius,
            world.y - self.radius,
            world.x + self.radius,
            world.y + self.radius,
        )
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
}

#[cfg(test)]
mod tests {
    use super::{AabbShape, CircleShape, PhysShape};
    use crate::Transform;
    use keisan::{Box2, Vector2};

    #[test]
    fn shapes_compute_aabb_in_world_space() {
        let transform = Transform::new(Vector2::new(5.0, 5.0), 0.0);
        let aabb = PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.1));
        let circle = PhysShape::Circle(CircleShape::new(Vector2::new(2.0, 0.0), 1.0));
        assert_eq!(aabb.compute_aabb(transform), Box2::new(3.9, 3.9, 6.1, 6.1));
        assert_eq!(circle.compute_aabb(transform), Box2::new(6.0, 4.0, 8.0, 6.0));
    }
}
