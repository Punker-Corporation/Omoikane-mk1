use crate::{ComponentMessage, ComponentStateValue, EntityUid};
use crate::component_event_args::IComponent;
use jikan::GameTick;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentLifeStage {
    PreAdd = 0,
    Adding,
    Added,
    Initializing,
    Initialized,
    Starting,
    Running,
    Stopping,
    Stopped,
    Removing,
    Deleted,
}

#[derive(Debug, Clone)]
pub struct Component {
    pub name: String,
    pub net_sync_enabled: bool,
    pub owner: EntityUid,
    pub life_stage: ComponentLifeStage,
    pub creation_tick: GameTick,
    pub last_modified_tick: GameTick,
}

impl Component {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            net_sync_enabled: true,
            owner: EntityUid::INVALID,
            life_stage: ComponentLifeStage::PreAdd,
            creation_tick: GameTick::ZERO,
            last_modified_tick: GameTick::ZERO,
        }
    }

    pub fn initialized(&self) -> bool {
        self.life_stage as i32 >= ComponentLifeStage::Initializing as i32
    }

    pub fn running(&self) -> bool {
        self.life_stage as i32 >= ComponentLifeStage::Starting as i32
            && self.life_stage as i32 <= ComponentLifeStage::Stopping as i32
    }

    pub fn deleted(&self) -> bool {
        self.life_stage as i32 >= ComponentLifeStage::Removing as i32
    }

    pub fn dirty(&mut self, tick: GameTick) {
        self.last_modified_tick = tick;
    }

    pub fn on_add(&mut self) {
        self.life_stage = ComponentLifeStage::Added;
    }

    pub fn initialize(&mut self) {
        self.life_stage = ComponentLifeStage::Initialized;
    }

    pub fn startup(&mut self) {
        self.life_stage = ComponentLifeStage::Running;
    }

    pub fn shutdown(&mut self) {
        self.life_stage = ComponentLifeStage::Stopped;
    }

    pub fn on_remove(&mut self) {
        self.life_stage = ComponentLifeStage::Deleted;
    }

    pub fn handle_network_message(&mut self, _message: ComponentMessage) {}

    pub fn get_component_state(&self) -> Option<ComponentStateValue> {
        None
    }

    pub fn handle_component_state(
        &mut self,
        _cur_state: Option<ComponentStateValue>,
        _next_state: Option<ComponentStateValue>,
    ) {
    }

    pub fn clear_ticks(&mut self) {
        self.last_modified_tick = GameTick::ZERO;
        self.creation_tick = GameTick::ZERO;
    }
}

impl IComponent for Component {
    fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ComponentAdd;
#[derive(Debug, Clone, Copy, Default)]
pub struct ComponentInit;
#[derive(Debug, Clone, Copy, Default)]
pub struct ComponentStartup;
#[derive(Debug, Clone, Copy, Default)]
pub struct ComponentShutdown;
#[derive(Debug, Clone, Copy, Default)]
pub struct ComponentRemove;

#[cfg(test)]
mod tests {
    use super::{Component, ComponentLifeStage};

    #[test]
    fn component_life_stage_transitions_work() {
        let mut comp = Component::new("meta");
        comp.on_add();
        assert_eq!(comp.life_stage, ComponentLifeStage::Added);
        comp.initialize();
        assert!(comp.initialized());
        comp.startup();
        assert!(comp.running());
        comp.shutdown();
        comp.on_remove();
        assert!(comp.deleted());
    }
}
