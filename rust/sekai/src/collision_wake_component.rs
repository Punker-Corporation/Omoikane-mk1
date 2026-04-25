use crate::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollisionWakeComponentState {
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct CollisionWakeComponent {
    pub base: Component,
    pub enabled: bool,
}

impl CollisionWakeComponent {
    pub fn new() -> Self {
        Self {
            base: Component::new("CollisionWake"),
            enabled: true,
        }
    }

    pub fn get_component_state(&self) -> CollisionWakeComponentState {
        CollisionWakeComponentState {
            enabled: self.enabled,
        }
    }

    pub fn handle_component_state(&mut self, state: CollisionWakeComponentState) {
        self.enabled = state.enabled;
    }
}

impl Default for CollisionWakeComponent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{CollisionWakeComponent, CollisionWakeComponentState};

    #[test]
    fn collision_wake_component_roundtrips_state() {
        let mut component = CollisionWakeComponent::new();
        component.enabled = false;
        let state = component.get_component_state();
        let mut restored = CollisionWakeComponent::new();
        restored.handle_component_state(CollisionWakeComponentState {
            enabled: state.enabled,
        });
        assert!(!restored.enabled);
    }
}
