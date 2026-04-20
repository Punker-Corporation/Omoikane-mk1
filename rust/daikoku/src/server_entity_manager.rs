use crate::ActorComponent;
use jikan::GameTick;
use sekai::{
    EntityManager, MapComponentState, MapGridComponentState, MetaDataComponentState,
    NetworkComponentMessage, PhysicsComponentState, RobustSerializer, SerializedComponentChange,
    SerializedEntityState, TransformComponentState, EntityUid,
};
use std::collections::HashMap;

pub struct ServerEntityManager {
    pub inner: EntityManager,
    pub actors: HashMap<EntityUid, ActorComponent>,
    component_deletion_history: HashMap<EntityUid, Vec<(GameTick, u16)>>,
    pub received_component_messages: Vec<NetworkComponentMessage<(), String, String>>,
    pub received_system_messages: Vec<(String, String)>,
}

impl ServerEntityManager {
    pub fn new() -> Self {
        Self {
            inner: EntityManager::new(),
            actors: HashMap::new(),
            component_deletion_history: HashMap::new(),
            received_component_messages: Vec::new(),
            received_system_messages: Vec::new(),
        }
    }

    pub fn initialize(&mut self) {}

    pub fn alloc_entity(&mut self, prototype_name: Option<&str>, uid: EntityUid) -> EntityUid {
        self.inner.alloc_entity_external(uid, prototype_name);
        uid
    }

    pub fn create_entity(&mut self, prototype_name: Option<&str>) -> EntityUid {
        self.inner.create_entity_uninitialized(prototype_name)
    }

    pub fn initialize_entity(&mut self, entity: EntityUid) {
        let _ = self.inner.initialize_entity(entity);
    }

    pub fn delete_entity(&mut self, entity: EntityUid) {
        self.actors.remove(&entity);
        self.inner.queue_delete_entity(entity);
        self.inner.flush_queued_deletions();
    }

    pub fn note_component_removed(&mut self, uid: EntityUid, tick: GameTick, net_id: u16) {
        self.component_deletion_history
            .entry(uid)
            .or_default()
            .push((tick, net_id));
    }

    pub fn get_deleted_components(&self, uid: EntityUid, from_tick: GameTick) -> Vec<u16> {
        self.component_deletion_history
            .get(&uid)
            .map(|history| {
                history
                    .iter()
                    .filter_map(|(tick, net_id)| (*tick >= from_tick).then_some(*net_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn cull_deletion_history(&mut self, oldest_ack: GameTick) {
        self.component_deletion_history.retain(|_, history| {
            history.retain(|(tick, _)| *tick >= oldest_ack);
            !history.is_empty()
        });
    }

    pub fn build_entity_state(
        &mut self,
        serializer: &mut RobustSerializer,
        uid: EntityUid,
    ) -> Option<SerializedEntityState> {
        if !self.inner.entity_exists(uid) {
            return None;
        }

        let mut changes = Vec::new();

        if let Some(meta) = self.inner.metadata.get(&uid) {
            changes.push(SerializedComponentChange::new(
                1,
                false,
                false,
                Some(serializer.serialize_component_state::<MetaDataComponentState>(&meta.get_component_state()).ok()?),
            ));
        }

        if let Some(xform) = self.inner.transforms.get(&uid) {
            changes.push(SerializedComponentChange::new(
                2,
                false,
                false,
                Some(serializer.serialize_component_state::<TransformComponentState>(&xform.get_component_state()).ok()?),
            ));
        }

        if let Some(map) = self.inner.map_components.get(&uid) {
            changes.push(SerializedComponentChange::new(
                3,
                false,
                false,
                Some(serializer.serialize_component_state::<MapComponentState>(&map.get_component_state()).ok()?),
            ));
        }

        if let Some(grid) = self.inner.map_grid_components.get(&uid) {
            changes.push(SerializedComponentChange::new(
                4,
                false,
                false,
                Some(serializer.serialize_component_state::<MapGridComponentState>(&grid.get_component_state()).ok()?),
            ));
        }

        if let Some(physics) = self.inner.physics.get(&uid) {
            changes.push(SerializedComponentChange::new(
                5,
                false,
                false,
                Some(serializer.serialize_component_state::<PhysicsComponentState>(&physics.get_component_state()).ok()?),
            ));
        }

        Some(SerializedEntityState { uid, component_changes: changes })
    }

    pub fn receive_component_message(
        &mut self,
        user_id: impl Into<String>,
        uid: EntityUid,
        net_id: u32,
        payload: impl Into<String>,
    ) {
        self.received_component_messages.push(NetworkComponentMessage::new(
            (),
            uid,
            net_id,
            payload.into(),
            Some(user_id.into()),
        ));
    }

    pub fn receive_system_message(&mut self, user_id: impl Into<String>, payload: impl Into<String>) {
        self.received_system_messages.push((user_id.into(), payload.into()));
    }
}

impl Default for ServerEntityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::ServerEntityManager;
    use jikan::GameTick;
    use sekai::RobustSerializer;

    #[test]
    fn server_entity_manager_builds_serialized_entity_states() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.create_entity(Some("mob"));
        entities.initialize_entity(uid);
        let mut serializer = RobustSerializer::new();
        let state = entities.build_entity_state(&mut serializer, uid).unwrap();
        assert_eq!(state.uid, uid);
        entities.note_component_removed(uid, GameTick::new(5), 7);
        assert_eq!(entities.get_deleted_components(uid, GameTick::new(4)), vec![7]);
    }
}
