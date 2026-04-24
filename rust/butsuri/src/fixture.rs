use crate::{BodyType, CollisionRay, PhysShape, Transform};
use keisan::{Box2, MathHelper};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fixture {
    pub id: String,
    pub shape: PhysShape,
    pub friction: f32,
    pub restitution: f32,
    pub hard: bool,
    pub mass: f32,
    pub collision_layer: i32,
    pub collision_mask: i32,
    pub body_type: BodyType,
}

impl Fixture {
    pub fn new(id: impl Into<String>, shape: PhysShape) -> Self {
        Self {
            id: id.into(),
            shape,
            friction: 0.4,
            restitution: 0.0,
            hard: true,
            mass: 0.0,
            collision_layer: 0,
            collision_mask: 0,
            body_type: BodyType::Static,
        }
    }

    pub fn compute_aabb(&self, transform: Transform) -> Box2 {
        self.shape.compute_aabb(transform)
    }

    pub fn ray_cast(&self, transform: Transform, ray: CollisionRay) -> Option<(f32, keisan::Vector2)> {
        self.shape.ray_cast(transform, ray.into())
    }

    pub fn area(&self) -> f32 {
        match self.shape {
            PhysShape::Aabb(shape) => shape.local_bounds.width() * shape.local_bounds.height(),
            PhysShape::Circle(shape) => core::f32::consts::PI * shape.radius * shape.radius,
        }
    }

    pub fn approx_eq(&self, other: &Self) -> bool {
        self.hard == other.hard
            && self.collision_layer == other.collision_layer
            && self.collision_mask == other.collision_mask
            && self.shape == other.shape
            && self.id == other.id
            && MathHelper::close_to(self.mass, other.mass, 0.0001)
    }
}

#[cfg(test)]
mod tests {
    use super::Fixture;
    use crate::{AabbShape, PhysShape};
    use keisan::Box2;

    #[test]
    fn fixture_tracks_shape_area() {
        let fixture = Fixture::new("main", PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -2.0, 1.0, 2.0), 0.0)));
        assert_eq!(fixture.area(), 8.0);
    }
}
