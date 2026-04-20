use crate::{CollisionRay, Fixture, RayCastHit, Transform};
use keisan::Box2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BroadphaseHit {
    pub fixture_index: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Broadphase {
    fixtures: Vec<(Fixture, Transform)>,
}

impl Broadphase {
    pub fn new() -> Self {
        Self { fixtures: Vec::new() }
    }

    pub fn insert(&mut self, fixture: Fixture, transform: Transform) -> usize {
        let index = self.fixtures.len();
        self.fixtures.push((fixture, transform));
        index
    }

    pub fn query_aabb(&self, aabb: Box2) -> Vec<BroadphaseHit> {
        self.fixtures
            .iter()
            .enumerate()
            .filter_map(|(index, (fixture, transform))| {
                fixture.compute_aabb(*transform).intersects(aabb).then_some(BroadphaseHit { fixture_index: index })
            })
            .collect()
    }

    pub fn query_ray(&self, ray: CollisionRay, max_length: f32, return_on_first_hit: bool) -> Vec<RayCastHit<BroadphaseHit>> {
        let mut results = Vec::new();
        for (index, (fixture, transform)) in self.fixtures.iter().enumerate() {
            let aabb = fixture.compute_aabb(*transform);
            if let Some((distance, hit_pos)) = ray.intersects(aabb) {
                if distance <= max_length {
                    results.push(RayCastHit::new(distance, hit_pos, BroadphaseHit { fixture_index: index }));
                    if return_on_first_hit {
                        break;
                    }
                }
            }
        }
        results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(core::cmp::Ordering::Equal));
        results
    }

    pub fn fixture(&self, index: usize) -> Option<&Fixture> {
        self.fixtures.get(index).map(|(fixture, _)| fixture)
    }
}

#[cfg(test)]
mod tests {
    use super::Broadphase;
    use crate::{AabbShape, CollisionRay, Fixture, PhysShape, Transform};
    use keisan::{Box2, Vector2};

    #[test]
    fn broadphase_queries_aabb_and_ray_hits() {
        let mut broadphase = Broadphase::new();
        broadphase.insert(
            Fixture::new("wall", PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0))),
            Transform::new(Vector2::new(5.0, 0.0), 0.0),
        );
        assert_eq!(broadphase.query_aabb(Box2::new(3.0, -2.0, 6.0, 2.0)).len(), 1);
        let hits = broadphase.query_ray(CollisionRay::new(Vector2::ZERO, Vector2::UNIT_X, -1), 10.0, true);
        assert_eq!(hits.len(), 1);
    }
}
