use crate::ComponentLifeStage;
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
pub struct MapPausedEvent {
    pub entity: EntityUid,
    pub paused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityTerminatingEvent {
    pub entity: EntityUid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapInitEvent {
    pub entity: EntityUid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntParentChangedMessage {
    pub entity: EntityUid,
    pub old_parent: Option<EntityUid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicsInitializedEvent {
    pub uid: EntityUid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentLifecycleEvent {
    pub owner: EntityUid,
    pub component: String,
    pub stage: ComponentLifeStage,
}

#[derive(Debug, Clone)]
pub struct TransformStartLerpMessage {
    pub transform: TransformComponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicsWakeMessage {
    pub body: EntityUid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicsSleepMessage {
    pub body: EntityUid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionChangeMessage {
    pub owner: EntityUid,
    pub can_collide: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JointAddedEvent {
    pub body_a: EntityUid,
    pub body_b: EntityUid,
    pub joint_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JointRemovedEvent {
    pub body_a: EntityUid,
    pub body_b: EntityUid,
    pub joint_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityRuntimeEvent {
    EntityInitialized(EntityInitializedMessage),
    EntityDeleted(EntityDeletedMessage),
    EntityPaused(EntityPausedEvent),
    MapPaused(MapPausedEvent),
    EntityTerminating(EntityTerminatingEvent),
    MapInit(MapInitEvent),
    ComponentLifecycle(ComponentLifecycleEvent),
    ParentChanged(EntParentChangedMessage),
    AnchorStateChanged(crate::AnchorStateChangedEvent),
    PhysicsInitialized(PhysicsInitializedEvent),
}

impl From<EntityInitializedMessage> for EntityRuntimeEvent {
    fn from(value: EntityInitializedMessage) -> Self {
        Self::EntityInitialized(value)
    }
}

impl From<EntityDeletedMessage> for EntityRuntimeEvent {
    fn from(value: EntityDeletedMessage) -> Self {
        Self::EntityDeleted(value)
    }
}

impl From<EntityPausedEvent> for EntityRuntimeEvent {
    fn from(value: EntityPausedEvent) -> Self {
        Self::EntityPaused(value)
    }
}

impl From<MapPausedEvent> for EntityRuntimeEvent {
    fn from(value: MapPausedEvent) -> Self {
        Self::MapPaused(value)
    }
}

impl From<EntityTerminatingEvent> for EntityRuntimeEvent {
    fn from(value: EntityTerminatingEvent) -> Self {
        Self::EntityTerminating(value)
    }
}

impl From<MapInitEvent> for EntityRuntimeEvent {
    fn from(value: MapInitEvent) -> Self {
        Self::MapInit(value)
    }
}

impl From<ComponentLifecycleEvent> for EntityRuntimeEvent {
    fn from(value: ComponentLifecycleEvent) -> Self {
        Self::ComponentLifecycle(value)
    }
}

impl From<EntParentChangedMessage> for EntityRuntimeEvent {
    fn from(value: EntParentChangedMessage) -> Self {
        Self::ParentChanged(value)
    }
}

impl From<crate::AnchorStateChangedEvent> for EntityRuntimeEvent {
    fn from(value: crate::AnchorStateChangedEvent) -> Self {
        Self::AnchorStateChanged(value)
    }
}

impl From<PhysicsInitializedEvent> for EntityRuntimeEvent {
    fn from(value: PhysicsInitializedEvent) -> Self {
        Self::PhysicsInitialized(value)
    }
}
