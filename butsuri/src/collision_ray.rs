use crate::Ray;
use keisan::{Box2, Vector2};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionRay {
    ray: Ray,
    pub collision_mask: i32,
}

impl CollisionRay {
    pub fn new(position: Vector2, direction: Vector2, collision_mask: i32) -> Self {
        Self {
            ray: Ray::new(position, direction),
            collision_mask,
        }
    }

    pub fn position(self) -> Vector2 {
        self.ray.position
    }

    pub fn direction(self) -> Vector2 {
        self.ray.direction
    }

    pub fn intersects(self, box2: Box2) -> Option<(f32, Vector2)> {
        self.ray.intersects(box2)
    }
}

impl From<CollisionRay> for Ray {
    fn from(value: CollisionRay) -> Self {
        value.ray
    }
}
