use crate::{EntityUid, TransformComponent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityInitializedMessage {
    pub entity: EntityUid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityDeletedMessage {
    pub entity: EntityUid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityPausedEvent {
    pub entity: EntityUid,
    pub paused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityTerminatingEvent {
    pub entity: EntityUid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntParentChangedMessage {
    pub entity: EntityUid,
    pub old_parent: Option<EntityUid>,
}

#[derive(Debug, Clone)]
pub struct TransformStartLerpMessage {
    pub transform: TransformComponent,
}
