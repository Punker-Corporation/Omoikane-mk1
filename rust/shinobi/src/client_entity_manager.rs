use keisan::Vector2i;
use sekai::{
    AppearanceComponentState, EntityManager, EntityUid, GameStateMapData, GridId,
    MapComponentState, MapGridComponentState, MetaDataComponentState, NetworkComponentMessage,
    PhysicsComponentState, RobustSerializer, SerializableComponentState, SerializedEntityState,
    Tile, TransformComponentState, FixturesComponentState, JointComponentState,
    EntityLookupComponentState, BroadphaseComponentState, SharedPhysicsMapComponentState,
};
use daikoku::{EntityMessageType, MsgEntity};
use keisan::{Angle, Vector2};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PendingTransformLerp {
    pub uid: EntityUid,
    pub source: Vector2,
    pub destination: Vector2,
    pub source_angle: Angle,
    pub destination_angle: Angle,
    pub parent: EntityUid,
    pub source_anchored: bool,
    pub destination_anchored: bool,
}

pub struct ClientEntityManager {
    pub inner: EntityManager,
    queued_messages: Vec<(u32, MsgEntity)>,
    incoming_sequence: u32,
    pending_transform_lerps: Vec<PendingTransformLerp>,
    pub received_component_messages: Vec<NetworkComponentMessage<(), (), String>>,
    pub received_system_messages: Vec<String>,
}

impl ClientEntityManager {
    pub fn new() -> Self {
        Self {
            inner: EntityManager::new(),
            queued_messages: Vec::new(),
            incoming_sequence: 0,
            pending_transform_lerps: Vec::new(),
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

    pub fn map_entity_for(&self, map_id: sekai::MapId) -> Option<EntityUid> {
        self.inner
            .map_components
            .iter()
            .find(|(_, component)| component.world_map == map_id)
            .map(|(uid, _)| *uid)
    }

    pub fn ensure_map_entity(&mut self, map_id: sekai::MapId) -> EntityUid {
        if let Some(uid) = self.map_entity_for(map_id) {
            return uid;
        }

        let uid = self.inner.create_entity_uninitialized(None);
        let map = self.inner.map_components.entry(uid).or_insert_with(|| {
            let mut component = sekai::MapComponent::new();
            component.base.owner = uid;
            component
        });
        map.world_map = map_id;
        if let Some(transform) = self.inner.transforms.get_mut(&uid) {
            transform.map_id = map_id;
            transform.parent = EntityUid::INVALID;
            transform.local_position = Vector2::ZERO;
            transform.local_rotation = Angle::ZERO;
            transform.grid_id = GridId::INVALID;
            transform.rebuild_for_manager();
        }
        uid
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

    pub fn take_pending_transform_lerps(&mut self) -> Vec<PendingTransformLerp> {
        std::mem::take(&mut self.pending_transform_lerps)
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
                        self.inner.remove_transform_component(state.uid);
                    }
                    3 => {
                        self.inner.remove_map_component(state.uid);
                    }
                    4 => {
                        self.inner.remove_map_grid_component(state.uid);
                    }
                    5 => {
                        self.inner.physics.remove(&state.uid);
                    }
                    6 => {
                        self.inner.appearances.remove(&state.uid);
                    }
                    7 => {
                        self.inner.fixtures.remove(&state.uid);
                    }
                    8 => {
                        self.inner.joint_components.remove(&state.uid);
                    }
                    9 => {
                        self.inner.entity_lookups.remove(&state.uid);
                    }
                    10 => {
                        self.inner.broadphases.remove(&state.uid);
                    }
                    11 => {
                        self.inner.physics_maps.remove(&state.uid);
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
        for grid_id in &map_data.deleted_grids {
            if let Some(uid) = self
                .inner
                .map_grid_components
                .iter()
                .find(|(_, component)| component.grid_index == *grid_id)
                .map(|(uid, _)| *uid)
            {
                self.delete_entity(uid);
            }
        }

        for (grid_id, datum) in &map_data.grid_data {
            let map_entity = self.ensure_map_entity(datum.coordinates.map_id);
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
                transform.grid_id = *grid_id;
                transform.local_position = datum.coordinates.position;
                transform.local_rotation = datum.angle;
                transform.rebuild_for_manager();
            }
            let _ = self.inner.set_parent(uid, map_entity);

            for chunk in &datum.chunk_data {
                if let Some(tile_data) = &chunk.tile_data {
                    self.apply_chunk_tiles(*grid_id, chunk.index, tile_data);
                } else if let Some(grid) = self.inner.map_grids.get_mut(&uid) {
                    grid.remove_chunk(chunk.index);
                }
            }
        }
    }

    pub fn rebuild_runtime_state(&mut self) {
        self.refresh_transform_spatial_metadata();
        self.refresh_grid_transforms();
        self.rebuild_lookup_runtime();
        self.rebuild_map_physics_runtime();
    }

    pub fn sync_map_physics_runtime(&mut self, map_id: sekai::MapId) {
        let Some(owner) = self.map_entity_for(map_id) else {
            return;
        };

        let bodies = self
            .inner
            .physics
            .iter()
            .filter_map(|(uid, body)| {
                if !body.can_collide {
                    return None;
                }
                let transform = self.inner.transforms.get(uid)?;
                let fixtures = self.inner.fixtures.get(uid)?;
                if transform.map_id != map_id || fixtures.fixtures.is_empty() {
                    return None;
                }
                Some(*uid)
            })
            .collect::<Vec<_>>();

        self.inner.ensure_broadphase(owner);
        sekai::SharedPhysicsSystem.sync_broadphase(&mut self.inner, owner, &bodies);

        let awake = bodies
            .iter()
            .copied()
            .filter(|uid| self.inner.physics.get(uid).is_some_and(|body| body.awake))
            .collect::<std::collections::HashSet<_>>();
        let body_set = bodies.iter().copied().collect::<std::collections::HashSet<_>>();
        let physics_map = self.inner.ensure_physics_map(owner);
        physics_map.bodies = body_set;
        physics_map.awake_bodies = awake;
        let _ = sekai::SharedPhysicsSystem.sync_contacts(&mut self.inner, owner, &bodies);
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

    fn refresh_grid_transforms(&mut self) {
        let grid_uids = self.inner.map_grids.keys().copied().collect::<Vec<_>>();
        for uid in grid_uids {
            let Some(transform) = self.inner.transforms.get(&uid) else {
                continue;
            };
            let (world_position, world_rotation, _) =
                transform.get_world_position_rotation_matrix(&self.inner);
            if let Some(grid) = self.inner.map_grids.get_mut(&uid) {
                grid.world_position = world_position;
                grid.world_rotation = world_rotation;
                grid.parent_map_id = transform.map_id;
            }
        }
    }

    fn refresh_transform_spatial_metadata(&mut self) {
        let roots = self
            .inner
            .transforms
            .iter()
            .filter_map(|(uid, transform)| (!transform.parent.is_valid()).then_some(*uid))
            .collect::<Vec<_>>();
        for root in roots {
            self.propagate_transform_spatial_metadata(root, sekai::MapId::NULLSPACE, sekai::GridId::INVALID);
        }
    }

    fn propagate_transform_spatial_metadata(
        &mut self,
        uid: EntityUid,
        inherited_map: sekai::MapId,
        inherited_grid: GridId,
    ) {
        let (parent_valid, current_map, current_grid) = self
            .inner
            .transforms
            .get(&uid)
            .map(|transform| (transform.parent.is_valid(), transform.map_id, transform.grid_id))
            .unwrap_or((false, sekai::MapId::NULLSPACE, GridId::INVALID));
        let mut map_id = if current_map != sekai::MapId::NULLSPACE {
            current_map
        } else {
            inherited_map
        };
        let mut grid_id = if current_grid.is_valid() {
            current_grid
        } else {
            inherited_grid
        };

        if let Some(map) = self.inner.map_components.get(&uid) {
            map_id = map.world_map;
            grid_id = GridId::INVALID;
        } else if let Some(grid) = self.inner.map_grid_components.get(&uid) {
            if map_id == sekai::MapId::NULLSPACE {
                map_id = inherited_map;
            }
            grid_id = grid.grid_index;
        } else if parent_valid {
            map_id = inherited_map;
            grid_id = inherited_grid;
        }

        if let Some(transform) = self.inner.transforms.get_mut(&uid) {
            transform.map_id = map_id;
            transform.grid_id = grid_id;
        }

        let children = self
            .inner
            .transforms
            .iter()
            .filter_map(|(child, transform)| (transform.parent == uid).then_some(*child))
            .collect::<Vec<_>>();
        for child in children {
            self.propagate_transform_spatial_metadata(child, map_id, grid_id);
        }
    }

    fn rebuild_lookup_runtime(&mut self) {
        for lookup in self.inner.entity_lookups.values_mut() {
            lookup.clear();
        }

        let roots = self
            .inner
            .transforms
            .iter()
            .filter_map(|(uid, transform)| (!transform.parent.is_valid()).then_some(*uid))
            .collect::<Vec<_>>();
        let lookup = sekai::EntityLookupSystem;
        for root in roots {
            lookup.update_subtree_bounds(&mut self.inner, root);
        }
    }

    fn rebuild_map_physics_runtime(&mut self) {
        let map_owners = self.inner.map_components.keys().copied().collect::<Vec<_>>();
        for owner in map_owners {
            let Some(map_id) = self.inner.map_components.get(&owner).map(|component| component.world_map) else {
                continue;
            };
            self.sync_map_physics_runtime(map_id);
        }
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
                    let previous = self.inner.transforms.get(&uid).map(|transform| PendingTransformLerp {
                        uid,
                        source: transform.local_position,
                        destination: state.local_position,
                        source_angle: transform.local_rotation,
                        destination_angle: state.rotation,
                        parent: state.parent_id,
                        source_anchored: transform.anchored,
                        destination_anchored: state.anchored,
                    });
                    self.inner.transforms.entry(uid).or_insert_with(|| {
                        let mut component = sekai::TransformComponent::new();
                        component.base.owner = uid;
                        component
                    });
                    let _ = self.inner.apply_transform_state(uid, state);
                    if let Some(lerp) = previous {
                        self.pending_transform_lerps.push(lerp);
                    }
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
            6 => {
                if let Ok(state) =
                    serializer.deserialize_component_state::<AppearanceComponentState>(component_state)
                {
                    let appearance = self.inner.appearances.entry(uid).or_insert_with(|| {
                        let mut component = sekai::AppearanceComponent::new();
                        component.base.owner = uid;
                        component
                    });
                    appearance.handle_component_state(state);
                }
            }
            7 => {
                if let Ok(state) =
                    serializer.deserialize_component_state::<FixturesComponentState>(component_state)
                {
                    let fixtures = self.inner.fixtures.entry(uid).or_insert_with(|| {
                        let mut component = sekai::FixturesComponent::new();
                        component.base.owner = uid;
                        component
                    });
                    fixtures.handle_component_state(state);
                }
            }
            8 => {
                if let Ok(state) =
                    serializer.deserialize_component_state::<JointComponentState>(component_state)
                {
                    let joints = self.inner.joint_components.entry(uid).or_insert_with(|| {
                        let mut component = sekai::JointComponent::new();
                        component.base.owner = uid;
                        component
                    });
                    joints.handle_component_state(state);
                }
            }
            9 => {
                if let Ok(state) =
                    serializer.deserialize_component_state::<EntityLookupComponentState>(component_state)
                {
                    let lookup = self.inner.entity_lookups.entry(uid).or_insert_with(|| {
                        let mut component = sekai::EntityLookupComponent::new();
                        component.base.owner = uid;
                        component
                    });
                    lookup.handle_component_state(state);
                }
            }
            10 => {
                if let Ok(state) =
                    serializer.deserialize_component_state::<BroadphaseComponentState>(component_state)
                {
                    let broadphase = self.inner.broadphases.entry(uid).or_insert_with(|| {
                        let mut component = sekai::BroadphaseComponent::new();
                        component.base.owner = uid;
                        component
                    });
                    broadphase.handle_component_state(state);
                }
            }
            11 => {
                if let Ok(state) =
                    serializer.deserialize_component_state::<SharedPhysicsMapComponentState>(component_state)
                {
                    let physics_map = self.inner.physics_maps.entry(uid).or_insert_with(|| {
                        let mut component = sekai::SharedPhysicsMapComponent::new();
                        component.base.owner = uid;
                        component
                    });
                    physics_map.handle_component_state(state);
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
    use super::{ClientEntityManager, PendingTransformLerp};
    use butsuri::{AabbShape, Fixture, Joint, JointType, PhysShape};
    use daikoku::{EntityMessageType, MsgEntity};
    use std::collections::HashMap;
    use sekai::{
        ChunkDatum, EntityUid, GameStateMapData, GridDatum, GridId, MapCoordinates,
        MapId, MetaDataComponentState, RobustSerializer, SerializedComponentChange,
        SerializedEntityState, Tile, TileRenderFlag, AppearanceComponentState, AppearanceValue,
        FixturesComponentState, JointComponentState, EntityLookupComponentState,
        EntityLookupEntry, BroadphaseComponentState, SharedPhysicsMapComponentState,
        TransformComponentState,
    };
    use keisan::{Angle, Box2, Vector2, Vector2i};

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
            deleted_grids: Vec::new(),
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

    #[test]
    fn client_entity_manager_applies_serialized_appearance_state() {
        let mut entities = ClientEntityManager::new();
        let mut serializer = RobustSerializer::new();
        let state = SerializedEntityState {
            uid: EntityUid::new(11),
            component_changes: vec![SerializedComponentChange::new(
                6,
                true,
                false,
                Some(
                    serializer
                        .serialize_component_state(&AppearanceComponentState {
                            data: HashMap::from([
                                (String::from("mode"), AppearanceValue::UInt(3)),
                                (String::from("name"), AppearanceValue::Text(String::from("shimmer"))),
                            ]),
                        })
                        .unwrap(),
                ),
            )],
        };
        entities.apply_serialized_entity_state(&mut serializer, &state);
        let appearance = entities.inner.appearances.get(&EntityUid::new(11)).unwrap();
        assert_eq!(appearance.get_data::<u32>("mode"), Some(3));
        assert_eq!(appearance.get_data::<String>("name").as_deref(), Some("shimmer"));
    }

    #[test]
    fn client_entity_manager_applies_deleted_component_changes() {
        let mut entities = ClientEntityManager::new();
        let uid = entities.create_entity(None, EntityUid::new(18));
        entities.initialize_entity(uid);
        entities.inner.ensure_appearance(uid).set_data("mode", 3u32);
        let state = SerializedEntityState {
            uid,
            component_changes: vec![SerializedComponentChange::new(6, false, true, None)],
        };
        let mut serializer = RobustSerializer::new();
        entities.apply_serialized_entity_state(&mut serializer, &state);
        assert!(!entities.inner.appearances.contains_key(&uid));
    }

    #[test]
    fn client_entity_manager_applies_deleted_spatial_components_with_runtime_cleanup() {
        let mut entities = ClientEntityManager::new();
        let map_uid = entities.ensure_map_entity(MapId::new(1));
        let grid_uid = entities.ensure_grid_entity(GridId::new(4));
        entities.inner.map_grid_components.get_mut(&grid_uid).unwrap().chunk_size = 8;
        entities
            .inner
            .map_grids
            .insert(grid_uid, sekai::MapGrid::new(MapId::new(1), grid_uid, GridId::new(4), 8));
        let _ = entities.inner.set_parent(grid_uid, map_uid);
        entities
            .inner
            .map_grids
            .get_mut(&grid_uid)
            .unwrap()
            .set_tile(Vector2i::new(0, 0), Tile::new(1, TileRenderFlag(0), 0));

        let child = entities.create_entity(None, EntityUid::new(181));
        entities.initialize_entity(child);
        let _ = entities.inner.apply_transform_state(
            child,
            TransformComponentState {
                local_position: Vector2::new(0.5, 0.5),
                rotation: Angle::ZERO,
                parent_id: grid_uid,
                map_id: MapId::new(1),
                grid_id: GridId::new(4),
                no_local_rotation: false,
                anchored: false,
            },
        );
        entities.rebuild_runtime_state();

        let mut serializer = RobustSerializer::new();
        let deleted_grid = SerializedEntityState {
            uid: grid_uid,
            component_changes: vec![SerializedComponentChange::new(4, false, true, None)],
        };
        entities.apply_serialized_entity_state(&mut serializer, &deleted_grid);
        assert!(!entities.inner.map_grid_components.contains_key(&grid_uid));
        assert!(!entities.inner.map_grids.contains_key(&grid_uid));

        let deleted_transform = SerializedEntityState {
            uid: child,
            component_changes: vec![SerializedComponentChange::new(2, false, true, None)],
        };
        entities.apply_serialized_entity_state(&mut serializer, &deleted_transform);
        assert!(!entities.inner.transforms.contains_key(&child));
    }

    #[test]
    fn client_entity_manager_applies_deleted_map_component_with_runtime_cleanup() {
        let mut entities = ClientEntityManager::new();
        let map_uid = entities.ensure_map_entity(MapId::new(1));
        entities.inner.ensure_broadphase(map_uid);
        entities.inner.ensure_physics_map(map_uid);

        let grid_uid = entities.ensure_grid_entity(GridId::new(12));
        entities.inner.map_grid_components.get_mut(&grid_uid).unwrap().chunk_size = 8;
        entities
            .inner
            .map_grids
            .insert(grid_uid, sekai::MapGrid::new(MapId::new(1), grid_uid, GridId::new(12), 8));
        let _ = entities.inner.set_parent(grid_uid, map_uid);

        let deleted_map = SerializedEntityState {
            uid: map_uid,
            component_changes: vec![SerializedComponentChange::new(3, false, true, None)],
        };
        let mut serializer = RobustSerializer::new();
        entities.apply_serialized_entity_state(&mut serializer, &deleted_map);

        assert!(!entities.inner.map_components.contains_key(&map_uid));
        assert!(!entities.inner.broadphases.contains_key(&map_uid));
        assert!(!entities.inner.physics_maps.contains_key(&map_uid));
        let grid_transform = entities.inner.transforms.get(&grid_uid).unwrap();
        assert_eq!(grid_transform.parent, EntityUid::INVALID);
        assert_eq!(grid_transform.map_id, MapId::NULLSPACE);
        assert_eq!(entities.inner.map_grids.get(&grid_uid).unwrap().parent_map_id, MapId::NULLSPACE);
    }

    #[test]
    fn client_entity_manager_applies_deleted_grids_from_map_data() {
        let mut entities = ClientEntityManager::new();
        entities.apply_map_data(&GameStateMapData {
            grid_data: std::iter::once((
                GridId::new(4),
                GridDatum {
                    coordinates: MapCoordinates::new(Vector2::ZERO, MapId::new(1)),
                    angle: Angle::ZERO,
                    chunk_data: Vec::new(),
                },
            ))
            .collect(),
            deleted_grids: Vec::new(),
        });

        assert!(entities
            .inner
            .map_grid_components
            .values()
            .any(|component| component.grid_index == GridId::new(4)));

        entities.apply_map_data(&GameStateMapData {
            grid_data: HashMap::new(),
            deleted_grids: vec![GridId::new(4)],
        });

        assert!(!entities
            .inner
            .map_grid_components
            .values()
            .any(|component| component.grid_index == GridId::new(4)));
    }

    #[test]
    fn client_entity_manager_applies_incremental_chunk_updates_and_deletions() {
        let mut entities = ClientEntityManager::new();
        entities.apply_map_data(&GameStateMapData {
            grid_data: std::iter::once((
                GridId::new(4),
                GridDatum {
                    coordinates: MapCoordinates::new(Vector2::ZERO, MapId::new(1)),
                    angle: Angle::ZERO,
                    chunk_data: vec![
                        ChunkDatum::create_modified(
                            Vector2i::new(0, 0),
                            vec![Tile::new(1, TileRenderFlag(0), 0)],
                        ),
                        ChunkDatum::create_modified(
                            Vector2i::new(1, 0),
                            vec![Tile::new(2, TileRenderFlag(0), 0)],
                        ),
                    ],
                },
            ))
            .collect(),
            deleted_grids: Vec::new(),
        });

        entities.apply_map_data(&GameStateMapData {
            grid_data: std::iter::once((
                GridId::new(4),
                GridDatum {
                    coordinates: MapCoordinates::new(Vector2::ZERO, MapId::new(1)),
                    angle: Angle::ZERO,
                    chunk_data: vec![
                        ChunkDatum::create_modified(
                            Vector2i::new(1, 0),
                            vec![Tile::new(9, TileRenderFlag(0), 0)],
                        ),
                        ChunkDatum::create_deleted(Vector2i::new(0, 0)),
                    ],
                },
            ))
            .collect(),
            deleted_grids: Vec::new(),
        });

        let uid = entities
            .inner
            .map_grid_components
            .iter()
            .find(|(_, component)| component.grid_index == GridId::new(4))
            .map(|(uid, _)| *uid)
            .unwrap();
        let grid = entities.inner.map_grids.get(&uid).unwrap();
        assert_eq!(grid.get_tile_ref(Vector2i::new(16, 0)).tile.type_id, 9);
        assert!(grid.get_tile_ref(Vector2i::new(0, 0)).tile.is_empty());
        assert!(grid.try_get_chunk(Vector2i::new(0, 0)).is_none());
        assert!(grid.try_get_chunk(Vector2i::new(1, 0)).is_some());
    }

    #[test]
    fn client_entity_manager_can_find_map_entity_by_map_id() {
        let mut entities = ClientEntityManager::new();
        let uid = entities.create_entity(None, EntityUid::new(90));
        entities.initialize_entity(uid);
        entities.inner.map_components.entry(uid).or_insert_with(|| {
            let mut component = sekai::MapComponent::new();
            component.base.owner = uid;
            component.world_map = MapId::new(7);
            component
        });
        assert_eq!(entities.map_entity_for(MapId::new(7)), Some(uid));
    }

    #[test]
    fn client_entity_manager_queues_pending_transform_lerps_from_snapshots() {
        let mut entities = ClientEntityManager::new();
        let uid = entities.create_entity(None, EntityUid::new(15));
        entities.initialize_entity(uid);
        entities.inner.transforms.get_mut(&uid).unwrap().local_position = Vector2::new(1.0, 0.0);
        let mut serializer = RobustSerializer::new();
        let state = SerializedEntityState {
            uid,
            component_changes: vec![SerializedComponentChange::new(
                2,
                false,
                false,
                Some(
                    serializer
                        .serialize_component_state(&TransformComponentState {
                            local_position: Vector2::new(3.0, 0.0),
                            rotation: Angle::from_degrees(30.0),
                            parent_id: EntityUid::INVALID,
                            map_id: MapId::NULLSPACE,
                            grid_id: GridId::INVALID,
                            no_local_rotation: false,
                            anchored: false,
                        })
                        .unwrap(),
                ),
            )],
        };
        entities.apply_serialized_entity_state(&mut serializer, &state);
        assert_eq!(
            entities.take_pending_transform_lerps(),
            vec![PendingTransformLerp {
                uid,
                source: Vector2::new(1.0, 0.0),
                destination: Vector2::new(3.0, 0.0),
                source_angle: Angle::ZERO,
                destination_angle: Angle::from_degrees(30.0),
                parent: EntityUid::INVALID,
                source_anchored: false,
                destination_anchored: false,
            }]
        );
    }

    #[test]
    fn client_entity_manager_applies_serialized_fixtures_and_joints() {
        let mut entities = ClientEntityManager::new();
        let mut serializer = RobustSerializer::new();
        let mut joint = Joint::new(12, 99, JointType::Distance);
        joint.id = String::from("rope");
        let state = SerializedEntityState {
            uid: EntityUid::new(12),
            component_changes: vec![
                SerializedComponentChange::new(
                    7,
                    true,
                    false,
                    Some(
                        serializer
                            .serialize_component_state(&FixturesComponentState {
                                fixtures: vec![Fixture::new(
                                    "main",
                                    PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
                                )],
                            })
                            .unwrap(),
                    ),
                ),
                SerializedComponentChange::new(
                    8,
                    true,
                    false,
                    Some(
                        serializer
                            .serialize_component_state(&JointComponentState { joints: vec![joint] })
                            .unwrap(),
                    ),
                ),
            ],
        };
        entities.apply_serialized_entity_state(&mut serializer, &state);
        assert_eq!(entities.inner.fixtures.get(&EntityUid::new(12)).unwrap().fixture_count(), 1);
        assert_eq!(entities.inner.joint_components.get(&EntityUid::new(12)).unwrap().joint_count(), 1);
    }

    #[test]
    fn client_entity_manager_applies_serialized_spatial_runtime_components() {
        let mut entities = ClientEntityManager::new();
        let mut serializer = RobustSerializer::new();
        let state = SerializedEntityState {
            uid: EntityUid::new(13),
            component_changes: vec![
                SerializedComponentChange::new(
                    9,
                    true,
                    false,
                    Some(
                        serializer
                            .serialize_component_state(&EntityLookupComponentState {
                                entries: vec![EntityLookupEntry {
                                    entity: EntityUid::new(77),
                                    bounds: Box2::new(-1.0, -1.0, 1.0, 1.0),
                                }],
                            })
                            .unwrap(),
                    ),
                ),
                SerializedComponentChange::new(
                    10,
                    true,
                    false,
                    Some(
                        serializer
                            .serialize_component_state(&BroadphaseComponentState {
                                entries: vec![butsuri::BroadphaseEntry {
                                    owner_id: 13,
                                    fixture: Fixture::new(
                                        "main",
                                        PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
                                    ),
                                    transform: butsuri::Transform::new(Vector2::new(3.0, 0.0), 0.0),
                                }],
                            })
                            .unwrap(),
                    ),
                ),
                SerializedComponentChange::new(
                    11,
                    true,
                    false,
                    Some(
                        serializer
                            .serialize_component_state(&SharedPhysicsMapComponentState {
                                auto_clear_forces: true,
                                gravity: Vector2::new(0.0, -9.8),
                                bodies: vec![EntityUid::new(13)],
                                awake_bodies: vec![EntityUid::new(13)],
                            })
                            .unwrap(),
                    ),
                ),
            ],
        };
        entities.apply_serialized_entity_state(&mut serializer, &state);
        assert_eq!(entities.inner.entity_lookups.get(&EntityUid::new(13)).unwrap().entities.len(), 1);
        assert_eq!(
            entities
                .inner
                .broadphases
                .get(&EntityUid::new(13))
                .unwrap()
                .tree
                .query_aabb(Box2::new(1.0, -2.0, 5.0, 2.0))
                .len(),
            1
        );
        let physics_map = entities.inner.physics_maps.get(&EntityUid::new(13)).unwrap();
        assert!(physics_map.auto_clear_forces);
        assert!(physics_map.bodies.contains(&EntityUid::new(13)));
        assert!(physics_map.awake_bodies.contains(&EntityUid::new(13)));
    }

    #[test]
    fn client_entity_manager_creates_map_owner_and_parents_grids_from_map_data() {
        let mut entities = ClientEntityManager::new();
        entities.apply_map_data(&GameStateMapData {
            grid_data: std::iter::once((
                GridId::new(11),
                GridDatum {
                    coordinates: MapCoordinates::new(Vector2::new(3.0, 4.0), MapId::new(7)),
                    angle: Angle::from_degrees(15.0),
                    chunk_data: Vec::new(),
                },
            ))
            .collect(),
            deleted_grids: Vec::new(),
        });

        let map_uid = entities.map_entity_for(MapId::new(7)).unwrap();
        let grid_uid = entities
            .inner
            .map_grid_components
            .iter()
            .find(|(_, component)| component.grid_index == GridId::new(11))
            .map(|(uid, _)| *uid)
            .unwrap();
        let transform = entities.inner.transforms.get(&grid_uid).unwrap();
        assert_eq!(transform.parent, map_uid);
        assert_eq!(transform.grid_id, GridId::new(11));
        assert_eq!(transform.map_id, MapId::new(7));
    }

    #[test]
    fn client_entity_manager_rebuilds_lookup_and_physics_runtime_after_grid_motion() {
        let mut entities = ClientEntityManager::new();
        entities.apply_map_data(&GameStateMapData {
            grid_data: std::iter::once((
                GridId::new(11),
                GridDatum {
                    coordinates: MapCoordinates::new(Vector2::ZERO, MapId::new(7)),
                    angle: Angle::ZERO,
                    chunk_data: vec![ChunkDatum::create_modified(
                        Vector2i::new(0, 0),
                        vec![Tile::new(1, TileRenderFlag(0), 0)],
                    )],
                },
            ))
            .collect(),
            deleted_grids: Vec::new(),
        });

        let map_uid = entities.map_entity_for(MapId::new(7)).unwrap();
        let grid_uid = entities
            .inner
            .map_grid_components
            .iter()
            .find(|(_, component)| component.grid_index == GridId::new(11))
            .map(|(uid, _)| *uid)
            .unwrap();

        let uid = entities.create_entity(None, EntityUid::new(250));
        entities.initialize_entity(uid);
        let _ = entities.inner.apply_transform_state(
            uid,
            TransformComponentState {
                local_position: Vector2::new(0.5, 0.5),
                rotation: Angle::ZERO,
                parent_id: grid_uid,
                map_id: MapId::new(7),
                grid_id: GridId::new(11),
                no_local_rotation: false,
                anchored: false,
            },
        );
        entities.inner.transforms.get_mut(&uid).unwrap().map_id = MapId::new(7);
        let body = entities.inner.ensure_physics(uid);
        body.set_body_type(sekai::BodyType::Dynamic);
        body.can_collide = true;
        body.awake = true;
        entities
            .inner
            .ensure_fixtures(uid)
            .insert_fixture(Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ));

        entities.rebuild_runtime_state();
        assert_eq!(
            sekai::EntityLookupSystem.get_entities_intersecting_world_aabb(
                &entities.inner,
                grid_uid,
                Box2::new(-1.0, -1.0, 1.0, 1.0),
                false,
            ),
            vec![uid]
        );

        entities.apply_map_data(&GameStateMapData {
            grid_data: std::iter::once((
                GridId::new(11),
                GridDatum {
                    coordinates: MapCoordinates::new(Vector2::new(10.0, 0.0), MapId::new(7)),
                    angle: Angle::ZERO,
                    chunk_data: Vec::new(),
                },
            ))
            .collect(),
            deleted_grids: Vec::new(),
        });
        entities.rebuild_runtime_state();

        assert_eq!(
            sekai::EntityLookupSystem.get_entities_intersecting_world_aabb(
                &entities.inner,
                grid_uid,
                Box2::new(9.0, -1.0, 11.0, 1.0),
                false,
            ),
            vec![uid]
        );
        assert_eq!(
            sekai::SharedPhysicsSystem.query_aabb_entities(
                &entities.inner,
                map_uid,
                Box2::new(9.0, -1.0, 11.0, 1.0),
            ),
            vec![uid]
        );
    }

    #[test]
    fn client_entity_manager_rebuild_runtime_infers_map_membership_from_parents() {
        let mut entities = ClientEntityManager::new();
        let map_uid = entities.ensure_map_entity(MapId::new(3));
        let grid_uid = entities.ensure_grid_entity(GridId::new(21));
        entities.inner.map_grid_components.get_mut(&grid_uid).unwrap().chunk_size = 8;
        entities.inner.map_grids.insert(
            grid_uid,
            sekai::MapGrid::new(MapId::new(3), grid_uid, GridId::new(21), 8),
        );
        let _ = entities.inner.set_parent(grid_uid, map_uid);

        let uid = entities.create_entity(None, EntityUid::new(301));
        entities.initialize_entity(uid);
        let _ = entities.inner.apply_transform_state(
            uid,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: grid_uid,
                map_id: MapId::new(3),
                grid_id: GridId::new(21),
                no_local_rotation: false,
                anchored: false,
            },
        );
        entities.inner.transforms.get_mut(&uid).unwrap().map_id = MapId::NULLSPACE;

        entities.rebuild_runtime_state();
        let transform = entities.inner.transforms.get(&uid).unwrap();
        assert_eq!(transform.map_id, MapId::new(3));
        assert_eq!(transform.grid_id, GridId::new(21));
    }
}
