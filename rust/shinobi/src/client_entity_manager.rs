use keisan::Vector2i;
use sekai::{
    EntityManager, EntityUid, GameStateMapData, GridId, MapComponentState, MapGridComponentState,
    MetaDataComponentState, NetworkComponentMessage, PhysicsComponentState, RobustSerializer,
    SerializableComponentState, SerializedEntityState, Tile, TransformComponentState,
};
use daikoku::{EntityMessageType, MsgEntity};

pub struct ClientEntityManager {
    pub inner: EntityManager,
    queued_messages: Vec<(u32, MsgEntity)>,
    incoming_sequence: u32,
    pub received_component_messages: Vec<NetworkComponentMessage<(), (), String>>,
    pub received_system_messages: Vec<String>,
}

impl ClientEntityManager {
    pub fn new() -> Self {
        Self {
            inner: EntityManager::new(),
            queued_messages: Vec::new(),
            incoming_sequence: 0,
            received_component_messages: Vec::new(),
            received_system_messages: Vec::new(),
        }
    }

    pub fn create_entity(&mut self, prototype: Option<&str>, uid: EntityUid) -> EntityUid {
        self.inner.alloc_entity_external(uid, prototype);
        uid
    }

    pub fn entity_exists(&self, uid: EntityUid) -> bool {
        self.inner.entity_exists(uid)
    }

    pub fn handle_entity_network_message(&mut self, cur_server_tick: jikan::GameTick, message: MsgEntity) {
        if message.source_tick <= cur_server_tick {
            self.dispatch_msg_entity(message);
        } else {
            self.incoming_sequence += 1;
            self.queued_messages.push((self.incoming_sequence, message));
            self.queued_messages.sort_by(|a, b| {
                a.1.source_tick
                    .value
                    .cmp(&b.1.source_tick.value)
                    .then(a.0.cmp(&b.0))
            });
        }
    }

    pub fn tick_update(&mut self, cur_server_tick: jikan::GameTick) {
        let mut ready = Vec::new();
        let mut pending = Vec::new();
        for entry in self.queued_messages.drain(..) {
            if entry.1.source_tick <= cur_server_tick {
                ready.push(entry);
            } else {
                pending.push(entry);
            }
        }
        self.queued_messages = pending;
        ready.sort_by(|a, b| {
            a.1.source_tick
                .value
                .cmp(&b.1.source_tick.value)
                .then(a.0.cmp(&b.0))
        });
        for (_, message) in ready {
            self.dispatch_msg_entity(message);
        }
    }

    pub fn delete_entity(&mut self, uid: EntityUid) {
        self.inner.queue_delete_entity(uid);
        self.inner.flush_queued_deletions();
    }

    pub fn initialize_entity(&mut self, uid: EntityUid) {
        let _ = self.inner.initialize_entity(uid);
    }

    pub fn apply_serialized_entity_state(
        &mut self,
        serializer: &mut RobustSerializer,
        state: &SerializedEntityState,
    ) -> Option<EntityUid> {
        if !self.entity_exists(state.uid) {
            let prototype = state
                .component_changes
                .iter()
                .find(|change| change.net_id == 1)
                .and_then(|change| change.state.as_ref())
                .and_then(|component| {
                    serializer
                        .deserialize_component_state::<MetaDataComponentState>(component)
                        .ok()
                })
                .and_then(|meta| meta.prototype_id);
            self.create_entity(prototype.as_deref(), state.uid);
        }

        for change in &state.component_changes {
            if change.deleted {
                match change.net_id {
                    1 => {
                        self.inner.metadata.remove(&state.uid);
                    }
                    2 => {
                        self.inner.transforms.remove(&state.uid);
                    }
                    3 => {
                        self.inner.map_components.remove(&state.uid);
                    }
                    4 => {
                        self.inner.map_grid_components.remove(&state.uid);
                    }
                    5 => {
                        self.inner.physics.remove(&state.uid);
                    }
                    _ => {}
                }
                continue;
            }

            let Some(component_state) = change.state.as_ref() else {
                continue;
            };

            self.apply_component_state(serializer, state.uid, change.net_id, component_state);
        }

        Some(state.uid)
    }

    pub fn apply_map_data(&mut self, map_data: &GameStateMapData) {
        for (grid_id, datum) in &map_data.grid_data {
            let uid = self.ensure_grid_entity(*grid_id);
            let grid_component = self.inner.map_grid_components.entry(uid).or_insert_with(|| {
                let mut component = sekai::MapGridComponent::new();
                component.base.owner = uid;
                component
            });
            grid_component.grid_index = *grid_id;
            let grid = self.inner.map_grids.entry(uid).or_insert_with(|| {
                sekai::MapGrid::new(datum.coordinates.map_id, uid, *grid_id, grid_component.chunk_size)
            });
            grid.parent_map_id = datum.coordinates.map_id;
            grid.world_position = datum.coordinates.position;
            grid.world_rotation = datum.angle;

            if let Some(transform) = self.inner.transforms.get_mut(&uid) {
                transform.map_id = datum.coordinates.map_id;
                transform.local_position = datum.coordinates.position;
                transform.local_rotation = datum.angle;
                transform.rebuild_for_manager();
            }

            for chunk in &datum.chunk_data {
                if let Some(tile_data) = &chunk.tile_data {
                    self.apply_chunk_tiles(*grid_id, chunk.index, tile_data);
                } else if let Some(grid) = self.inner.map_grids.get_mut(&uid) {
                    grid.remove_chunk(chunk.index);
                }
            }
        }
    }

    fn ensure_grid_entity(&mut self, grid_id: GridId) -> EntityUid {
        if let Some((uid, _)) = self
            .inner
            .map_grid_components
            .iter()
            .find(|(_, component)| component.grid_index == grid_id)
        {
            return *uid;
        }

        let uid = self.inner.create_entity_uninitialized(None);
        let component = self.inner.map_grid_components.entry(uid).or_insert_with(|| {
            let mut component = sekai::MapGridComponent::new();
            component.base.owner = uid;
            component
        });
        component.grid_index = grid_id;
        uid
    }

    fn apply_chunk_tiles(&mut self, grid_id: GridId, chunk_index: Vector2i, tile_data: &[Tile]) {
        let Some(uid) = self
            .inner
            .map_grid_components
            .iter()
            .find(|(_, component)| component.grid_index == grid_id)
            .map(|(uid, _)| *uid)
        else {
            return;
        };

        let Some(grid_component) = self.inner.map_grid_components.get(&uid) else {
            return;
        };
        let chunk_size = grid_component.chunk_size.max(1) as i32;
        let Some(grid) = self.inner.map_grids.get_mut(&uid) else {
            return;
        };

        let mut changed = Vec::with_capacity(tile_data.len());
        for (offset, tile) in tile_data.iter().copied().enumerate() {
            let offset = offset as i32;
            let local_x = offset % chunk_size;
            let local_y = offset / chunk_size;
            let indices = Vector2i::new(
                chunk_index.x * chunk_size + local_x,
                chunk_index.y * chunk_size + local_y,
            );
            changed.push((indices, tile));
        }

        grid.set_tiles(&changed);
    }

    fn apply_component_state(
        &mut self,
        serializer: &mut RobustSerializer,
        uid: EntityUid,
        net_id: u16,
        component_state: &SerializableComponentState,
    ) {
        match net_id {
            1 => {
                if let Ok(state) =
                    serializer.deserialize_component_state::<MetaDataComponentState>(component_state)
                {
                    let metadata = self.inner.metadata.entry(uid).or_insert_with(|| {
                        let mut component = sekai::MetaDataComponent::new();
                        component.base.owner = uid;
                        component
                    });
                    metadata.handle_component_state(state, self.inner.current_tick);
                }
            }
            2 => {
                if let Ok(state) =
                    serializer.deserialize_component_state::<TransformComponentState>(component_state)
                {
                    let transform = self.inner.transforms.entry(uid).or_insert_with(|| {
                        let mut component = sekai::TransformComponent::new();
                        component.base.owner = uid;
                        component
                    });
                    transform.handle_transform_state(state);
                }
            }
            3 => {
                if let Ok(state) =
                    serializer.deserialize_component_state::<MapComponentState>(component_state)
                {
                    let map = self.inner.map_components.entry(uid).or_insert_with(|| {
                        let mut component = sekai::MapComponent::new();
                        component.base.owner = uid;
                        component
                    });
                    map.handle_map_state(state);
                }
            }
            4 => {
                if let Ok(state) =
                    serializer.deserialize_component_state::<MapGridComponentState>(component_state)
                {
                    let grid = self.inner.map_grid_components.entry(uid).or_insert_with(|| {
                        let mut component = sekai::MapGridComponent::new();
                        component.base.owner = uid;
                        component
                    });
                    grid.handle_map_grid_state(state);
                }
            }
            5 => {
                if let Ok(state) =
                    serializer.deserialize_component_state::<PhysicsComponentState>(component_state)
                {
                    let physics = self.inner.physics.entry(uid).or_insert_with(|| {
                        let mut component = sekai::PhysicsComponent::new();
                        component.base.owner = uid;
                        component
                    });
                    physics.handle_component_state(state);
                }
            }
            _ => {}
        }
    }

    fn dispatch_msg_entity(&mut self, message: MsgEntity) {
        match message.message_type {
            EntityMessageType::ComponentMessage => {
                self.received_component_messages.push(NetworkComponentMessage::new(
                    (),
                    message.entity_uid,
                    message.net_id,
                    message.component_message.unwrap_or_default(),
                    None::<()>,
                ));
            }
            EntityMessageType::SystemMessage => {
                if let Some(system_message) = message.system_message {
                    self.received_system_messages.push(system_message);
                }
            }
            EntityMessageType::Error => {}
        }
    }
}

impl Default for ClientEntityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::ClientEntityManager;
    use daikoku::{EntityMessageType, MsgEntity};
    use sekai::{
        ChunkDatum, EntityUid, GameStateMapData, GridDatum, GridId, MapCoordinates,
        MapId, MetaDataComponentState, RobustSerializer, SerializedComponentChange,
        SerializedEntityState, Tile, TileRenderFlag,
    };
    use keisan::{Angle, Vector2, Vector2i};

    #[test]
    fn client_entity_manager_creates_and_applies_serialized_entities() {
        let mut entities = ClientEntityManager::new();
        let mut serializer = RobustSerializer::new();
        let state = SerializedEntityState {
            uid: EntityUid::new(5),
            component_changes: vec![SerializedComponentChange::new(
                1,
                true,
                false,
                Some(
                    serializer
                        .serialize_component_state(&MetaDataComponentState {
                            name: Some("mob".to_string()),
                            description: None,
                            prototype_id: Some("mob".to_string()),
                        })
                        .unwrap(),
                ),
            )],
        };
        entities.apply_serialized_entity_state(&mut serializer, &state);
        assert!(entities.entity_exists(EntityUid::new(5)));
        assert_eq!(
            entities.inner.metadata.get(&EntityUid::new(5)).unwrap().prototype_id.as_deref(),
            Some("mob")
        );
    }

    #[test]
    fn client_entity_manager_applies_map_data_to_grids() {
        let mut entities = ClientEntityManager::new();
        entities.apply_map_data(&GameStateMapData {
            grid_data: std::iter::once((
                GridId::new(3),
                GridDatum {
                    coordinates: MapCoordinates::new(Vector2::new(1.0, 2.0), MapId::new(7)),
                    angle: Angle::ZERO,
                    chunk_data: vec![ChunkDatum::create_modified(
                        Vector2i::new(0, 0),
                        vec![Tile::new(1, TileRenderFlag(0), 0)],
                    )],
                },
            ))
            .collect(),
        });

        let uid = entities
            .inner
            .map_grid_components
            .iter()
            .find(|(_, component)| component.grid_index == GridId::new(3))
            .map(|(uid, _)| *uid)
            .unwrap();
        let grid = entities.inner.map_grids.get(&uid).unwrap();
        assert_eq!(grid.parent_map_id, MapId::new(7));
        assert_eq!(grid.get_tile_ref(Vector2i::new(0, 0)).tile.type_id, 1);
    }

    #[test]
    fn client_entity_manager_dispatches_entity_messages_in_tick_order() {
        let mut entities = ClientEntityManager::new();
        entities.handle_entity_network_message(
            jikan::GameTick::ZERO,
            MsgEntity {
                message_type: EntityMessageType::SystemMessage,
                system_message: Some("later".to_string()),
                component_message: None,
                entity_uid: EntityUid::new(1),
                net_id: 0,
                sequence: 2,
                source_tick: jikan::GameTick::new(3),
            },
        );
        entities.handle_entity_network_message(
            jikan::GameTick::ZERO,
            MsgEntity {
                message_type: EntityMessageType::ComponentMessage,
                system_message: None,
                component_message: Some("comp".to_string()),
                entity_uid: EntityUid::new(2),
                net_id: 7,
                sequence: 1,
                source_tick: jikan::GameTick::ZERO,
            },
        );
        assert_eq!(entities.received_component_messages.len(), 1);
        entities.tick_update(jikan::GameTick::new(3));
        assert_eq!(entities.received_system_messages, vec!["later".to_string()]);
    }
}
