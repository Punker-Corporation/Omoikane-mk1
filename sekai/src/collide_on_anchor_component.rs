use crate::Component;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollideOnAnchorComponentState {
    pub enable: bool,
}

#[derive(Debug, Clone)]
pub struct CollideOnAnchorComponent {
    pub base: Component,
    pub enable: bool,
}

impl CollideOnAnchorComponent {
    pub fn new() -> Self {
        Self {
            base: Component::new("CollideOnAnchor"),
            enable: false,
        }
    }

    pub fn get_component_state(&self) -> CollideOnAnchorComponentState {
        CollideOnAnchorComponentState {
            enable: self.enable,
        }
    }

    pub fn handle_component_state(&mut self, state: CollideOnAnchorComponentState) {
        self.enable = state.enable;
    }
}

impl Default for CollideOnAnchorComponent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{CollideOnAnchorComponent, CollideOnAnchorComponentState};

    #[test]
    fn collide_on_anchor_component_roundtrips_state() {
        let mut component = CollideOnAnchorComponent::new();
        component.enable = true;
        let state = component.get_component_state();
        let mut restored = CollideOnAnchorComponent::new();
        restored.handle_component_state(CollideOnAnchorComponentState {
            enable: state.enable,
        });
        assert!(restored.enable);
    }
}
