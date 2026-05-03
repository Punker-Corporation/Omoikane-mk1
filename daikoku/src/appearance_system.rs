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

    pub fn mark_dirty(&mut self, entities: &mut ServerEntityManager, uid: EntityUid) -> bool {
        if !entities.inner.appearances.contains_key(&uid) {
            return false;
        }
        let tick = entities.inner.current_tick;
        if let Some(component) = entities.inner.appearances.get_mut(&uid) {
            component.base.last_modified_tick = tick;
            self.shared.mark_dirty(component);
        }
        entities.inner.dirty_entity(uid);
        true
    }

    pub fn get_state(
        &self,
        entities: &ServerEntityManager,
        uid: EntityUid,
    ) -> Option<AppearanceComponentState> {
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
        entities.set_current_tick(jikan::GameTick::new(4));
        assert!(system.mark_dirty(&mut entities, uid));
        let state = system.get_state(&entities, uid).unwrap();
        assert_eq!(state.data.len(), 1);
        assert_eq!(
            entities
                .inner
                .metadata
                .get(&uid)
                .unwrap()
                .entity_last_modified_tick,
            jikan::GameTick::new(4)
        );
        assert!(system.clear_dirty(&mut entities, uid));
    }
}
