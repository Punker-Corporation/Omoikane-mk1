use crate::{Box2, MathHelper, Vector2};
use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Circle {
    pub position: Vector2,
    pub radius: f32,
}

impl Circle {
    pub const fn new(position: Vector2, radius: f32) -> Self {
        Self { position, radius }
    }

    pub fn contains(self, point: Vector2) -> bool {
        self.contains_xy(point.x, point.y)
    }

    pub fn intersects_circle(self, circle: Self) -> bool {
        let dx = self.position.x - circle.position.x;
        let dy = self.position.y - circle.position.y;
        let sum_r = self.radius + circle.radius;
        dx * dx + dy * dy < sum_r * sum_r
    }

    pub fn intersects_box(self, box2: Box2) -> bool {
        let closest_x = MathHelper::median(box2.left, box2.right, self.position.x);
        let closest_y = MathHelper::median(box2.bottom, box2.top, self.position.y);
        self.contains_xy(closest_x, closest_y)
    }

    fn contains_xy(self, x: f32, y: f32) -> bool {
        let dx = self.position.x - x;
        let dy = self.position.y - y;
        let d2 = dx * dx + dy * dy;
        let r2 = self.radius * self.radius;
        d2 < r2 || MathHelper::close_to_percent(d2, r2, 0.00001)
    }
}

impl fmt::Display for Circle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Circle ({}, {}), {} r", self.position.x, self.position.y, self.radius)
    }
}

#[cfg(test)]
mod tests {
    use super::Circle;
    use crate::{Box2, Vector2};

    #[test]
    fn circle_intersections_match_expected_geometry() {
        let circle = Circle::new(Vector2::new(0.0, 0.0), 5.0);
        assert!(circle.contains(Vector2::new(3.0, 4.0)));
        assert!(circle.intersects_circle(Circle::new(Vector2::new(8.0, 0.0), 5.0)));
        assert!(circle.intersects_box(Box2::new(4.0, -1.0, 7.0, 1.0)));
    }
}
