use crate::Component;
use keisan::Box2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::EntityUid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityLookupEntry {
    pub entity: EntityUid,
    pub bounds: Box2,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityLookupComponentState {
    pub entries: Vec<EntityLookupEntry>,
}

#[derive(Debug, Clone)]
pub struct EntityLookupComponent {
    pub base: Component,
    pub entities: HashMap<EntityUid, Box2>,
}

impl EntityLookupComponent {
    pub fn new() -> Self {
        Self {
            base: Component::new("EntityLookupComponent"),
            entities: HashMap::new(),
        }
    }

    pub fn add_or_update(&mut self, entity: EntityUid, bounds: Box2) {
        self.entities.insert(entity, bounds);
    }

    pub fn remove(&mut self, entity: EntityUid) {
        self.entities.remove(&entity);
    }

    pub fn clear(&mut self) {
        self.entities.clear();
    }

    pub fn get_component_state(&self) -> EntityLookupComponentState {
        let mut entries = self
            .entities
            .iter()
            .map(|(entity, bounds)| EntityLookupEntry {
                entity: *entity,
                bounds: *bounds,
            })
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| a.entity.cmp(&b.entity));
        EntityLookupComponentState { entries }
    }

    pub fn handle_component_state(&mut self, state: EntityLookupComponentState) {
        self.entities.clear();
        for entry in state.entries {
            self.entities.insert(entry.entity, entry.bounds);
        }
    }
}

impl Default for EntityLookupComponent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{EntityLookupComponent, EntityLookupComponentState};
    use crate::EntityUid;
    use keisan::Box2;

    #[test]
    fn lookup_component_tracks_entity_bounds() {
        let mut lookup = EntityLookupComponent::new();
        lookup.add_or_update(EntityUid::new(1), Box2::new(0.0, 0.0, 1.0, 1.0));
        assert_eq!(lookup.entities.len(), 1);
        lookup.remove(EntityUid::new(1));
        assert!(lookup.entities.is_empty());
    }

    #[test]
    fn lookup_component_roundtrips_state() {
        let mut lookup = EntityLookupComponent::new();
        lookup.add_or_update(EntityUid::new(1), Box2::new(0.0, 0.0, 1.0, 1.0));
        let state = lookup.get_component_state();
        let mut restored = EntityLookupComponent::new();
        restored.handle_component_state(EntityLookupComponentState {
            entries: state.entries,
        });
        assert_eq!(restored.entities.len(), 1);
        assert!(restored.entities.contains_key(&EntityUid::new(1)));
    }
}
