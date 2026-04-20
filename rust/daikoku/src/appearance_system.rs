use crate::ServerEntityManager;
use sekai::{AppearanceComponentState, EntityUid, SharedAppearanceSystem};

#[derive(Default)]
pub struct AppearanceSystem {
    shared: SharedAppearanceSystem,
}

impl AppearanceSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark_dirty(&mut self, entities: &ServerEntityManager, uid: EntityUid) -> bool {
        let Some(component) = entities.inner.appearances.get(&uid) else {
            return false;
        };
        self.shared.mark_dirty(component);
        true
    }

    pub fn get_state(&self, entities: &ServerEntityManager, uid: EntityUid) -> Option<AppearanceComponentState> {
        let component = entities.inner.appearances.get(&uid)?;
        Some(self.shared.get_state(component))
    }

    pub fn clear_dirty(&mut self, entities: &mut ServerEntityManager, uid: EntityUid) -> bool {
        let Some(component) = entities.inner.appearances.get_mut(&uid) else {
            return false;
        };
        self.shared.clear_component_dirty(component);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::AppearanceSystem;
    use crate::ServerEntityManager;

    #[test]
    fn appearance_system_reads_component_state_from_server_entities() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.create_entity(None);
        entities.inner.ensure_appearance(uid).set_data("mode", 1u8);
        let mut system = AppearanceSystem::new();
        assert!(system.mark_dirty(&entities, uid));
        let state = system.get_state(&entities, uid).unwrap();
        assert_eq!(state.data.len(), 1);
        assert!(system.clear_dirty(&mut entities, uid));
    }
}
