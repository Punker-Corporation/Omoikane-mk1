use crate::serialization::SerializableComponentState;
use crate::{ComponentStateValue, EntityUid};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Default)]
pub struct EntityState {
    pub uid: EntityUid,
    pub component_changes: Vec<ComponentChange>,
}

impl EntityState {
    pub fn new(uid: EntityUid, component_changes: Vec<ComponentChange>) -> Self {
        Self {
            uid,
            component_changes,
        }
    }

    pub fn empty(&self) -> bool {
        self.component_changes.is_empty()
    }
}

#[derive(Clone, Default)]
pub struct ComponentChange {
    pub deleted: bool,
    pub created: bool,
    pub state: Option<ComponentStateValue>,
    pub net_id: u16,
}

impl ComponentChange {
    pub fn new(net_id: u16, created: bool, deleted: bool, state: Option<ComponentStateValue>) -> Self {
        Self {
            deleted,
            created,
            state,
            net_id,
        }
    }

    pub fn added(net_id: u16, state: Option<ComponentStateValue>) -> Self {
        Self::new(net_id, true, false, state)
    }

    pub fn changed(net_id: u16, state: ComponentStateValue) -> Self {
        Self::new(net_id, false, false, Some(state))
    }

    pub fn removed(net_id: u16) -> Self {
        Self::new(net_id, false, true, None)
    }
}

impl fmt::Debug for EntityState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EntityState")
            .field("uid", &self.uid)
            .field("component_changes_len", &self.component_changes.len())
            .finish()
    }
}

impl fmt::Debug for ComponentChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {}",
            if self.deleted { "D" } else { "C" },
            self.net_id,
            if self.state.is_some() { "SomeState" } else { "None" }
        )
    }
}

impl fmt::Display for ComponentChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {}",
            if self.deleted { "D" } else { "C" },
            self.net_id,
            if self.state.is_some() { "SomeState" } else { "None" }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SerializedEntityState {
    pub uid: EntityUid,
    pub component_changes: Vec<SerializedComponentChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SerializedComponentChange {
    pub deleted: bool,
    pub created: bool,
    pub state: Option<SerializableComponentState>,
    pub net_id: u16,
}

impl SerializedComponentChange {
    pub fn new(
        net_id: u16,
        created: bool,
        deleted: bool,
        state: Option<SerializableComponentState>,
    ) -> Self {
        Self {
            deleted,
            created,
            state,
            net_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SerializedComponentChange, SerializedEntityState};
    use crate::{serialization::SerializableComponentState, EntityUid};

    #[test]
    fn serialized_entity_state_keeps_serialized_component_payloads() {
        let state = SerializedEntityState {
            uid: EntityUid::new(42),
            component_changes: vec![SerializedComponentChange::new(
                7,
                true,
                false,
                Some(SerializableComponentState::new("T", vec![1, 2])),
            )],
        };
        assert_eq!(state.uid, EntityUid::new(42));
        assert_eq!(state.component_changes.len(), 1);
    }
}
