use crate::{Component, EntityLifeStage, MapId};
use jikan::GameTick;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaDataComponentState {
    pub name: Option<String>,
    pub description: Option<String>,
    pub prototype_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MetaDataFlags(pub u8);

impl MetaDataFlags {
    pub const NONE: Self = Self(0);
    pub const ENTITY_SPECIFIC: Self = Self(1 << 0);

    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }
}

#[derive(Debug, Clone)]
pub struct MetaDataComponent {
    pub base: Component,
    pub entity_name: Option<String>,
    pub entity_description: Option<String>,
    pub prototype_id: Option<String>,
    pub entity_last_modified_tick: GameTick,
    pub entity_life_stage: EntityLifeStage,
    pub flags: MetaDataFlags,
    pub visibility_mask: i32,
    pub entity_paused: bool,
    pub map_id: MapId,
}

impl Default for MetaDataComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl MetaDataComponent {
    pub fn new() -> Self {
        Self {
            base: Component::new("MetaDataComponent"),
            entity_name: None,
            entity_description: None,
            prototype_id: None,
            entity_last_modified_tick: GameTick::FIRST,
            entity_life_stage: EntityLifeStage::PreInit,
            flags: MetaDataFlags::NONE,
            visibility_mask: 0,
            entity_paused: false,
            map_id: MapId::NULLSPACE,
        }
    }

    pub fn entity_initialized(&self) -> bool {
        self.entity_life_stage as i32 >= EntityLifeStage::Initialized as i32
    }

    pub fn entity_initializing(&self) -> bool {
        self.entity_life_stage == EntityLifeStage::Initializing
    }

    pub fn entity_deleted(&self) -> bool {
        self.entity_life_stage as i32 >= EntityLifeStage::Deleted as i32
    }

    pub fn dirty(&mut self, tick: GameTick) {
        self.entity_last_modified_tick = tick;
    }

    pub fn set_name(&mut self, value: Option<String>, tick: GameTick) {
        if self.entity_name != value {
            self.entity_name = value;
            self.dirty(tick);
        }
    }

    pub fn set_description(&mut self, value: Option<String>, tick: GameTick) {
        if self.entity_description != value {
            self.entity_description = value;
            self.dirty(tick);
        }
    }

    pub fn set_prototype(&mut self, value: Option<String>, tick: GameTick) {
        if self.prototype_id != value {
            self.prototype_id = value;
            self.dirty(tick);
        }
    }

    pub fn get_component_state(&self) -> MetaDataComponentState {
        MetaDataComponentState {
            name: self.entity_name.clone(),
            description: self.entity_description.clone(),
            prototype_id: self.prototype_id.clone(),
        }
    }

    pub fn handle_component_state(&mut self, state: MetaDataComponentState, tick: GameTick) {
        self.entity_name = state.name;
        self.entity_description = state.description;
        self.prototype_id = state.prototype_id;
        self.dirty(tick);
    }
}

#[cfg(test)]
mod tests {
    use super::{MetaDataComponent, MetaDataFlags};
    use crate::EntityLifeStage;
    use jikan::GameTick;

    #[test]
    fn metadata_tracks_state_and_dirty_ticks() {
        let mut meta = MetaDataComponent::new();
        meta.set_name(Some("Omoikane".to_string()), GameTick::new(5));
        meta.flags = MetaDataFlags::ENTITY_SPECIFIC;
        meta.entity_life_stage = EntityLifeStage::Initialized;
        assert_eq!(meta.entity_name.as_deref(), Some("Omoikane"));
        assert_eq!(meta.entity_last_modified_tick, GameTick::new(5));
        assert!(meta.flags.contains(MetaDataFlags::ENTITY_SPECIFIC));
        assert!(meta.entity_initialized());
    }
}
