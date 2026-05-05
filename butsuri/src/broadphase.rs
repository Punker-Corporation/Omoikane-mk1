use crate::{CollisionRay, Fixture, RayCastHit, Transform};
use keisan::Box2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BroadphaseHit {
    pub fixture_index: usize,
    pub owner_id: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BroadphaseEntry {
    pub owner_id: i32,
    pub fixture: Fixture,
    pub transform: Transform,
}

#[derive(Debug, Clone, Default)]
pub struct Broadphase {
    fixtures: Vec<BroadphaseEntry>,
}

impl Broadphase {
    pub fn new() -> Self {
        Self {
            fixtures: Vec::new(),
        }
    }

    pub fn insert(&mut self, owner_id: i32, fixture: Fixture, transform: Transform) -> usize {
        let index = self.fixtures.len();
        self.fixtures.push(BroadphaseEntry {
            owner_id,
            fixture,
            transform,
        });
        index
    }

    pub fn query_aabb(&self, aabb: Box2) -> Vec<BroadphaseHit> {
        self.fixtures
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                entry
                    .fixture
                    .compute_aabb(entry.transform)
                    .intersects(aabb)
                    .then_some(BroadphaseHit {
                        fixture_index: index,
                        owner_id: entry.owner_id,
                    })
            })
            .collect()
    }

    pub fn query_ray(
        &self,
        ray: CollisionRay,
        max_length: f32,
        return_on_first_hit: bool,
    ) -> Vec<RayCastHit<BroadphaseHit>> {
        let mut results = Vec::new();
        for (index, entry) in self.fixtures.iter().enumerate() {
            if !Self::ray_can_hit_fixture(ray, &entry.fixture) {
                continue;
            }
            let aabb = entry.fixture.compute_aabb(entry.transform);
            if ray.intersects(aabb).is_none() {
                continue;
            }
            if let Some((distance, hit_pos)) = entry.fixture.ray_cast(entry.transform, ray) {
                if distance <= max_length {
                    results.push(RayCastHit::new(
                        distance,
                        hit_pos,
                        BroadphaseHit {
                            fixture_index: index,
                            owner_id: entry.owner_id,
                        },
                    ));
                }
            }
        }
        results.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        if return_on_first_hit && results.len() > 1 {
            results.truncate(1);
        }
        results
    }

    pub fn fixture(&self, index: usize) -> Option<&Fixture> {
        self.fixtures.get(index).map(|entry| &entry.fixture)
    }

    pub fn entry(&self, index: usize) -> Option<&BroadphaseEntry> {
        self.fixtures.get(index)
    }

    pub fn snapshot(&self) -> Vec<BroadphaseEntry> {
        self.fixtures.clone()
    }

    pub fn rebuild_from_snapshot(&mut self, entries: Vec<BroadphaseEntry>) {
        self.fixtures = entries;
    }

    fn ray_can_hit_fixture(ray: CollisionRay, fixture: &Fixture) -> bool {
        ray.collision_mask == -1
            || fixture.collision_layer == 0
            || (ray.collision_mask & fixture.collision_layer) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::{Broadphase, BroadphaseEntry};
    use crate::{AabbShape, CollisionRay, Fixture, PhysShape, Transform};
    use keisan::{Box2, Vector2};

    #[test]
    fn broadphase_queries_aabb_and_ray_hits() {
        let mut broadphase = Broadphase::new();
        broadphase.insert(
            44,
            Fixture::new(
                "wall",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
            Transform::new(Vector2::new(5.0, 0.0), 0.0),
        );
        assert_eq!(
            broadphase.query_aabb(Box2::new(3.0, -2.0, 6.0, 2.0)).len(),
            1
        );
        let hits = broadphase.query_ray(
            CollisionRay::new(Vector2::ZERO, Vector2::UNIT_X, -1),
            10.0,
            true,
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].hit.owner_id, 44);
    }

    #[test]
    fn broadphase_query_ray_respects_collision_masks() {
        let mut broadphase = Broadphase::new();
        let mut first = Fixture::new(
            "wall",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        );
        first.collision_layer = 1 << 0;
        broadphase.insert(44, first, Transform::new(Vector2::new(5.0, 0.0), 0.0));

        let mut second = Fixture::new(
            "door",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        );
        second.collision_layer = 1 << 1;
        broadphase.insert(55, second, Transform::new(Vector2::new(8.0, 0.0), 0.0));

        let hits = broadphase.query_ray(
            CollisionRay::new(Vector2::ZERO, Vector2::UNIT_X, 1 << 1),
            10.0,
            false,
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].hit.owner_id, 55);
    }

    #[test]
    fn broadphase_query_ray_first_hit_uses_closest_distance_not_insertion_order() {
        let mut broadphase = Broadphase::new();
        broadphase.insert(
            44,
            Fixture::new(
                "far",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
            Transform::new(Vector2::new(8.0, 0.0), 0.0),
        );
        broadphase.insert(
            55,
            Fixture::new(
                "near",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
            Transform::new(Vector2::new(5.0, 0.0), 0.0),
        );

        let hits = broadphase.query_ray(
            CollisionRay::new(Vector2::ZERO, Vector2::UNIT_X, -1),
            10.0,
            true,
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].hit.owner_id, 55);
    }

    #[test]
    fn broadphase_query_ray_uses_exact_shape_hits_after_aabb_prefilter() {
        let mut broadphase = Broadphase::new();
        broadphase.insert(
            44,
            Fixture::new(
                "circle",
                PhysShape::Circle(crate::CircleShape::new(Vector2::ZERO, 1.0)),
            ),
            Transform::new(Vector2::ZERO, 0.0),
        );

        let hits = broadphase.query_ray(
            CollisionRay::new(Vector2::new(-2.0, 1.1), Vector2::UNIT_X, -1),
            10.0,
            false,
        );
        assert!(hits.is_empty());
    }

    #[test]
    fn broadphase_snapshot_roundtrips_entries() {
        let mut broadphase = Broadphase::new();
        broadphase.insert(
            77,
            Fixture::new(
                "wall",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
            Transform::new(Vector2::new(5.0, 0.0), 0.0),
        );
        let snapshot = broadphase.snapshot();
        let mut restored = Broadphase::new();
        restored.rebuild_from_snapshot(snapshot.clone());
        assert_eq!(snapshot.len(), 1);
        assert_eq!(restored.snapshot(), snapshot);
        assert_eq!(
            snapshot[0],
            BroadphaseEntry {
                owner_id: 77,
                fixture: Fixture::new(
                    "wall",
                    PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0))
                ),
                transform: Transform::new(Vector2::new(5.0, 0.0), 0.0),
            }
        );
    }
}
