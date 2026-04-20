use keisan::{Box2, MathHelper, Vector2};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    pub position: Vector2,
    pub direction: Vector2,
}

impl Ray {
    pub fn new(position: Vector2, direction: Vector2) -> Self {
        assert!(
            MathHelper::close_to_percent(direction.length_squared(), 1.0, 0.00001),
            "ray direction must be normalized"
        );
        Self { position, direction }
    }

    pub fn intersects(self, box2: Box2) -> Option<(f32, Vector2)> {
        let mut tmin = 0.0_f32;
        let mut tmax = f32::MAX;
        const EPSILON: f32 = 1.0E-07;

        if self.direction.x.abs() < EPSILON {
            if self.position.x < self.direction_min_x(box2) || self.position.x > self.direction_max_x(box2) {
                return None;
            }
        } else {
            let ood = 1.0 / self.direction.x;
            let mut t1 = (self.direction_min_x(box2) - self.position.x) * ood;
            let mut t2 = (self.direction_max_x(box2) - self.position.x) * ood;
            if t1 > t2 {
                core::mem::swap(&mut t1, &mut t2);
            }
            tmin = tmin.max(t1);
            tmax = tmax.min(t2);
            if tmin > tmax {
                return None;
            }
        }

        if self.direction.y.abs() < EPSILON {
            if self.position.y < self.direction_min_y(box2) || self.position.y > self.direction_max_y(box2) {
                return None;
            }
        } else {
            let ood = 1.0 / self.direction.y;
            let mut t1 = (self.direction_min_y(box2) - self.position.y) * ood;
            let mut t2 = (self.direction_max_y(box2) - self.position.y) * ood;
            if t1 > t2 {
                core::mem::swap(&mut t1, &mut t2);
            }
            tmin = tmin.max(t1);
            tmax = tmax.min(t2);
            if tmin > tmax {
                return None;
            }
        }

        let hit_pos = self.position + self.direction * tmin;
        Some((tmin, hit_pos))
    }

    fn direction_min_x(self, box2: Box2) -> f32 {
        box2.left.min(box2.right)
    }

    fn direction_max_x(self, box2: Box2) -> f32 {
        box2.left.max(box2.right)
    }

    fn direction_min_y(self, box2: Box2) -> f32 {
        box2.top.min(box2.bottom)
    }

    fn direction_max_y(self, box2: Box2) -> f32 {
        box2.top.max(box2.bottom)
    }
}

#[cfg(test)]
mod tests {
    use super::Ray;
    use keisan::{Box2, Vector2};

    #[test]
    fn ray_hits_axis_aligned_box() {
        let ray = Ray::new(Vector2::new(-5.0, 0.0), Vector2::new(1.0, 0.0));
        let hit = ray.intersects(Box2::new(0.0, -1.0, 2.0, 1.0)).unwrap();
        assert_eq!(hit.0, 5.0);
        assert_eq!(hit.1, Vector2::new(0.0, 0.0));
    }

    #[test]
    fn ray_misses_box_when_parallel_and_outside() {
        let ray = Ray::new(Vector2::new(-5.0, 5.0), Vector2::new(1.0, 0.0));
        assert!(ray.intersects(Box2::new(0.0, -1.0, 2.0, 1.0)).is_none());
    }
}
