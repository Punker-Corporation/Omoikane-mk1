use daikoku::{EntityMessageType, MsgEntity};
use keisan::{Angle, Vector2};
use sekai::{
    EntityManager, EntityUid, NetworkComponentMessage, RobustSerializer, TransformComponentState,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PendingTransformLerp {
    pub(crate) uid: EntityUid,
    pub(crate) source: Vector2,
    pub(crate) destination: Vector2,
    pub(crate) source_angle: Angle,
    pub(crate) destination_angle: Angle,
    pub(crate) parent: EntityUid,
    pub(crate) source_anchored: bool,
    pub(crate) destination_anchored: bool,
}

pub(crate) struct ClientEntityManager {
    pub(crate) inner: EntityManager,
    queued_messages: Vec<(u32, MsgEntity)>,
    incoming_sequence: u32,
    pending_transform_lerps: Vec<PendingTransformLerp>,
    pub(crate) received_component_messages: Vec<NetworkComponentMessage<(), (), String>>,
    pub(crate) received_system_messages: Vec<String>,
}

impl ClientEntityManager {
    pub(crate) fn new() -> Self {
        Self {
            inner: EntityManager::new(),
            queued_messages: Vec::new(),
            incoming_sequence: 0,
            pending_transform_lerps: Vec::new(),
            received_component_messages: Vec::new(),
            received_system_messages: Vec::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn create_entity(&mut self, prototype: Option<&str>, uid: EntityUid) -> EntityUid {
        self.inner.alloc_entity_external(uid, prototype);
        uid
    }

    #[cfg(test)]
    pub(crate) fn ensure_map_entity(&mut self, map_id: sekai::MapId) -> EntityUid {
        if let Some(uid) = self.inner.map_entity_for(map_id) {
            return uid;
        }

        self.inner.create_entity_uninitialized_as_map(None, map_id)
    }

    pub(crate) fn handle_entity_network_message(
        &mut self,
        cur_server_tick: jikan::GameTick,
        message: MsgEntity,
    ) {
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

    pub(crate) fn tick_update(&mut self, cur_server_tick: jikan::GameTick) {
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

    pub(crate) fn take_pending_transform_lerps(&mut self) -> Vec<PendingTransformLerp> {
        std::mem::take(&mut self.pending_transform_lerps)
    }

    #[cfg(test)]
    pub(crate) fn apply_serialized_entity_state(
        &mut self,
        serializer: &mut RobustSerializer,
        state: &sekai::SerializedEntityState,
    ) -> Option<EntityUid> {
        let pending_transform_lerps = &mut self.pending_transform_lerps;
        self.inner.apply_serialized_entity_state_with(
            serializer,
            state,
            |manager, uid, transform_state| {
                Self::apply_transform_component_state_with_pending_lerp(
                    pending_transform_lerps,
                    manager,
                    uid,
                    transform_state,
                );
            },
        )
    }

    pub(crate) fn apply_game_state(
        &mut self,
        serializer: &mut RobustSerializer,
        state: &sekai::GameState,
    ) -> Vec<EntityUid> {
        let pending_transform_lerps = &mut self.pending_transform_lerps;
        self.inner.apply_game_state_delta_with(
            serializer,
            state,
            |manager, uid, transform_state| {
                Self::apply_transform_component_state_with_pending_lerp(
                    pending_transform_lerps,
                    manager,
                    uid,
                    transform_state,
                );
            },
        )
    }

    #[cfg(test)]
    fn ensure_grid_entity(&mut self, grid_id: sekai::GridId) -> EntityUid {
        self.inner.ensure_grid_entity_shell(grid_id)
    }

    fn pending_transform_lerp_for(
        manager: &EntityManager,
        uid: EntityUid,
        transform_state: &TransformComponentState,
    ) -> Option<PendingTransformLerp> {
        manager
            .transforms
            .get(&uid)
            .map(|transform| PendingTransformLerp {
                uid,
                source: transform.local_position,
                destination: transform_state.local_position,
                source_angle: transform.local_rotation,
                destination_angle: transform_state.rotation,
                parent: transform_state.parent_id,
                source_anchored: transform.anchored,
                destination_anchored: transform_state.anchored,
            })
    }

    fn apply_transform_component_state_with_pending_lerp(
        pending_transform_lerps: &mut Vec<PendingTransformLerp>,
        manager: &mut EntityManager,
        uid: EntityUid,
        transform_state: TransformComponentState,
    ) {
        let previous = Self::pending_transform_lerp_for(manager, uid, &transform_state);
        let _ = manager.apply_transform_component_state(uid, transform_state);
        if let Some(lerp) = previous {
            pending_transform_lerps.push(lerp);
        }
    }

    #[cfg(test)]
    fn apply_transform_component_state(
        &mut self,
        uid: EntityUid,
        transform_state: TransformComponentState,
    ) {
        Self::apply_transform_component_state_with_pending_lerp(
            &mut self.pending_transform_lerps,
            &mut self.inner,
            uid,
            transform_state,
        );
    }

    #[cfg(test)]
    fn apply_component_state(
        &mut self,
        serializer: &mut RobustSerializer,
        uid: EntityUid,
        net_id: u16,
        component_state: &sekai::SerializableComponentState,
    ) {
        match net_id {
            EntityManager::TRANSFORM_NET_ID => {
                if let Ok(state) = serializer
                    .deserialize_component_state::<TransformComponentState>(component_state)
                {
                    self.apply_transform_component_state(uid, state);
                }
            }
            _ => {
                let _ = self.inner.apply_serialized_component_by_net_id(
                    serializer,
                    uid,
                    net_id,
                    component_state,
                );
            }
        }
    }

    fn dispatch_msg_entity(&mut self, message: MsgEntity) {
        match message.message_type {
            EntityMessageType::ComponentMessage => {
                self.received_component_messages
                    .push(NetworkComponentMessage::new(
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
    use keisan::{Angle, Box2, Vector2, Vector2i};
    use sekai::{
        AppearanceComponentState, AppearanceValue, BroadphaseComponentState, ChunkDatum,
        CollideOnAnchorComponentState, CollisionWakeComponentState, EntityLookupComponentState,
        EntityLookupEntry, EntityUid, FixturesComponentState, GameStateMapData, GridDatum, GridId,
        JointComponentState, MapComponentState, MapCoordinates, MapGridComponentState, MapId,
        MetaDataComponentState, PhysicsComponentState, RobustSerializer, SerializedComponentChange,
        SerializedEntityState, SharedPhysicsMapComponentState, Tile, TileRenderFlag,
        TransformComponentState,
    };
    use std::collections::HashMap;

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
        assert!(entities.inner.entity_exists(EntityUid::new(5)));
        assert_eq!(
            entities
                .inner
                .metadata
                .get(&EntityUid::new(5))
                .unwrap()
                .prototype_id
                .as_deref(),
            Some("mob")
        );
    }

    #[test]
    fn client_entity_manager_applies_map_data_to_grids() {
        let mut entities = ClientEntityManager::new();
        entities.inner.apply_game_state_map_data(&GameStateMapData {
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

        let uid = entities.inner.grid_entity_for(GridId::new(3)).unwrap();
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
                                (
                                    String::from("name"),
                                    AppearanceValue::Text(String::from("shimmer")),
                                ),
                            ]),
                        })
                        .unwrap(),
                ),
            )],
        };
        entities.apply_serialized_entity_state(&mut serializer, &state);
        let appearance = entities.inner.appearances.get(&EntityUid::new(11)).unwrap();
        assert_eq!(appearance.get_data::<u32>("mode"), Some(3));
        assert_eq!(
            appearance.get_data::<String>("name").as_deref(),
            Some("shimmer")
        );
    }

    #[test]
    fn client_entity_manager_applies_deleted_component_changes() {
        let mut entities = ClientEntityManager::new();
        let uid = entities.create_entity(None, EntityUid::new(18));
        entities.inner.initialize_entity(uid);
        assert!(entities.inner.set_appearance_data(uid, "mode", 3u32));
        let state = SerializedEntityState {
            uid,
            component_changes: vec![SerializedComponentChange::new(6, false, true, None)],
        };
        let mut serializer = RobustSerializer::new();
        entities.apply_serialized_entity_state(&mut serializer, &state);
        assert!(!entities.inner.appearances.contains_key(&uid));
    }

    #[test]
    fn client_entity_manager_apply_map_data_parents_grid_to_map_entity_atomically() {
        let mut entities = ClientEntityManager::new();
        let map_uid = entities.ensure_map_entity(MapId::new(7));
        let _ = entities.inner.apply_transform_state(
            map_uid,
            TransformComponentState {
                local_position: Vector2::new(8.0, -3.0),
                rotation: Angle::from_degrees(20.0),
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(7),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );

        entities.inner.apply_game_state_map_data(&GameStateMapData {
            grid_data: std::iter::once((
                GridId::new(44),
                GridDatum {
                    coordinates: MapCoordinates::new(Vector2::new(1.5, 2.5), MapId::new(7)),
                    angle: Angle::from_degrees(15.0),
                    chunk_data: Vec::new(),
                },
            ))
            .collect(),
            deleted_grids: Vec::new(),
        });

        let grid_uid = entities.inner.grid_entity_for(GridId::new(44)).unwrap();
        let transform = entities.inner.transforms.get(&grid_uid).unwrap();
        assert_eq!(transform.parent, map_uid);
        assert_eq!(transform.map_id, MapId::new(7));
        assert_eq!(transform.grid_id, GridId::new(44));
        assert_eq!(transform.local_position, Vector2::new(1.5, 2.5));
        assert_eq!(transform.local_rotation, Angle::from_degrees(15.0));
    }

    #[test]
    fn client_entity_manager_ensure_grid_entity_materializes_runtime_shell() {
        let mut entities = ClientEntityManager::new();
        let uid = entities.ensure_grid_entity(GridId::new(55));

        let transform = entities.inner.transforms.get(&uid).unwrap();
        assert_eq!(transform.map_id, MapId::NULLSPACE);
        assert_eq!(transform.grid_id, GridId::new(55));
        assert_eq!(transform.parent, EntityUid::INVALID);

        let grid_component = entities.inner.map_grid_components.get(&uid).unwrap();
        assert_eq!(grid_component.grid_index, GridId::new(55));
        assert_eq!(grid_component.chunk_size, 16);

        let runtime = entities.inner.map_grids.get(&uid).unwrap();
        assert_eq!(runtime.index, GridId::new(55));
        assert_eq!(runtime.parent_map_id, MapId::NULLSPACE);
    }

    #[test]
    fn client_entity_manager_materializes_grid_runtime_from_snapshot_component_states() {
        let mut entities = ClientEntityManager::new();
        let mut serializer = RobustSerializer::new();
        let uid = EntityUid::new(301);

        entities.create_entity(None, uid);
        let grid_state = serializer
            .serialize_component_state(&MapGridComponentState {
                grid_index: GridId::new(61),
                chunk_size: 8,
            })
            .unwrap();
        entities.apply_component_state(&mut serializer, uid, 4, &grid_state);
        let transform_state = serializer
            .serialize_component_state(&TransformComponentState {
                local_position: Vector2::new(6.0, -2.0),
                rotation: Angle::from_degrees(12.0),
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(11),
                grid_id: GridId::new(61),
                no_local_rotation: false,
                anchored: false,
            })
            .unwrap();
        entities.apply_component_state(&mut serializer, uid, 2, &transform_state);

        let runtime = entities.inner.map_grids.get(&uid).unwrap();
        assert_eq!(runtime.parent_map_id, MapId::new(11));
        assert_eq!(runtime.index, GridId::new(61));
        assert_eq!(runtime.world_position, Vector2::new(6.0, -2.0));
        assert_eq!(runtime.world_rotation, Angle::from_degrees(12.0));
    }

    #[test]
    fn client_entity_manager_handles_map_and_grid_component_snapshot_before_transform() {
        let mut entities = ClientEntityManager::new();
        let mut serializer = RobustSerializer::new();
        let map_uid = EntityUid::new(401);
        let grid_uid = EntityUid::new(402);

        entities.create_entity(None, map_uid);
        let map_state = serializer
            .serialize_component_state(&MapComponentState {
                map_id: MapId::new(17),
                lighting_enabled: true,
                map_paused: true,
            })
            .unwrap();
        entities.apply_component_state(&mut serializer, map_uid, 3, &map_state);
        assert!(entities.inner.transforms.contains_key(&map_uid));
        assert!(
            entities
                .inner
                .map_components
                .get(&map_uid)
                .unwrap()
                .map_paused
        );

        entities.create_entity(None, grid_uid);
        let grid_state = serializer
            .serialize_component_state(&MapGridComponentState {
                grid_index: GridId::new(62),
                chunk_size: 8,
            })
            .unwrap();
        entities.apply_component_state(&mut serializer, grid_uid, 4, &grid_state);
        assert!(entities.inner.transforms.contains_key(&grid_uid));
        assert!(entities.inner.map_grids.contains_key(&grid_uid));

        let transform_state = serializer
            .serialize_component_state(&TransformComponentState {
                local_position: Vector2::new(1.0, 2.0),
                rotation: Angle::from_degrees(10.0),
                parent_id: map_uid,
                map_id: MapId::new(17),
                grid_id: GridId::new(62),
                no_local_rotation: false,
                anchored: false,
            })
            .unwrap();
        let map_transform_state = serializer
            .serialize_component_state(&TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(17),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            })
            .unwrap();
        entities.apply_component_state(&mut serializer, map_uid, 2, &map_transform_state);
        entities.apply_component_state(&mut serializer, grid_uid, 2, &transform_state);
        entities.inner.rebuild_runtime_state();

        let grid_transform = entities.inner.transforms.get(&grid_uid).unwrap();
        assert_eq!(grid_transform.parent, map_uid);
        assert_eq!(grid_transform.map_id, MapId::new(17));
        assert_eq!(grid_transform.grid_id, GridId::new(62));
        let runtime = entities.inner.map_grids.get(&grid_uid).unwrap();
        assert_eq!(runtime.parent_map_id, MapId::new(17));
        assert_eq!(runtime.index, GridId::new(62));
    }

    #[test]
    fn client_entity_manager_applies_deleted_spatial_components_with_runtime_cleanup() {
        let mut entities = ClientEntityManager::new();
        let map_uid = entities.ensure_map_entity(MapId::new(1));
        let grid_uid = entities.ensure_grid_entity(GridId::new(4));
        entities
            .inner
            .map_grid_components
            .get_mut(&grid_uid)
            .unwrap()
            .chunk_size = 8;
        entities.inner.map_grids.insert(
            grid_uid,
            sekai::MapGrid::new(MapId::new(1), grid_uid, GridId::new(4), 8),
        );
        let _ = entities.inner.set_parent(grid_uid, map_uid);
        entities
            .inner
            .map_grids
            .get_mut(&grid_uid)
            .unwrap()
            .set_tile(Vector2i::new(0, 0), Tile::new(1, TileRenderFlag(0), 0));

        let child = entities.create_entity(None, EntityUid::new(181));
        entities.inner.initialize_entity(child);
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
        entities.inner.rebuild_runtime_state();

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
        entities
            .inner
            .map_grid_components
            .get_mut(&grid_uid)
            .unwrap()
            .chunk_size = 8;
        entities.inner.map_grids.insert(
            grid_uid,
            sekai::MapGrid::new(MapId::new(1), grid_uid, GridId::new(12), 8),
        );
        let _ = entities.inner.set_parent(grid_uid, map_uid);

        let deleted_map = SerializedEntityState {
            uid: map_uid,
            component_changes: vec![SerializedComponentChange::new(3, false, true, None)],
        };
        let mut serializer = RobustSerializer::new();
        entities.apply_serialized_entity_state(&mut serializer, &deleted_map);

        assert!(!entities.inner.map_components.contains_key(&map_uid));
        assert!(!entities.inner.has_map_broadphase(MapId::new(1)));
        assert!(!entities.inner.has_map_physics_runtime(MapId::new(1)));
        let grid_transform = entities.inner.transforms.get(&grid_uid).unwrap();
        assert_eq!(grid_transform.parent, EntityUid::INVALID);
        assert_eq!(grid_transform.map_id, MapId::NULLSPACE);
        assert_eq!(
            entities
                .inner
                .map_grids
                .get(&grid_uid)
                .unwrap()
                .parent_map_id,
            MapId::NULLSPACE
        );
    }

    #[test]
    fn client_entity_manager_applies_deleted_physics_and_fixtures_with_runtime_cleanup() {
        let mut entities = ClientEntityManager::new();
        let map_uid = entities.ensure_map_entity(MapId::new(2));
        entities.inner.ensure_broadphase(map_uid);
        entities.inner.ensure_physics_map(map_uid);

        let first = entities.create_entity(None, EntityUid::new(210));
        entities.inner.initialize_entity(first);
        let _ = entities.inner.apply_transform_state(
            first,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(2),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(first);
        body.can_collide = true;
        body.set_body_type(sekai::BodyType::Dynamic);
        body.awake = true;
        let _ = entities.inner.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "first",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let second = entities.create_entity(None, EntityUid::new(211));
        entities.inner.initialize_entity(second);
        let _ = entities.inner.apply_transform_state(
            second,
            TransformComponentState {
                local_position: Vector2::new(0.5, 0.0),
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(2),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(second);
        body.can_collide = true;
        body.set_body_type(sekai::BodyType::Dynamic);
        body.awake = true;
        let _ = entities.inner.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "second",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        entities.inner.rebuild_runtime_state();
        assert_eq!(entities.inner.map_contact_count(MapId::new(2)), 1);

        let mut serializer = RobustSerializer::new();
        entities.apply_serialized_entity_state(
            &mut serializer,
            &SerializedEntityState {
                uid: first,
                component_changes: vec![SerializedComponentChange::new(5, false, true, None)],
            },
        );
        assert_eq!(entities.inner.map_contact_count(MapId::new(2)), 0);
        assert_eq!(
            entities
                .inner
                .query_aabb_entities(map_uid, Box2::new(-1.0, -1.0, 1.0, 1.0)),
            vec![second]
        );

        entities.apply_serialized_entity_state(
            &mut serializer,
            &SerializedEntityState {
                uid: second,
                component_changes: vec![SerializedComponentChange::new(7, false, true, None)],
            },
        );
        assert!(
            entities
                .inner
                .query_aabb_entities(map_uid, Box2::new(-1.0, -1.0, 1.0, 1.0))
                .is_empty()
        );
    }

    #[test]
    fn client_entity_manager_applies_deleted_joint_component_with_runtime_cleanup() {
        let mut entities = ClientEntityManager::new();
        let map_uid = entities.ensure_map_entity(MapId::new(3));
        entities.inner.ensure_broadphase(map_uid);
        entities.inner.ensure_physics_map(map_uid);

        let first = entities.create_entity(None, EntityUid::new(212));
        entities.inner.initialize_entity(first);
        let _ = entities.inner.apply_transform_state(
            first,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(3),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(first);
        body.can_collide = true;
        body.set_body_type(sekai::BodyType::Dynamic);
        body.awake = true;
        let _ = entities.inner.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "first",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let second = entities.create_entity(None, EntityUid::new(213));
        entities.inner.initialize_entity(second);
        let _ = entities.inner.apply_transform_state(
            second,
            TransformComponentState {
                local_position: Vector2::new(0.5, 0.0),
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(3),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(second);
        body.can_collide = true;
        body.set_body_type(sekai::BodyType::Dynamic);
        body.awake = true;
        let _ = entities.inner.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "second",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let mut joint =
            butsuri::Joint::new(first.raw(), second.raw(), butsuri::JointType::Distance);
        joint.id = "rope".to_string();
        joint.collide_connected = false;
        assert!(entities.inner.add_joint_between(joint));
        entities.inner.rebuild_runtime_state();
        assert_eq!(entities.inner.map_contact_count(MapId::new(3)), 0);

        let mut serializer = RobustSerializer::new();
        entities.apply_serialized_entity_state(
            &mut serializer,
            &SerializedEntityState {
                uid: first,
                component_changes: vec![SerializedComponentChange::new(8, false, true, None)],
            },
        );

        assert!(!entities.inner.joint_components.contains_key(&first));
        assert_eq!(
            entities
                .inner
                .joint_components
                .get(&second)
                .unwrap()
                .joint_count(),
            0
        );
        assert_eq!(entities.inner.map_contact_count(MapId::new(3)), 1);
    }

    #[test]
    fn client_entity_manager_applies_deleted_physics_with_implicit_joint_cleanup() {
        let mut entities = ClientEntityManager::new();
        let map_uid = entities.ensure_map_entity(MapId::new(4));
        entities.inner.ensure_broadphase(map_uid);
        entities.inner.ensure_physics_map(map_uid);

        let first = entities.create_entity(None, EntityUid::new(214));
        entities.inner.initialize_entity(first);
        let _ = entities.inner.apply_transform_state(
            first,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(4),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(first);
        body.can_collide = true;
        body.set_body_type(sekai::BodyType::Dynamic);
        body.awake = true;
        let _ = entities.inner.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "first",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let second = entities.create_entity(None, EntityUid::new(215));
        entities.inner.initialize_entity(second);
        let _ = entities.inner.apply_transform_state(
            second,
            TransformComponentState {
                local_position: Vector2::new(0.5, 0.0),
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(4),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(second);
        body.can_collide = true;
        body.set_body_type(sekai::BodyType::Dynamic);
        body.awake = true;
        let _ = entities.inner.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "second",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let mut joint =
            butsuri::Joint::new(first.raw(), second.raw(), butsuri::JointType::Distance);
        joint.id = "rope".to_string();
        joint.collide_connected = false;
        assert!(entities.inner.add_joint_between(joint));
        entities.inner.rebuild_runtime_state();
        assert_eq!(entities.inner.map_contact_count(MapId::new(5)), 0);

        let mut serializer = RobustSerializer::new();
        entities.apply_serialized_entity_state(
            &mut serializer,
            &SerializedEntityState {
                uid: first,
                component_changes: vec![
                    SerializedComponentChange::new(5, false, true, None),
                    SerializedComponentChange::new(8, false, true, None),
                ],
            },
        );

        assert!(!entities.inner.physics.contains_key(&first));
        assert!(!entities.inner.joint_components.contains_key(&first));
        assert_eq!(
            entities
                .inner
                .joint_components
                .get(&second)
                .unwrap()
                .joint_count(),
            0
        );
    }

    #[test]
    fn client_entity_manager_applies_deleted_grids_from_map_data() {
        let mut entities = ClientEntityManager::new();
        entities.inner.apply_game_state_map_data(&GameStateMapData {
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

        assert!(entities.inner.grid_exists(GridId::new(4)));

        entities.inner.apply_game_state_map_data(&GameStateMapData {
            grid_data: HashMap::new(),
            deleted_grids: vec![GridId::new(4)],
        });

        assert!(!entities.inner.grid_exists(GridId::new(4)));
    }

    #[test]
    fn client_entity_manager_grid_lookup_helpers_track_map_data_lifecycle() {
        let mut entities = ClientEntityManager::new();
        entities.inner.apply_game_state_map_data(&GameStateMapData {
            grid_data: std::iter::once((
                GridId::new(13),
                GridDatum {
                    coordinates: MapCoordinates::new(Vector2::new(2.0, 3.0), MapId::new(6)),
                    angle: Angle::ZERO,
                    chunk_data: Vec::new(),
                },
            ))
            .collect(),
            deleted_grids: Vec::new(),
        });

        let map_uid = entities.inner.map_entity_for(MapId::new(6)).unwrap();
        let grid_uid = entities.inner.grid_entity_for(GridId::new(13)).unwrap();
        assert_eq!(
            entities.inner.transforms.get(&grid_uid).unwrap().parent,
            map_uid
        );
        assert!(entities.inner.grid_exists(GridId::new(13)));

        entities.inner.apply_game_state_map_data(&GameStateMapData {
            grid_data: HashMap::new(),
            deleted_grids: vec![GridId::new(13)],
        });

        assert_eq!(entities.inner.grid_entity_for(GridId::new(13)), None);
        assert!(!entities.inner.grid_exists(GridId::new(13)));
    }

    #[test]
    fn client_entity_manager_applies_incremental_chunk_updates_and_deletions() {
        let mut entities = ClientEntityManager::new();
        entities.inner.apply_game_state_map_data(&GameStateMapData {
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

        entities.inner.apply_game_state_map_data(&GameStateMapData {
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

        let uid = entities.inner.grid_entity_for(GridId::new(4)).unwrap();
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
        entities.inner.initialize_entity(uid);
        entities.inner.map_components.entry(uid).or_insert_with(|| {
            let mut component = sekai::MapComponent::new();
            component.base.owner = uid;
            component.world_map = MapId::new(7);
            component
        });
        assert_eq!(entities.inner.map_entity_for(MapId::new(7)), Some(uid));
    }

    #[test]
    fn client_entity_manager_queues_pending_transform_lerps_from_snapshots() {
        let mut entities = ClientEntityManager::new();
        let uid = entities.create_entity(None, EntityUid::new(15));
        entities.inner.initialize_entity(uid);
        entities
            .inner
            .transforms
            .get_mut(&uid)
            .unwrap()
            .local_position = Vector2::new(1.0, 0.0);
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
    fn client_entity_manager_materializes_new_snapshot_entities_with_initial_transform() {
        let mut entities = ClientEntityManager::new();
        let mut serializer = RobustSerializer::new();
        let state = SerializedEntityState {
            uid: EntityUid::new(115),
            component_changes: vec![
                SerializedComponentChange::new(
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
                ),
                SerializedComponentChange::new(
                    2,
                    true,
                    false,
                    Some(
                        serializer
                            .serialize_component_state(&TransformComponentState {
                                local_position: Vector2::new(2.0, 3.0),
                                rotation: Angle::from_degrees(20.0),
                                parent_id: EntityUid::INVALID,
                                map_id: MapId::new(12),
                                grid_id: GridId::INVALID,
                                no_local_rotation: false,
                                anchored: false,
                            })
                            .unwrap(),
                    ),
                ),
            ],
        };

        entities.apply_serialized_entity_state(&mut serializer, &state);

        let transform = entities.inner.transforms.get(&EntityUid::new(115)).unwrap();
        assert_eq!(transform.local_position, Vector2::new(2.0, 3.0));
        assert_eq!(transform.local_rotation, Angle::from_degrees(20.0));
        assert_eq!(transform.map_id, MapId::new(12));
        assert_eq!(
            entities
                .inner
                .metadata
                .get(&EntityUid::new(115))
                .unwrap()
                .prototype_id
                .as_deref(),
            Some("mob")
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
                                    PhysShape::Aabb(AabbShape::new(
                                        Box2::new(-1.0, -1.0, 1.0, 1.0),
                                        0.0,
                                    )),
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
                            .serialize_component_state(&JointComponentState {
                                joints: vec![joint],
                            })
                            .unwrap(),
                    ),
                ),
            ],
        };
        entities.apply_serialized_entity_state(&mut serializer, &state);
        assert_eq!(
            entities
                .inner
                .fixtures
                .get(&EntityUid::new(12))
                .unwrap()
                .fixture_count(),
            1
        );
        assert_eq!(
            entities
                .inner
                .joint_components
                .get(&EntityUid::new(12))
                .unwrap()
                .joint_count(),
            1
        );
    }

    #[test]
    fn client_entity_manager_applies_serialized_awake_state_in_physics_component() {
        let mut entities = ClientEntityManager::new();
        let mut serializer = RobustSerializer::new();
        let state = SerializedEntityState {
            uid: EntityUid::new(120),
            component_changes: vec![SerializedComponentChange::new(
                5,
                true,
                false,
                Some(
                    serializer
                        .serialize_component_state(&PhysicsComponentState {
                            can_collide: true,
                            awake: false,
                            sleeping_allowed: true,
                            fixed_rotation: false,
                            status: sekai::BodyStatus::OnGround,
                            linear_velocity: Vector2::ZERO,
                            angular_velocity: 0.0,
                            body_type: sekai::BodyType::Dynamic,
                        })
                        .unwrap(),
                ),
            )],
        };
        entities.apply_serialized_entity_state(&mut serializer, &state);
        let physics = entities.inner.physics.get(&EntityUid::new(120)).unwrap();
        assert!(!physics.awake);
        assert_eq!(physics.linear_velocity, Vector2::ZERO);
    }

    #[test]
    fn client_entity_manager_applies_non_sleeping_and_fixed_rotation_physics_state_semantically() {
        let mut entities = ClientEntityManager::new();
        let mut serializer = RobustSerializer::new();
        let state = SerializedEntityState {
            uid: EntityUid::new(121),
            component_changes: vec![SerializedComponentChange::new(
                5,
                true,
                false,
                Some(
                    serializer
                        .serialize_component_state(&PhysicsComponentState {
                            can_collide: true,
                            awake: false,
                            sleeping_allowed: false,
                            fixed_rotation: true,
                            status: sekai::BodyStatus::InAir,
                            linear_velocity: Vector2::new(1.0, 0.0),
                            angular_velocity: 2.0,
                            body_type: sekai::BodyType::Dynamic,
                        })
                        .unwrap(),
                ),
            )],
        };
        entities.apply_serialized_entity_state(&mut serializer, &state);
        let physics = entities.inner.physics.get(&EntityUid::new(121)).unwrap();
        assert!(physics.awake);
        assert!(!physics.sleeping_allowed);
        assert!(physics.fixed_rotation);
        assert_eq!(physics.angular_velocity, 0.0);
        assert_eq!(physics.body_status, sekai::BodyStatus::InAir);
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
                                        PhysShape::Aabb(AabbShape::new(
                                            Box2::new(-1.0, -1.0, 1.0, 1.0),
                                            0.0,
                                        )),
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
        assert_eq!(
            entities
                .inner
                .entity_lookups
                .get(&EntityUid::new(13))
                .unwrap()
                .entities
                .len(),
            1
        );
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
        assert!(entities.inner.has_physics_runtime_owner(EntityUid::new(13)));
        assert_eq!(
            entities.inner.owner_auto_clear_forces(EntityUid::new(13)),
            Some(true)
        );
        assert!(
            entities
                .inner
                .owner_contains_body(EntityUid::new(13), EntityUid::new(13))
        );
        assert!(
            entities
                .inner
                .owner_contains_awake_body(EntityUid::new(13), EntityUid::new(13))
        );
    }

    #[test]
    fn client_entity_manager_applies_serialized_collision_wake_state() {
        let mut entities = ClientEntityManager::new();
        let mut serializer = RobustSerializer::new();
        let state = SerializedEntityState {
            uid: EntityUid::new(131),
            component_changes: vec![SerializedComponentChange::new(
                12,
                true,
                false,
                Some(
                    serializer
                        .serialize_component_state(&CollisionWakeComponentState { enabled: false })
                        .unwrap(),
                ),
            )],
        };
        entities.apply_serialized_entity_state(&mut serializer, &state);
        let collision_wake = entities
            .inner
            .collision_wakes
            .get(&EntityUid::new(131))
            .unwrap();
        assert!(!collision_wake.enabled);
    }

    #[test]
    fn client_entity_manager_applies_serialized_collide_on_anchor_state() {
        let mut entities = ClientEntityManager::new();
        let mut serializer = RobustSerializer::new();
        let state = SerializedEntityState {
            uid: EntityUid::new(132),
            component_changes: vec![SerializedComponentChange::new(
                13,
                true,
                false,
                Some(
                    serializer
                        .serialize_component_state(&CollideOnAnchorComponentState { enable: true })
                        .unwrap(),
                ),
            )],
        };
        entities.apply_serialized_entity_state(&mut serializer, &state);
        let collide_on_anchor = entities
            .inner
            .collide_on_anchors
            .get(&EntityUid::new(132))
            .unwrap();
        assert!(collide_on_anchor.enable);
    }

    #[test]
    fn client_entity_manager_creates_map_owner_and_parents_grids_from_map_data() {
        let mut entities = ClientEntityManager::new();
        entities.inner.apply_game_state_map_data(&GameStateMapData {
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

        let map_uid = entities.inner.map_entity_for(MapId::new(7)).unwrap();
        let grid_uid = entities.inner.grid_entity_for(GridId::new(11)).unwrap();
        let transform = entities.inner.transforms.get(&grid_uid).unwrap();
        assert_eq!(transform.parent, map_uid);
        assert_eq!(transform.grid_id, GridId::new(11));
        assert_eq!(transform.map_id, MapId::new(7));
    }

    #[test]
    fn client_entity_manager_rebuilds_lookup_and_physics_runtime_after_grid_motion() {
        let mut entities = ClientEntityManager::new();
        entities.inner.apply_game_state_map_data(&GameStateMapData {
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

        let map_uid = entities.inner.map_entity_for(MapId::new(7)).unwrap();
        let grid_uid = entities.inner.grid_entity_for(GridId::new(11)).unwrap();

        let uid = entities.create_entity(None, EntityUid::new(250));
        entities.inner.initialize_entity(uid);
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

        entities.inner.rebuild_runtime_state();
        assert_eq!(
            entities.inner.entities_in_grid_aabb(
                GridId::new(11),
                Box2::new(-1.0, -1.0, 1.0, 1.0),
                false
            ),
            vec![uid]
        );

        entities.inner.apply_game_state_map_data(&GameStateMapData {
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
        entities.inner.rebuild_runtime_state();

        assert_eq!(
            entities.inner.entities_in_grid_aabb(
                GridId::new(11),
                Box2::new(9.0, -1.0, 11.0, 1.0),
                false
            ),
            vec![uid]
        );
        assert_eq!(
            entities
                .inner
                .query_aabb_entities(map_uid, Box2::new(9.0, -1.0, 11.0, 1.0)),
            vec![uid]
        );
    }

    #[test]
    fn client_entity_manager_sync_map_physics_runtime_creates_runtime_components_on_demand() {
        let mut entities = ClientEntityManager::new();
        let map_uid = entities.ensure_map_entity(MapId::new(8));
        assert!(!entities.inner.has_map_broadphase(MapId::new(8)));
        assert!(!entities.inner.has_map_physics_runtime(MapId::new(4)));

        let uid = entities.create_entity(None, EntityUid::new(320));
        entities.inner.initialize_entity(uid);
        let _ = entities.inner.apply_transform_state(
            uid,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(8),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(uid);
        body.can_collide = true;
        body.set_body_type(sekai::BodyType::Dynamic);
        body.awake = true;
        let _ = entities.inner.insert_fixture_and_reconcile(
            uid,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        entities
            .inner
            .refresh_map_physics_runtime_many([MapId::new(8)]);

        assert!(entities.inner.has_map_broadphase(MapId::new(8)));
        assert!(entities.inner.has_map_physics_runtime(MapId::new(8)));
        assert_eq!(
            entities
                .inner
                .query_aabb_entities(map_uid, Box2::new(-1.0, -1.0, 1.0, 1.0)),
            vec![uid]
        );
        assert!(entities.inner.map_contains_body(MapId::new(8), uid));
        assert!(entities.inner.map_contains_awake_body(MapId::new(8), uid));
    }

    #[test]
    fn client_entity_manager_rebuild_runtime_infers_map_membership_from_parents() {
        let mut entities = ClientEntityManager::new();
        let map_uid = entities.ensure_map_entity(MapId::new(3));
        let grid_uid = entities.ensure_grid_entity(GridId::new(21));
        entities
            .inner
            .map_grid_components
            .get_mut(&grid_uid)
            .unwrap()
            .chunk_size = 8;
        entities.inner.map_grids.insert(
            grid_uid,
            sekai::MapGrid::new(MapId::new(3), grid_uid, GridId::new(21), 8),
        );
        let _ = entities.inner.set_parent(grid_uid, map_uid);

        let uid = entities.create_entity(None, EntityUid::new(301));
        entities.inner.initialize_entity(uid);
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

        entities.inner.rebuild_runtime_state();
        let transform = entities.inner.transforms.get(&uid).unwrap();
        assert_eq!(transform.map_id, MapId::new(3));
        assert_eq!(transform.grid_id, GridId::new(21));
    }
}
