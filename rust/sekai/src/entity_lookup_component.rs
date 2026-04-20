use crate::Component;
use keisan::Box2;
use std::collections::HashMap;

use crate::EntityUid;

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
}

impl Default for EntityLookupComponent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::EntityLookupComponent;
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
}
