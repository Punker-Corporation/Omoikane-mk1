use crate::serialization::SerializableComponentState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentHandleState {
    pub current: Option<SerializableComponentState>,
    pub next: Option<SerializableComponentState>,
}

impl ComponentHandleState {
    pub fn new(
        current: Option<SerializableComponentState>,
        next: Option<SerializableComponentState>,
    ) -> Self {
        Self { current, next }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ComponentGetState {
    pub state: Option<SerializableComponentState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentGetStateAttemptEvent {
    pub player: String,
    pub cancelled: bool,
}

impl ComponentGetStateAttemptEvent {
    pub fn new(player: impl Into<String>) -> Self {
        Self {
            player: player.into(),
            cancelled: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ComponentGetState, ComponentGetStateAttemptEvent, ComponentHandleState};
    use crate::serialization::SerializableComponentState;

    #[test]
    fn component_state_events_store_serialized_state_payloads() {
        let payload = SerializableComponentState::new("TestState", vec![1, 2, 3]);
        let handle = ComponentHandleState::new(Some(payload.clone()), None);
        let get = ComponentGetState {
            state: Some(payload),
        };
        let attempt = ComponentGetStateAttemptEvent::new("session-1");
        assert!(handle.current.is_some());
        assert!(get.state.is_some());
        assert!(!attempt.cancelled);
    }
}
