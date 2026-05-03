use crate::{MetaDataComponent, MetaDataComponentState, MetaDataFlags};
use jikan::GameTick;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MetaFlagRemoveAttemptEvent {
    pub cancelled: bool,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MetaDataSystem;

impl MetaDataSystem {
    pub fn get_state(&self, component: &MetaDataComponent) -> MetaDataComponentState {
        component.get_component_state()
    }

    pub fn handle_state(
        &self,
        component: &mut MetaDataComponent,
        state: MetaDataComponentState,
        tick: GameTick,
    ) {
        component.handle_component_state(state, tick);
    }

    pub fn add_flag(&self, component: &mut MetaDataComponent, flags: MetaDataFlags) {
        component.flags.insert(flags);
    }

    pub fn remove_flag(
        &self,
        component: &mut MetaDataComponent,
        flags: MetaDataFlags,
        event: MetaFlagRemoveAttemptEvent,
    ) -> bool {
        if !component.flags.contains(flags) || event.cancelled {
            return false;
        }

        component.flags.remove(flags);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{MetaDataSystem, MetaFlagRemoveAttemptEvent};
    use crate::{MetaDataComponent, MetaDataFlags};

    #[test]
    fn metadata_system_adds_and_removes_flags() {
        let system = MetaDataSystem;
        let mut component = MetaDataComponent::new();
        system.add_flag(&mut component, MetaDataFlags::ENTITY_SPECIFIC);
        assert!(component.flags.contains(MetaDataFlags::ENTITY_SPECIFIC));
        assert!(system.remove_flag(
            &mut component,
            MetaDataFlags::ENTITY_SPECIFIC,
            MetaFlagRemoveAttemptEvent::default(),
        ));
        assert!(!component.flags.contains(MetaDataFlags::ENTITY_SPECIFIC));
    }
}
