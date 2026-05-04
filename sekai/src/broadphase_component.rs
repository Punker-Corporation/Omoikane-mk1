use crate::Component;
use butsuri::{Broadphase, BroadphaseEntry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BroadphaseComponentState {
    pub entries: Vec<BroadphaseEntry>,
}

#[derive(Debug, Clone)]
pub struct BroadphaseComponent {
    pub base: Component,
    pub tree: Broadphase,
}

impl BroadphaseComponent {
    pub fn new() -> Self {
        Self {
            base: Component::new("BroadphaseComponent"),
            tree: Broadphase::new(),
        }
    }

    pub fn get_component_state(&self) -> BroadphaseComponentState {
        BroadphaseComponentState {
            entries: self.tree.snapshot(),
        }
    }

    pub fn handle_component_state(&mut self, state: BroadphaseComponentState) {
        self.tree.rebuild_from_snapshot(state.entries);
    }
}

impl Default for BroadphaseComponent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::BroadphaseComponent;
    use butsuri::{AabbShape, Fixture, PhysShape, Transform};
    use keisan::{Box2, Vector2};

    #[test]
    fn broadphase_component_starts_empty() {
        let component = BroadphaseComponent::new();
        assert!(component.tree.query_aabb(Default::default()).is_empty());
    }

    #[test]
    fn broadphase_component_roundtrips_state() {
        let mut component = BroadphaseComponent::new();
        component.tree.insert(
            5,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
            Transform::new(Vector2::new(2.0, 0.0), 0.0),
        );
        let state = component.get_component_state();
        let mut restored = BroadphaseComponent::new();
        restored.handle_component_state(state);
        assert_eq!(
            restored
                .tree
                .query_aabb(Box2::new(0.0, -2.0, 4.0, 2.0))
                .len(),
            1
        );
    }
}
