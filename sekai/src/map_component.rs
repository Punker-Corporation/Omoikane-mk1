use crate::{Component, ComponentStateValue, MapId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct MapComponent {
    pub base: Component,
    pub world_map: MapId,
    pub lighting_enabled: bool,
    pub map_paused: bool,
    pub map_pre_init: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapComponentState {
    pub map_id: MapId,
    pub lighting_enabled: bool,
    pub map_paused: bool,
}

impl MapComponent {
    pub fn new() -> Self {
        Self {
            base: Component::new("MapComponent"),
            world_map: MapId::NULLSPACE,
            lighting_enabled: true,
            map_paused: false,
            map_pre_init: false,
        }
    }

    pub fn get_component_state(&self) -> MapComponentState {
        MapComponentState {
            map_id: self.world_map,
            lighting_enabled: self.lighting_enabled,
            map_paused: self.map_paused,
        }
    }

    pub fn handle_map_state(&mut self, state: MapComponentState) {
        self.world_map = state.map_id;
        self.lighting_enabled = state.lighting_enabled;
        self.map_paused = state.map_paused;
    }

    pub fn as_state_value(&self) -> ComponentStateValue {
        std::sync::Arc::new(self.get_component_state())
    }
}

#[cfg(test)]
mod tests {
    use super::{MapComponent, MapComponentState};
    use crate::MapId;

    #[test]
    fn map_component_tracks_network_state() {
        let mut component = MapComponent::new();
        component.handle_map_state(MapComponentState {
            map_id: MapId::new(5),
            lighting_enabled: false,
            map_paused: true,
        });
        assert_eq!(component.world_map, MapId::new(5));
        assert!(!component.lighting_enabled);
        assert!(component.map_paused);
    }
}
