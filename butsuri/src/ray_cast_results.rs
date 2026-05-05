use keisan::Vector2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayCastHit<T = usize> {
    pub distance: f32,
    pub hit_pos: Vector2,
    pub hit: T,
}

impl<T> RayCastHit<T> {
    pub fn new(distance: f32, hit_pos: Vector2, hit: T) -> Self {
        Self {
            distance,
            hit_pos,
            hit,
        }
    }
}
