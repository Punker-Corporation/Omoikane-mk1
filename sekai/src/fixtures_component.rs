use crate::Component;
use butsuri::{Fixture, Transform as PhysicsTransform};
use keisan::Box2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FixturesComponentState {
    pub fixtures: Vec<Fixture>,
}

#[derive(Debug, Clone)]
pub struct FixturesComponent {
    pub base: Component,
    pub fixtures: HashMap<String, Fixture>,
    pub serialized_fixtures: Vec<Fixture>,
}

impl FixturesComponent {
    pub fn new() -> Self {
        Self {
            base: Component::new("Fixtures"),
            fixtures: HashMap::new(),
            serialized_fixtures: Vec::new(),
        }
    }

    pub fn fixture_count(&self) -> usize {
        self.fixtures.len()
    }

    pub fn insert_fixture(&mut self, fixture: Fixture) -> Option<Fixture> {
        self.fixtures.insert(fixture.id.clone(), fixture)
    }

    pub fn remove_fixture(&mut self, id: &str) -> Option<Fixture> {
        self.fixtures.remove(id)
    }

    pub fn before_serialization(&mut self, skip_serialization: bool) {
        if !self.serialized_fixtures.is_empty() || skip_serialization {
            return;
        }

        self.serialized_fixtures
            .extend(self.fixtures.values().cloned());
    }

    pub fn get_component_state(&self) -> FixturesComponentState {
        let mut fixtures = self.fixtures.values().cloned().collect::<Vec<_>>();
        fixtures.sort_by(|a, b| a.id.cmp(&b.id));
        FixturesComponentState { fixtures }
    }

    pub fn handle_component_state(&mut self, state: FixturesComponentState) {
        self.fixtures.clear();
        self.serialized_fixtures.clear();
        for fixture in state.fixtures {
            self.fixtures.insert(fixture.id.clone(), fixture);
        }
    }

    pub fn compute_aabb(&self, transform: PhysicsTransform) -> Option<Box2> {
        let mut bounds: Option<Box2> = None;
        for fixture in self.fixtures.values() {
            let next = fixture.compute_aabb(transform);
            bounds = Some(match bounds {
                Some(existing) => existing.union(next),
                None => next,
            });
        }
        bounds
    }

    pub fn compute_hard_aabb(&self, transform: PhysicsTransform) -> Option<Box2> {
        let mut bounds: Option<Box2> = None;
        for fixture in self.fixtures.values() {
            if !fixture.hard {
                continue;
            }
            let next = fixture.compute_aabb(transform);
            bounds = Some(match bounds {
                Some(existing) => existing.union(next),
                None => next,
            });
        }
        bounds
    }
}

impl Default for FixturesComponent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{FixturesComponent, FixturesComponentState};
    use butsuri::{AabbShape, Fixture, PhysShape, Transform};
    use keisan::{Box2, Vector2};

    #[test]
    fn fixtures_component_aggregates_fixture_bounds() {
        let mut component = FixturesComponent::new();
        component.insert_fixture(Fixture::new(
            "main",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        ));
        let bounds = component
            .compute_aabb(Transform::new(Vector2::new(5.0, 2.0), 0.0))
            .unwrap();
        assert_eq!(bounds, Box2::new(4.0, 1.0, 6.0, 3.0));
    }

    #[test]
    fn fixtures_component_roundtrips_component_state() {
        let mut component = FixturesComponent::new();
        component.insert_fixture(Fixture::new(
            "main",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        ));
        let state = component.get_component_state();
        let mut restored = FixturesComponent::new();
        restored.handle_component_state(FixturesComponentState {
            fixtures: state.fixtures,
        });
        assert_eq!(restored.fixture_count(), 1);
        assert!(restored.fixtures.contains_key("main"));
    }
}
