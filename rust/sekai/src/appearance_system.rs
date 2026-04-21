use crate::{AppearanceComponent, AppearanceComponentState, EntityUid};
use std::collections::HashSet;

pub struct SharedAppearanceSystem {
    dirty_components: HashSet<EntityUid>,
}

impl SharedAppearanceSystem {
    pub fn new() -> Self {
        Self {
            dirty_components: HashSet::new(),
        }
    }

    pub fn mark_dirty(&mut self, component: &AppearanceComponent) {
        self.dirty_components.insert(component.base.owner);
    }

    pub fn mark_dirty_uid(&mut self, uid: EntityUid) {
        self.dirty_components.insert(uid);
    }

    pub fn get_state(&self, component: &AppearanceComponent) -> AppearanceComponentState {
        AppearanceComponentState {
            data: component.appearance_data.clone(),
        }
    }

    pub fn handle_state(&mut self, component: &mut AppearanceComponent, state: AppearanceComponentState) {
        component.handle_component_state(state);
        self.mark_dirty(component);
    }

    pub fn clear_component_dirty(&mut self, component: &mut AppearanceComponent) {
        component.clear_dirty();
        self.dirty_components.remove(&component.base.owner);
    }

    pub fn drain_dirty(&mut self) -> Vec<EntityUid> {
        self.dirty_components.drain().collect()
    }
}

impl Default for SharedAppearanceSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::SharedAppearanceSystem;
    use crate::AppearanceComponent;

    #[test]
    fn appearance_system_tracks_dirty_components_and_states() {
        let mut system = SharedAppearanceSystem::new();
        let mut component = AppearanceComponent::new();
        component.set_data("mode", 3u8);
        system.mark_dirty(&component);
        let state = system.get_state(&component);
        assert_eq!(state.data.len(), 1);
        let dirty = system.drain_dirty();
        assert_eq!(dirty.len(), 1);
        system.handle_state(&mut component, state);
        assert!(component.appearance_dirty);
    }
}
