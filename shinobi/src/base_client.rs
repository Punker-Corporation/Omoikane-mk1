use crate::{
    client_entity_manager::ClientEntityManager, client_game_state_manager::ClientGameStateManager,
    client_net_manager::ClientNetManager, input_system::InputSystem, physics_system::PhysicsSystem,
    player_manager::PlayerManager, transform_system::TransformSystem,
};
#[cfg(test)]
use daikoku::DaikokuServer;
use daikoku::{BoundKeyFunction, BoundKeyState, FullInputCmdMessage};
use keisan::Vector2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientOptions {
    pub username: String,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            username: "shinobi".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientRunLevel {
    Initialize,
    Connected,
    InGame,
}

pub struct BaseClient {
    pub(crate) options: ClientOptions,
    pub(crate) run_level: ClientRunLevel,
    pub(crate) entities: ClientEntityManager,
    pub(crate) players: PlayerManager,
    pub(crate) network: ClientNetManager,
    pub(crate) game_states: ClientGameStateManager,
    pub(crate) input: InputSystem,
    pub(crate) transforms: TransformSystem,
    pub(crate) physics: PhysicsSystem,
}

impl BaseClient {
    pub fn new(options: ClientOptions) -> Self {
        Self {
            options,
            run_level: ClientRunLevel::Initialize,
            entities: ClientEntityManager::new(),
            players: PlayerManager::new(),
            network: ClientNetManager::new(),
            game_states: ClientGameStateManager::new(),
            input: InputSystem::new(),
            transforms: TransformSystem::new(),
            physics: PhysicsSystem::new(),
        }
    }

    pub fn startup(&mut self, user_id: impl Into<String>) {
        self.network.connect();
        self.players.startup(user_id, self.options.username.clone());
        self.network.request_player_list();
        self.run_level = ClientRunLevel::Connected;
    }

    pub fn shutdown(&mut self) {
        self.network.disconnect();
        self.players.shutdown();
        self.game_states.reset();
        self.input.clear();
        self.run_level = ClientRunLevel::Initialize;
    }

    pub fn dispatch_input(&mut self, input: FullInputCmdMessage) -> u32 {
        let sequence = self
            .game_states
            .input_command_dispatched(&mut self.network, input.clone());
        self.input
            .predict_input_command(&mut self.entities, &self.players, &input);
        sequence
    }

    pub fn handle_local_input(
        &mut self,
        function: impl Into<BoundKeyFunction>,
        state: BoundKeyState,
    ) -> bool {
        let Some(controlled) = self.players.controlled_entity() else {
            return false;
        };

        let world_offset = self
            .entities
            .inner
            .local_position(controlled)
            .unwrap_or(Vector2::ZERO);

        let message = self.input.build_local_input(
            self.game_states.last_processed_tick + 1,
            0,
            function,
            state,
            controlled,
            world_offset,
        );

        self.input.handle_input_command(
            &mut self.entities,
            &self.players,
            &mut self.game_states,
            &mut self.network,
            message.input_function_id.clone(),
            message,
            false,
        )
    }

    pub fn tick_update(&mut self) {
        self.process_network_inbound();
        if let Some((_state, context)) = self
            .game_states
            .apply_next_state_with_runtime_context(&mut self.entities, &mut self.players)
        {
            self.handle_applied_state(context);
        }
        self.entities
            .tick_update(self.game_states.last_processed_tick);
        let _ = self
            .input
            .apply_held_movement_state(&mut self.entities, &self.players);
        self.physics.update(&mut self.entities, 0.016);
        self.transforms.frame_update(&mut self.entities, 0.5);
    }

    pub fn run_level(&self) -> ClientRunLevel {
        self.run_level
    }

    #[cfg(test)]
    fn flush_to_server(&mut self, server: &mut DaikokuServer) {
        let local_user = self
            .players
            .local_player()
            .map(|player| player.user_id.clone())
            .unwrap_or_default();
        let batch = self.network.take_outbound_batch();
        for ack in batch.acks {
            server.ack_state(&local_user, ack.sequence);
        }
        for input in batch.inputs {
            let _ = server.queue_input(&local_user, input);
        }
        for message in batch.entities {
            let _ = server.queue_entity(&local_user, message);
        }
        for _ in 0..batch.player_list_requests {
            let _ = server.queue_player_list_request(&local_user);
        }
    }

    fn process_network_inbound(&mut self) {
        let batch = self.network.take_inbound_batch();
        for state in batch.states {
            self.game_states
                .handle_state_message(&mut self.network, state);
        }
        for player_list in batch.player_lists {
            self.players.apply_player_states(&player_list.plyrs, true);
        }
        for message in batch.entities {
            self.entities
                .handle_entity_network_message(self.game_states.last_processed_tick, message);
        }
    }

    fn handle_applied_state(
        &mut self,
        context: crate::client_game_state_manager::GameStateRuntimeApplyContext,
    ) {
        self.input.replay_pending_inputs(
            &mut self.entities,
            &self.players,
            &context.pending_inputs,
        );
        self.transforms.queue_pending_snapshot_lerps(
            self.entities.take_pending_transform_lerps(),
            context
                .pending_for_local
                .then_some(context.local_controlled)
                .flatten(),
        );
        self.run_level = ClientRunLevel::InGame;
        if let (true, Some(controlled)) = (context.pending_for_local, context.local_controlled) {
            self.physics.suppress_prediction_once(controlled);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BaseClient, ClientOptions, ClientRunLevel};
    use butsuri::{AabbShape, CollisionRay, Fixture, Joint, JointType, PhysShape};
    use daikoku::{
        BoundKeyState, DaikokuServer, EntityMessageType, FullInputCmdMessage, MsgEntity, MsgState,
        ServerOptions,
    };
    use jikan::GameTick;
    use keisan::{Angle, Box2, Vector2, Vector2i};
    use sekai::GameState;
    use sekai::{
        EntityCoordinates, EntityUid, GridId, MapId, RobustSerializer, ScreenCoordinates,
        SerializedComponentChange, SerializedEntityState, TransformComponentState, WindowId,
    };

    const TRANSFORM_NET_ID: u16 = 2;
    const MAP_NET_ID: u16 = 3;
    const MAP_GRID_NET_ID: u16 = 4;

    fn feed_server_states_to_client(
        client: &mut BaseClient,
        server: &mut DaikokuServer,
        user_id: &str,
    ) {
        for message in server.take_outbox(user_id) {
            if let daikoku::OutboundMessage::State(state) = message {
                client.network.receive_state(state);
            }
        }
    }

    #[test]
    fn base_client_starts_requests_player_list_and_applies_state() {
        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        assert_eq!(client.network.take_outbound_batch().player_list_requests, 1);
        client.network.receive_state(MsgState::new(GameState {
            from_sequence: GameTick::ZERO,
            to_sequence: GameTick::new(3),
            last_processed_input: 0,
            entity_states: Vec::new(),
            player_states: Vec::new(),
            entity_deletions: Vec::new(),
            map_data: None,
            extrapolated: false,
            payload_size: 0,
        }));
        client.network.receive_state(MsgState::new(GameState {
            from_sequence: GameTick::new(3),
            to_sequence: GameTick::new(4),
            last_processed_input: 0,
            entity_states: Vec::new(),
            player_states: Vec::new(),
            entity_deletions: Vec::new(),
            map_data: None,
            extrapolated: false,
            payload_size: 0,
        }));
        client.network.receive_state(MsgState::new(GameState {
            from_sequence: GameTick::new(4),
            to_sequence: GameTick::new(5),
            last_processed_input: 0,
            entity_states: Vec::new(),
            player_states: Vec::new(),
            entity_deletions: Vec::new(),
            map_data: None,
            extrapolated: false,
            payload_size: 0,
        }));
        client.tick_update();
        assert_eq!(client.run_level(), ClientRunLevel::InGame);
        assert_eq!(client.network.take_outbound_batch().acks.len(), 3);
        client.shutdown();
        assert_eq!(client.run_level(), ClientRunLevel::Initialize);
    }

    #[test]
    fn client_can_consume_real_server_state_roundtrip() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        let map_id = server.create_map(None);
        let grid_id = server.create_grid(map_id, None, 8);
        server.set_map_tick(GameTick::ZERO);
        assert!(server.set_tile(
            grid_id,
            Vector2i::new(0, 0),
            sekai::Tile::new(7, sekai::TileRenderFlag(0), 0),
        ));
        let uid = server.create_entity_uninitialized(Some("mob"));
        server.initialize_entity(uid);
        assert!(server.set_entity_map(uid, map_id));
        assert!(server.configure_physics_body(
            uid,
            Some(sekai::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        assert!(server.insert_fixture(
            uid,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        ));
        let mut joint = Joint::new(uid.raw(), 999, JointType::Distance);
        joint.id = "rope".to_string();
        assert!(server.add_joint_between(joint));
        server.attach_player("u1", uid, false);
        server.set_replication_tick(GameTick::FIRST);
        assert!(server.join_player("u1"));
        let mut server_messages = Vec::new();
        for _ in 0..3 {
            server.tick_update(0.016);
            server_messages.extend(server.take_outbox("u1"));
        }

        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        for message in server_messages {
            if let daikoku::OutboundMessage::State(state) = message {
                client.network.receive_state(state);
            }
        }
        client.tick_update();

        assert_eq!(client.run_level(), ClientRunLevel::InGame);
        assert_eq!(
            client.players.local_player().unwrap().controlled_entity,
            Some(uid)
        );
        assert_eq!(
            client
                .entities
                .inner
                .fixtures
                .get(&uid)
                .unwrap()
                .fixture_count(),
            1
        );
        assert_eq!(
            client
                .entities
                .inner
                .joint_components
                .get(&uid)
                .unwrap()
                .joint_count(),
            1
        );
        let client_grid_uid = client.entities.inner.grid_entity_for(grid_id).unwrap();
        assert_eq!(
            client
                .entities
                .inner
                .map_grids
                .get(&client_grid_uid)
                .unwrap()
                .get_tile_ref(Vector2i::new(0, 0))
                .tile
                .type_id,
            7
        );
        assert_eq!(
            client
                .entities
                .inner
                .entities_in_map_aabb(map_id, Box2::new(-2.0, -2.0, 2.0, 2.0)),
            vec![uid]
        );
        assert_eq!(
            client.entities.inner.intersect_ray(
                map_id,
                CollisionRay::new(Vector2::new(-5.0, 0.0), Vector2::UNIT_X, -1),
                10.0,
                true,
            )[0]
            .entity,
            uid
        );
    }

    #[test]
    fn client_roundtrips_map_gravity_and_uses_it_for_local_prediction() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        let map_id = server.create_map(Some(MapId::new(11)));
        assert!(server.set_map_gravity(map_id, Vector2::new(0.0, -10.0)));
        assert!(server.set_map_auto_clear_forces(map_id, true));

        let uid = server.create_entity_uninitialized(Some("mob"));
        server.initialize_entity(uid);
        let _ = server.apply_transform_state(
            uid,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id,
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(server.configure_physics_body(
            uid,
            Some(sekai::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        assert!(server.insert_fixture(
            uid,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        ));
        server.attach_player("u1", uid, false);
        server.set_replication_tick(GameTick::FIRST);
        assert!(server.join_player("u1"));

        server.tick_update(0.0);

        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        feed_server_states_to_client(&mut client, &mut server, "u1");
        client.process_network_inbound();
        if let Some((_state, context)) = client
            .game_states
            .apply_next_state_with_runtime_context(&mut client.entities, &mut client.players)
        {
            client.handle_applied_state(context);
        }

        let _client_map_uid = client.entities.inner.map_entity_for(map_id).unwrap();
        assert_eq!(
            client.entities.inner.map_gravity(map_id),
            Some(Vector2::new(0.0, -10.0))
        );
        assert_eq!(
            client.entities.inner.map_auto_clear_forces(map_id),
            Some(true)
        );

        let _ = client
            .input
            .apply_held_movement_state(&mut client.entities, &client.players);
        client.entities.inner.physics.get_mut(&uid).unwrap().predict = true;
        client.physics.update(&mut client.entities, 0.016);

        let physics = client.entities.inner.physics.get(&uid).unwrap();
        assert_eq!(physics.linear_velocity, Vector2::new(0.0, -0.159488));
        assert_eq!(physics.angular_velocity, 0.0);
        assert_eq!(physics.force, Vector2::ZERO);
        assert_eq!(physics.torque, 0.0);
        let transform = client.entities.inner.transforms.get(&uid).unwrap();
        assert!((transform.local_position.x - 0.0).abs() < 0.0001);
        assert!(transform.local_position.y < 0.0);
        assert_eq!(transform.local_rotation, Angle::ZERO);
    }

    #[test]
    fn client_roundtrips_paused_map_state_from_server() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));

        let map_id = server.create_map(Some(MapId::new(19)));
        let uid = server.create_entity_uninitialized(Some("mob"));
        server.initialize_entity(uid);
        let _ = server.apply_transform_state(
            uid,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id,
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        server.attach_player("u1", uid, false);
        server.set_replication_tick(GameTick::FIRST);
        assert!(server.join_player("u1"));
        assert!(server.set_map_paused(map_id, true));

        server.tick_update(0.0);

        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        feed_server_states_to_client(&mut client, &mut server, "u1");
        client.tick_update();

        assert!(client.entities.inner.is_map_paused(map_id));
    }

    #[test]
    fn client_roundtrips_incremental_chunk_updates_and_deletions_from_server() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        let map_id = server.create_map(Some(sekai::MapId::new(2)));
        let grid_id = server.create_grid(map_id, Some(sekai::GridId::new(9)), 4);
        let controlled = server.create_entity_uninitialized(Some("mob"));
        server.initialize_entity(controlled);
        assert!(server.set_entity_map(controlled, map_id));
        server.attach_player("u1", controlled, false);
        server.set_replication_tick(GameTick::FIRST);
        assert!(server.join_player("u1"));

        server.set_map_tick(GameTick::new(1));
        assert!(server.set_tile(
            grid_id,
            Vector2i::new(0, 0),
            sekai::Tile::new(1, sekai::TileRenderFlag(0), 0),
        ));
        server.tick_update(0.016);
        let initial = server.take_outbox("u1");

        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        for message in initial {
            if let daikoku::OutboundMessage::State(state) = message {
                client.network.receive_state(state);
            }
        }
        client.tick_update();
        client.flush_to_server(&mut server);

        server.set_replication_tick(GameTick::new(2));
        assert!(server.set_tile(
            grid_id,
            Vector2i::new(4, 0),
            sekai::Tile::new(9, sekai::TileRenderFlag(0), 0),
        ));
        server.tick_update(0.016);
        let changed = server.take_outbox("u1");
        for message in changed {
            if let daikoku::OutboundMessage::State(state) = message {
                client.network.receive_state(state);
            }
        }
        client.tick_update();
        client.flush_to_server(&mut server);

        let client_grid_uid = client.entities.inner.grid_entity_for(grid_id).unwrap();
        let grid = client
            .entities
            .inner
            .map_grids
            .get(&client_grid_uid)
            .unwrap();
        assert_eq!(grid.get_tile_ref(Vector2i::new(4, 0)).tile.type_id, 9);
        assert_eq!(grid.get_tile_ref(Vector2i::new(0, 0)).tile.type_id, 1);

        server.set_replication_tick(GameTick::new(3));
        assert!(server.remove_chunk(grid_id, Vector2i::new(0, 0)));
        server.tick_update(0.016);
        let deleted = server.take_outbox("u1");
        for message in deleted {
            if let daikoku::OutboundMessage::State(state) = message {
                client.network.receive_state(state);
            }
        }
        client.tick_update();

        let grid = client
            .entities
            .inner
            .map_grids
            .get(&client_grid_uid)
            .unwrap();
        assert!(grid.get_tile_ref(Vector2i::new(0, 0)).tile.is_empty());
        assert!(grid.try_get_chunk(Vector2i::new(0, 0)).is_none());
        assert!(grid.try_get_chunk(Vector2i::new(1, 0)).is_some());
    }

    #[test]
    fn client_roundtrips_incremental_transform_and_grid_component_deletions_from_server() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        let map_id = server.create_map(Some(MapId::new(2)));
        let grid_id = server.create_grid(map_id, Some(GridId::new(9)), 4);
        let grid_uid = server.grid_entity_for(grid_id).unwrap();
        server.set_map_tick(GameTick::new(1));
        assert!(server.set_tile(
            grid_id,
            Vector2i::new(0, 0),
            sekai::Tile::new(5, sekai::TileRenderFlag(0), 0),
        ));

        let controlled = server.create_entity_uninitialized(Some("mob"));
        server.initialize_entity(controlled);
        assert!(server.set_entity_map(controlled, map_id));
        server.attach_player("u1", controlled, false);
        server.set_replication_tick(GameTick::FIRST);
        assert!(server.join_player("u1"));

        let child = server.create_entity_uninitialized(Some("item"));
        server.initialize_entity(child);
        let _ = server.apply_transform_state(
            child,
            TransformComponentState {
                local_position: Vector2::new(0.5, 0.5),
                rotation: Angle::ZERO,
                parent_id: grid_uid,
                map_id,
                grid_id,
                no_local_rotation: false,
                anchored: false,
            },
        );
        server.reconcile_transform_runtime(child, None);

        server.tick_update(0.016);

        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        feed_server_states_to_client(&mut client, &mut server, "u1");
        client.tick_update();
        client.flush_to_server(&mut server);

        let client_grid_uid = client.entities.inner.grid_entity_for(grid_id).unwrap();
        assert!(
            client
                .entities
                .inner
                .map_grids
                .contains_key(&client_grid_uid)
        );
        assert_eq!(
            client
                .entities
                .inner
                .map_grids
                .get(&client_grid_uid)
                .unwrap()
                .get_tile_ref(Vector2i::new(0, 0))
                .tile
                .type_id,
            5
        );
        assert!(client.entities.inner.transforms.contains_key(&child));

        server.set_current_tick(GameTick::new(2));
        assert!(server.remove_component_by_net_id(grid_uid, MAP_GRID_NET_ID));
        assert!(server.remove_component_by_net_id(child, TRANSFORM_NET_ID));

        server.tick_update(0.016);
        feed_server_states_to_client(&mut client, &mut server, "u1");
        client.tick_update();

        assert!(
            !client
                .entities
                .inner
                .map_grid_components
                .contains_key(&client_grid_uid)
        );
        assert!(
            !client
                .entities
                .inner
                .map_grids
                .contains_key(&client_grid_uid)
        );
        assert!(!client.entities.inner.transforms.contains_key(&child));
    }

    #[test]
    fn client_roundtrips_incremental_map_component_deletion_from_server() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        let map_id = server.create_map(Some(MapId::new(4)));
        let map_uid = server.map_entity_for(map_id).unwrap();
        let grid_id = server.create_grid(map_id, Some(GridId::new(12)), 4);

        server.set_map_tick(GameTick::new(1));
        assert!(server.set_tile(
            grid_id,
            Vector2i::new(0, 0),
            sekai::Tile::new(3, sekai::TileRenderFlag(0), 0),
        ));

        let controlled = server.create_entity_uninitialized(Some("mob"));
        server.initialize_entity(controlled);
        assert!(server.set_entity_map(controlled, map_id));
        server.attach_player("u1", controlled, false);
        server.set_replication_tick(GameTick::FIRST);
        assert!(server.join_player("u1"));
        server.tick_update(0.016);

        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        feed_server_states_to_client(&mut client, &mut server, "u1");
        client.tick_update();
        client.flush_to_server(&mut server);

        let client_map_uid = client.entities.inner.map_entity_for(map_id).unwrap();
        let client_grid_uid = client.entities.inner.grid_entity_for(grid_id).unwrap();
        assert!(client.entities.inner.has_map_broadphase(map_id));
        assert!(client.entities.inner.has_map_physics_runtime(map_id));
        assert_eq!(
            client
                .entities
                .inner
                .transforms
                .get(&client_grid_uid)
                .unwrap()
                .parent,
            client_map_uid
        );

        server.set_current_tick(GameTick::new(2));
        assert!(server.remove_component_by_net_id(map_uid, MAP_NET_ID));

        server.tick_update(0.016);
        feed_server_states_to_client(&mut client, &mut server, "u1");
        client.tick_update();

        assert!(client.entities.inner.map_entity_for(map_id).is_none());
        assert!(
            !client
                .entities
                .inner
                .map_components
                .contains_key(&client_map_uid)
        );
        assert!(!client.entities.inner.has_map_broadphase(map_id));
        assert!(!client.entities.inner.has_map_physics_runtime(map_id));
        if let Some(grid_transform) = client.entities.inner.transforms.get(&client_grid_uid) {
            assert_eq!(grid_transform.parent, EntityUid::INVALID);
            assert_eq!(grid_transform.map_id, MapId::NULLSPACE);
            assert_eq!(grid_transform.grid_id, grid_id);
            assert_eq!(
                client
                    .entities
                    .inner
                    .map_grids
                    .get(&client_grid_uid)
                    .unwrap()
                    .parent_map_id,
                MapId::NULLSPACE
            );
            assert_eq!(
                client
                    .entities
                    .inner
                    .map_grids
                    .get(&client_grid_uid)
                    .unwrap()
                    .get_tile_ref(Vector2i::new(0, 0))
                    .tile
                    .type_id,
                3
            );
        } else {
            assert!(
                !client
                    .entities
                    .inner
                    .map_grid_components
                    .contains_key(&client_grid_uid)
            );
            assert!(
                !client
                    .entities
                    .inner
                    .map_grids
                    .contains_key(&client_grid_uid)
            );
        }
    }

    #[test]
    fn client_roundtrips_incremental_physics_and_fixtures_deletions_from_server() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        let map_id = server.create_map(Some(MapId::new(5)));

        let controlled = server.create_entity_uninitialized(Some("mob"));
        server.initialize_entity(controlled);
        let _ = server.apply_transform_state(
            controlled,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id,
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(server.configure_physics_body(
            controlled,
            Some(sekai::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        assert!(server.insert_fixture(
            controlled,
            Fixture::new(
                "controlled",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        ));
        server.attach_player("u1", controlled, false);
        server.set_replication_tick(GameTick::FIRST);
        assert!(server.join_player("u1"));

        let other = server.create_entity_uninitialized(Some("other"));
        server.initialize_entity(other);
        let _ = server.apply_transform_state(
            other,
            TransformComponentState {
                local_position: Vector2::new(0.5, 0.0),
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id,
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(server.configure_physics_body(
            other,
            Some(sekai::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        assert!(server.insert_fixture(
            other,
            Fixture::new(
                "other",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        ));

        server.tick_update(0.016);

        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        feed_server_states_to_client(&mut client, &mut server, "u1");
        client.tick_update();
        client.flush_to_server(&mut server);
        let _ = client.entities.inner.drain_map_contact_events(map_id);

        assert_eq!(client.entities.inner.map_contact_count(map_id), 1);

        server.set_current_tick(GameTick::new(2));
        assert!(server.remove_component_by_net_id(controlled, 5));
        assert!(server.remove_component_by_net_id(other, 7));

        server.tick_update(0.016);
        feed_server_states_to_client(&mut client, &mut server, "u1");
        client.tick_update();

        assert_eq!(client.entities.inner.map_contact_count(map_id), 0);
        let events = client.entities.inner.drain_map_contact_events(map_id);
        assert!(events.is_empty());
        assert!(
            client
                .entities
                .inner
                .entities_in_map_aabb(map_id, Box2::new(-1.0, -1.0, 1.0, 1.0))
                .is_empty()
        );
        assert!(!client.entities.inner.physics.contains_key(&controlled));
        assert!(!client.entities.inner.fixtures.contains_key(&other));
    }

    #[test]
    fn client_roundtrips_incremental_joint_component_deletion_from_server() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        let map_id = server.create_map(Some(MapId::new(6)));

        let controlled = server.create_entity_uninitialized(Some("mob"));
        server.initialize_entity(controlled);
        let _ = server.apply_transform_state(
            controlled,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id,
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(server.configure_physics_body(
            controlled,
            Some(sekai::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        assert!(server.insert_fixture(
            controlled,
            Fixture::new(
                "controlled",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        ));
        server.attach_player("u1", controlled, false);
        server.set_replication_tick(GameTick::FIRST);
        assert!(server.join_player("u1"));

        let other = server.create_entity_uninitialized(Some("other"));
        server.initialize_entity(other);
        let _ = server.apply_transform_state(
            other,
            TransformComponentState {
                local_position: Vector2::new(0.5, 0.0),
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id,
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(server.configure_physics_body(
            other,
            Some(sekai::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        assert!(server.insert_fixture(
            other,
            Fixture::new(
                "other",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        ));

        let mut joint = Joint::new(controlled.raw(), other.raw(), JointType::Distance);
        joint.id = "rope".to_string();
        joint.collide_connected = false;
        assert!(server.add_joint_between(joint));

        server.tick_update(0.016);

        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        feed_server_states_to_client(&mut client, &mut server, "u1");
        client.tick_update();
        client.flush_to_server(&mut server);
        let _ = client.entities.inner.drain_map_contact_events(map_id);

        assert_eq!(client.entities.inner.map_contact_count(map_id), 0);

        server.set_current_tick(GameTick::new(2));
        assert!(server.remove_component_by_net_id(controlled, 8));

        server.tick_update(0.016);
        feed_server_states_to_client(&mut client, &mut server, "u1");
        client.tick_update();

        assert_eq!(client.entities.inner.map_contact_count(map_id), 1);
        let events = client.entities.inner.drain_map_contact_events(map_id);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, butsuri::ContactStatus::StartTouching);
        assert!(
            !client
                .entities
                .inner
                .joint_components
                .contains_key(&controlled)
        );
        assert_eq!(
            client
                .entities
                .inner
                .joint_components
                .get(&other)
                .unwrap()
                .joint_count(),
            0
        );
    }

    #[test]
    fn client_roundtrips_physics_deletion_with_implicit_joint_deletion_from_server() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        let map_id = server.create_map(Some(MapId::new(7)));

        let controlled = server.create_entity_uninitialized(Some("mob"));
        server.initialize_entity(controlled);
        let _ = server.apply_transform_state(
            controlled,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id,
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(server.configure_physics_body(
            controlled,
            Some(sekai::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        assert!(server.insert_fixture(
            controlled,
            Fixture::new(
                "controlled",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        ));
        server.attach_player("u1", controlled, false);
        server.set_replication_tick(GameTick::FIRST);
        assert!(server.join_player("u1"));

        let other = server.create_entity_uninitialized(Some("other"));
        server.initialize_entity(other);
        let _ = server.apply_transform_state(
            other,
            TransformComponentState {
                local_position: Vector2::new(0.5, 0.0),
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id,
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(server.configure_physics_body(
            other,
            Some(sekai::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        assert!(server.insert_fixture(
            other,
            Fixture::new(
                "other",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        ));

        let mut joint = Joint::new(controlled.raw(), other.raw(), JointType::Distance);
        joint.id = "rope".to_string();
        joint.collide_connected = false;
        assert!(server.add_joint_between(joint));

        server.tick_update(0.016);

        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        feed_server_states_to_client(&mut client, &mut server, "u1");
        client.tick_update();
        client.flush_to_server(&mut server);

        assert!(
            client
                .entities
                .inner
                .joint_components
                .contains_key(&controlled)
        );
        assert_eq!(
            client
                .entities
                .inner
                .joint_components
                .get(&other)
                .unwrap()
                .joint_count(),
            1
        );

        server.set_current_tick(GameTick::new(2));
        assert!(server.remove_component_by_net_id(controlled, 5));

        server.tick_update(0.016);
        feed_server_states_to_client(&mut client, &mut server, "u1");
        client.tick_update();

        assert!(!client.entities.inner.physics.contains_key(&controlled));
        assert!(
            !client
                .entities
                .inner
                .joint_components
                .contains_key(&controlled)
        );
        assert_eq!(
            client
                .entities
                .inner
                .joint_components
                .get(&other)
                .unwrap()
                .joint_count(),
            0
        );
    }

    #[test]
    fn client_flushes_inputs_and_acks_back_to_server() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        server.set_replication_tick(GameTick::FIRST);
        assert!(server.join_player("u1"));

        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        let seq = client.dispatch_input(FullInputCmdMessage::new(
            GameTick::FIRST,
            0,
            0,
            "MoveRight",
            BoundKeyState::Down,
            EntityCoordinates::new(EntityUid::new(1), Vector2::ZERO),
            ScreenCoordinates::new_xy(0.0, 0.0, WindowId::MAIN),
        ));
        assert_eq!(seq, 1);
        server.tick_update(0.016);
        for message in server.take_outbox("u1") {
            if let daikoku::OutboundMessage::State(state) = message {
                client.network.receive_state(state);
            }
        }
        client.tick_update();
        client.flush_to_server(&mut server);
        server.tick_update(0.016);
        assert_eq!(server.last_processed_input("u1"), Some(1));
    }

    #[test]
    fn client_replays_pending_inputs_after_authoritative_state() {
        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        let uid = client.entities.create_entity(None, EntityUid::new(40));
        client.entities.inner.initialize_entity(uid);
        assert!(client.players.attach_local_entity(uid));

        let seq = client.dispatch_input(FullInputCmdMessage::new(
            GameTick::FIRST,
            0,
            0,
            "MoveRight",
            BoundKeyState::Down,
            EntityCoordinates::new(uid, Vector2::ZERO),
            ScreenCoordinates::new_xy(0.0, 0.0, WindowId::MAIN),
        ));
        assert_eq!(seq, 1);
        assert_eq!(
            client
                .entities
                .inner
                .transforms
                .get(&uid)
                .unwrap()
                .local_position,
            Vector2::new(1.0, 0.0)
        );

        let mut serializer = RobustSerializer::new();
        client.network.receive_state(MsgState::new(GameState {
            from_sequence: GameTick::ZERO,
            to_sequence: GameTick::new(2),
            last_processed_input: 0,
            entity_states: vec![SerializedEntityState {
                uid,
                component_changes: vec![SerializedComponentChange::new(
                    2,
                    false,
                    false,
                    Some(
                        serializer
                            .serialize_component_state(&TransformComponentState {
                                local_position: Vector2::ZERO,
                                rotation: Angle::ZERO,
                                parent_id: EntityUid::INVALID,
                                map_id: sekai::MapId::NULLSPACE,
                                grid_id: sekai::GridId::INVALID,
                                no_local_rotation: false,
                                anchored: false,
                            })
                            .unwrap(),
                    ),
                )],
            }],
            player_states: Vec::new(),
            entity_deletions: Vec::new(),
            map_data: None,
            extrapolated: false,
            payload_size: 0,
        }));

        client.tick_update();
        assert_eq!(
            client
                .entities
                .inner
                .transforms
                .get(&uid)
                .unwrap()
                .local_position,
            Vector2::new(1.0, 0.0)
        );
    }

    #[test]
    fn base_client_handles_local_input_through_prediction_and_network_queue() {
        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        let uid = client.entities.create_entity(None, EntityUid::new(41));
        client.entities.inner.initialize_entity(uid);
        assert!(client.players.attach_local_entity(uid));

        assert!(!client.handle_local_input("MoveRight", BoundKeyState::Down));
        assert_eq!(
            client
                .entities
                .inner
                .transforms
                .get(&uid)
                .unwrap()
                .local_position,
            Vector2::new(1.0, 0.0)
        );
        assert_eq!(
            client
                .entities
                .inner
                .physics
                .get(&uid)
                .unwrap()
                .linear_velocity,
            Vector2::new(crate::input_system::InputSystem::MOVE_SPEED, 0.0)
        );
        assert_eq!(client.network.take_outbound_batch().inputs.len(), 1);
    }

    #[test]
    fn base_client_continues_predicting_held_input_across_frames() {
        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        let uid = client.entities.create_entity(None, EntityUid::new(42));
        client.entities.inner.initialize_entity(uid);
        assert!(client.players.attach_local_entity(uid));

        assert!(!client.handle_local_input("MoveRight", BoundKeyState::Down));
        assert_eq!(
            client
                .entities
                .inner
                .transforms
                .get(&uid)
                .unwrap()
                .local_position,
            Vector2::new(1.0, 0.0)
        );
        client.tick_update();
        assert_eq!(
            client
                .entities
                .inner
                .transforms
                .get(&uid)
                .unwrap()
                .local_position,
            Vector2::new(1.9968, 0.0)
        );
    }

    #[test]
    fn base_client_stops_predicted_integration_after_authoritative_sleep_state() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));

        let uid = server.create_entity_uninitialized(Some("mob"));
        server.initialize_entity(uid);
        assert!(server.configure_physics_body(
            uid,
            Some(sekai::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        assert!(server.set_body_linear_velocity(uid, Vector2::new(3.0, 0.0)));
        server.attach_player("u1", uid, false);
        server.set_replication_tick(GameTick::FIRST);
        assert!(server.join_player("u1"));
        server.tick_update(0.0);

        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        feed_server_states_to_client(&mut client, &mut server, "u1");
        client.tick_update();
        client.flush_to_server(&mut server);

        assert_eq!(
            client
                .entities
                .inner
                .physics
                .get(&uid)
                .unwrap()
                .linear_velocity,
            Vector2::new(2.9904, 0.0)
        );
        assert!(client.entities.inner.physics.get(&uid).unwrap().awake);

        server.set_current_tick(GameTick::new(2));
        assert!(server.set_body_awake(uid, false));
        server.tick_update(0.0);

        feed_server_states_to_client(&mut client, &mut server, "u1");
        client.tick_update();

        let physics = client.entities.inner.physics.get(&uid).unwrap();
        assert!(!physics.awake);
        assert_eq!(physics.linear_velocity, Vector2::ZERO);
        let position = client
            .entities
            .inner
            .transforms
            .get(&uid)
            .unwrap()
            .local_position;
        client.tick_update();
        assert_eq!(
            client
                .entities
                .inner
                .transforms
                .get(&uid)
                .unwrap()
                .local_position,
            position
        );
    }

    #[test]
    fn base_client_roundtrips_non_sleeping_fixed_rotation_physics_state_from_server() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));

        let uid = server.create_entity_uninitialized(Some("mob"));
        server.initialize_entity(uid);
        assert!(server.configure_physics_body(
            uid,
            Some(sekai::BodyType::Dynamic),
            Some(false),
            Some(true),
            None,
        ));
        let _ = server.set_body_sleeping_allowed(uid, true);
        assert!(server.set_body_angular_velocity(uid, 4.0));
        server.attach_player("u1", uid, false);
        server.set_replication_tick(GameTick::FIRST);
        assert!(server.join_player("u1"));

        server.tick_update(0.0);

        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        feed_server_states_to_client(&mut client, &mut server, "u1");
        client.tick_update();
        client.flush_to_server(&mut server);

        server.set_current_tick(GameTick::new(2));
        assert!(server.set_body_sleeping_allowed(uid, false));
        assert!(server.set_body_fixed_rotation(uid, true));
        assert!(server.set_body_status(uid, sekai::BodyStatus::InAir));
        server.tick_update(0.0);

        feed_server_states_to_client(&mut client, &mut server, "u1");
        client.tick_update();

        let physics = client.entities.inner.physics.get(&uid).unwrap();
        assert!(physics.awake);
        assert!(!physics.sleeping_allowed);
        assert!(physics.fixed_rotation);
        assert_eq!(physics.angular_velocity, 0.0);
        assert_eq!(physics.body_status, sekai::BodyStatus::InAir);
    }

    #[test]
    fn base_client_roundtrips_collision_wake_and_rebuilds_can_collide_semantics() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        let map_id = server.create_map(Some(MapId::new(18)));
        let grid_id = server.create_grid(map_id, Some(GridId::new(44)), 8);

        let uid = server.create_entity_uninitialized(Some("mob"));
        server.initialize_entity(uid);
        let grid_uid = server.grid_entity_for(grid_id).unwrap();
        let _ = server.apply_transform_state(
            uid,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: grid_uid,
                map_id,
                grid_id,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(server.configure_physics_body(
            uid,
            Some(sekai::BodyType::Dynamic),
            Some(false),
            Some(true),
            None,
        ));
        assert!(server.set_collision_wake_enabled(uid, true));
        server.attach_player("u1", uid, false);
        server.set_replication_tick(GameTick::FIRST);
        assert!(server.join_player("u1"));
        server.tick_update(0.0);

        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        feed_server_states_to_client(&mut client, &mut server, "u1");
        client.tick_update();

        assert!(!client.entities.inner.physics.get(&uid).unwrap().can_collide);
        assert!(!client.entities.inner.physics.get(&uid).unwrap().awake);
        assert!(
            client
                .entities
                .inner
                .collision_wakes
                .get(&uid)
                .unwrap()
                .enabled
        );
    }

    #[test]
    fn client_roundtrips_entity_messages() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));

        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        client.network.send_entity(MsgEntity {
            message_type: EntityMessageType::SystemMessage,
            system_message: Some("wave".to_string()),
            component_message: None,
            entity_uid: EntityUid::new(1),
            net_id: 0,
            sequence: 2,
            source_tick: GameTick::FIRST,
        });
        client.flush_to_server(&mut server);
        server.tick_update(0.016);
        assert_eq!(
            server.received_system_messages(),
            vec![("u1".to_string(), "wave".to_string())]
        );

        client.network.receive_entity(MsgEntity {
            message_type: EntityMessageType::ComponentMessage,
            system_message: None,
            component_message: Some("blink".to_string()),
            entity_uid: EntityUid::new(3),
            net_id: 5,
            sequence: 3,
            source_tick: GameTick::ZERO,
        });
        client.tick_update();
        assert_eq!(client.entities.received_component_messages.len(), 1);
    }

    #[test]
    fn client_roundtrips_player_list_requests() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));

        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        client.flush_to_server(&mut server);
        server.tick_update(0.016);
        for message in server.take_outbox("u1") {
            if let daikoku::OutboundMessage::PlayerList(list) = message {
                client.network.receive_player_list(list);
            }
        }
        client.tick_update();
        assert_eq!(client.players.session("u1").unwrap().name, "pedel");
    }

    #[test]
    fn base_client_interpolates_transform_snapshots() {
        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        let uid = EntityUid::new(22);
        client.entities.create_entity(None, uid);
        client.entities.inner.initialize_entity(uid);
        client
            .entities
            .inner
            .transforms
            .get_mut(&uid)
            .unwrap()
            .local_position = Vector2::new(0.0, 0.0);
        let mut serializer = RobustSerializer::new();
        client.network.receive_state(MsgState::new(GameState {
            from_sequence: GameTick::ZERO,
            to_sequence: GameTick::new(1),
            last_processed_input: 0,
            entity_states: vec![SerializedEntityState {
                uid,
                component_changes: vec![SerializedComponentChange::new(
                    2,
                    false,
                    false,
                    Some(
                        serializer
                            .serialize_component_state(&TransformComponentState {
                                local_position: Vector2::new(1.0, 0.0),
                                rotation: Angle::ZERO,
                                parent_id: EntityUid::INVALID,
                                map_id: sekai::MapId::NULLSPACE,
                                grid_id: sekai::GridId::INVALID,
                                no_local_rotation: false,
                                anchored: false,
                            })
                            .unwrap(),
                    ),
                )],
            }],
            player_states: Vec::new(),
            entity_deletions: Vec::new(),
            map_data: None,
            extrapolated: false,
            payload_size: 0,
        }));
        client.network.receive_state(MsgState::new(GameState {
            from_sequence: GameTick::new(1),
            to_sequence: GameTick::new(2),
            last_processed_input: 0,
            entity_states: Vec::new(),
            player_states: Vec::new(),
            entity_deletions: Vec::new(),
            map_data: None,
            extrapolated: false,
            payload_size: 0,
        }));
        client.network.receive_state(MsgState::new(GameState {
            from_sequence: GameTick::new(2),
            to_sequence: GameTick::new(3),
            last_processed_input: 0,
            entity_states: Vec::new(),
            player_states: Vec::new(),
            entity_deletions: Vec::new(),
            map_data: None,
            extrapolated: false,
            payload_size: 0,
        }));
        client.tick_update();
        assert_eq!(
            client
                .entities
                .inner
                .transforms
                .get(&uid)
                .unwrap()
                .local_position
                .x,
            0.5
        );
        client.tick_update();
        assert_eq!(
            client
                .entities
                .inner
                .transforms
                .get(&uid)
                .unwrap()
                .local_position
                .x,
            1.0
        );
    }
}
