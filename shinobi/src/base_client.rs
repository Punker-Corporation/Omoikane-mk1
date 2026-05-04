use crate::{
    ClientEntityManager, ClientGameStateManager, ClientNetManager, InputSystem, MapSystem,
    PhysicsSystem, PlayerManager, TransformSystem,
};
use daikoku::{BoundKeyFunction, BoundKeyState, DaikokuServer, FullInputCmdMessage};
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
    pub options: ClientOptions,
    pub run_level: ClientRunLevel,
    pub entities: ClientEntityManager,
    pub players: PlayerManager,
    pub network: ClientNetManager,
    pub game_states: ClientGameStateManager,
    pub input: InputSystem,
    pub transforms: TransformSystem,
    pub maps: MapSystem,
    pub physics: PhysicsSystem,
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
            maps: MapSystem::new(),
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
        let Some(controlled) = self
            .players
            .local_player()
            .and_then(|player| player.controlled_entity)
        else {
            return false;
        };

        let world_offset = self
            .entities
            .inner
            .transforms
            .get(&controlled)
            .map(|transform| transform.local_position)
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
        self.game_states.pump_network(&mut self.network);
        while let Some(player_list) = self.network.next_player_list() {
            self.players.apply_player_states(&player_list.plyrs, true);
        }
        while let Some(message) = self.network.next_entity() {
            self.entities
                .handle_entity_network_message(self.game_states.last_processed_tick, message);
        }
        if let Some(_state) = self
            .game_states
            .apply_next_state(&mut self.entities, &mut self.players)
        {
            let pending_inputs = self.game_states.pending_inputs_snapshot();
            let local_controlled = self
                .players
                .local_player()
                .and_then(|player| player.controlled_entity);
            let pending_for_local = local_controlled.is_some_and(|controlled| {
                pending_inputs
                    .iter()
                    .any(|input| input.coordinates.entity_id == controlled)
            });
            self.input
                .replay_pending_inputs(&mut self.entities, &self.players, &pending_inputs);
            for lerp in self.entities.take_pending_transform_lerps() {
                if pending_for_local && Some(lerp.uid) == local_controlled {
                    continue;
                }
                self.transforms.queue_snapshot_lerp(
                    lerp.uid,
                    lerp.source,
                    lerp.destination,
                    lerp.source_angle,
                    lerp.destination_angle,
                    lerp.parent,
                    lerp.parent,
                    lerp.source_anchored,
                    lerp.destination_anchored,
                );
            }
            self.run_level = ClientRunLevel::InGame;
            if pending_for_local && let Some(controlled) = local_controlled {
                self.physics.suppress_prediction_once(controlled);
            }
        }
        self.entities
            .tick_update(self.game_states.last_processed_tick);
        let _ = self
            .input
            .apply_held_movement_state(&mut self.entities, &self.players);
        self.physics.update(&mut self.entities, 0.016);
        self.transforms.frame_update(&mut self.entities, 0.5);
    }

    pub fn flush_to_server(&mut self, server: &mut DaikokuServer) {
        let local_user = self
            .players
            .local_player()
            .map(|player| player.user_id.clone())
            .unwrap_or_default();
        for ack in self.network.take_acks() {
            server.ack_state(&local_user, ack);
        }
        for input in self.network.take_inputs() {
            let _ = server.receive_input(&local_user, input);
        }
        for message in self.network.take_entities() {
            let _ = server.receive_entity_message(&local_user, message);
        }
        let requests = self.network.take_player_list_requests();
        for _ in 0..requests {
            let _ = server.receive_player_list_request(&local_user);
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
        for message in server.network.take_outbox(user_id) {
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
        assert_eq!(client.network.take_player_list_requests(), 1);
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
        assert_eq!(client.run_level, ClientRunLevel::InGame);
        assert_eq!(client.network.take_acks().len(), 3);
        client.shutdown();
        assert_eq!(client.run_level, ClientRunLevel::Initialize);
    }

    #[test]
    fn client_can_consume_real_server_state_roundtrip() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        let map_id = server.maps.create_map(&mut server.entities.inner, None);
        let grid_id = server
            .maps
            .create_grid(&mut server.entities.inner, map_id, None, 8);
        server.maps.set_current_tick(GameTick::ZERO);
        assert!(server.maps.set_tile(
            &mut server.entities.inner,
            grid_id,
            Vector2i::new(0, 0),
            sekai::Tile::new(7, sekai::TileRenderFlag(0), 0),
        ));
        let uid = server.entities.create_entity(Some("mob"));
        server.entities.initialize_entity(uid);
        {
            let transform = server.entities.inner.transforms.get_mut(&uid).unwrap();
            transform.map_id = map_id;
            transform.rebuild_for_manager();
        }
        {
            let physics = server.entities.inner.ensure_physics(uid);
            physics.can_collide = true;
            physics.set_body_type(sekai::BodyType::Dynamic);
            physics.awake = true;
        }
        server
            .entities
            .inner
            .ensure_fixtures(uid)
            .insert_fixture(Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ));
        let mut joint = Joint::new(uid.raw(), 999, JointType::Distance);
        joint.id = "rope".to_string();
        server.entities.inner.ensure_joints(uid).add_joint(joint);
        server.attach_player("u1", uid, false);
        server.players.set_current_tick(GameTick::FIRST);
        assert!(server.players.join_game("u1"));
        let mut server_messages = Vec::new();
        for _ in 0..3 {
            server.tick_update(0.016);
            server_messages.extend(server.network.take_outbox("u1"));
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

        assert_eq!(client.run_level, ClientRunLevel::InGame);
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
        let client_grid_uid = client
            .entities
            .inner
            .map_grid_components
            .iter()
            .find(|(_, component)| component.grid_index == grid_id)
            .map(|(uid, _)| *uid)
            .unwrap();
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
        let map_owner = client.entities.map_entity_for(map_id).unwrap();
        assert_eq!(
            client.physics.query_aabb_entities(
                &client.entities,
                map_owner,
                Box2::new(-2.0, -2.0, 2.0, 2.0)
            ),
            vec![uid]
        );
        assert_eq!(
            client.physics.intersect_ray(
                &client.entities,
                map_owner,
                CollisionRay::new(Vector2::new(-5.0, 0.0), Vector2::UNIT_X, -1),
                10.0,
                true,
            )[0]
            .entity,
            uid
        );
    }

    #[test]
    fn client_roundtrips_incremental_chunk_updates_and_deletions_from_server() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        let map_id = server
            .maps
            .create_map(&mut server.entities.inner, Some(sekai::MapId::new(2)));
        let grid_id = server.maps.create_grid(
            &mut server.entities.inner,
            map_id,
            Some(sekai::GridId::new(9)),
            4,
        );
        let controlled = server.entities.create_entity(Some("mob"));
        server.entities.initialize_entity(controlled);
        {
            let transform = server
                .entities
                .inner
                .transforms
                .get_mut(&controlled)
                .unwrap();
            transform.map_id = map_id;
            transform.rebuild_for_manager();
        }
        server.attach_player("u1", controlled, false);
        server.players.set_current_tick(GameTick::FIRST);
        assert!(server.players.join_game("u1"));

        server.maps.set_current_tick(GameTick::new(1));
        assert!(server.maps.set_tile(
            &mut server.entities.inner,
            grid_id,
            Vector2i::new(0, 0),
            sekai::Tile::new(1, sekai::TileRenderFlag(0), 0),
        ));
        server.tick_update(0.016);
        let initial = server.network.take_outbox("u1");

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

        server.maps.set_current_tick(GameTick::new(2));
        server.players.set_current_tick(GameTick::new(2));
        assert!(server.maps.set_tile(
            &mut server.entities.inner,
            grid_id,
            Vector2i::new(4, 0),
            sekai::Tile::new(9, sekai::TileRenderFlag(0), 0),
        ));
        server.tick_update(0.016);
        let changed = server.network.take_outbox("u1");
        for message in changed {
            if let daikoku::OutboundMessage::State(state) = message {
                client.network.receive_state(state);
            }
        }
        client.tick_update();
        client.flush_to_server(&mut server);

        let client_grid_uid = client
            .entities
            .inner
            .map_grid_components
            .iter()
            .find(|(_, component)| component.grid_index == grid_id)
            .map(|(uid, _)| *uid)
            .unwrap();
        let grid = client
            .entities
            .inner
            .map_grids
            .get(&client_grid_uid)
            .unwrap();
        assert_eq!(grid.get_tile_ref(Vector2i::new(4, 0)).tile.type_id, 9);
        assert_eq!(grid.get_tile_ref(Vector2i::new(0, 0)).tile.type_id, 1);

        server.maps.set_current_tick(GameTick::new(3));
        server.players.set_current_tick(GameTick::new(3));
        assert!(
            server
                .maps
                .remove_chunk(&mut server.entities.inner, grid_id, Vector2i::new(0, 0))
        );
        server.tick_update(0.016);
        let deleted = server.network.take_outbox("u1");
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
        let map_id = server
            .maps
            .create_map(&mut server.entities.inner, Some(MapId::new(2)));
        let grid_id =
            server
                .maps
                .create_grid(&mut server.entities.inner, map_id, Some(GridId::new(9)), 4);
        let grid_uid = server.maps.get_grid_euid(grid_id).unwrap();
        server.maps.set_current_tick(GameTick::new(1));
        assert!(server.maps.set_tile(
            &mut server.entities.inner,
            grid_id,
            Vector2i::new(0, 0),
            sekai::Tile::new(5, sekai::TileRenderFlag(0), 0),
        ));

        let controlled = server.entities.create_entity(Some("mob"));
        server.entities.initialize_entity(controlled);
        {
            let transform = server
                .entities
                .inner
                .transforms
                .get_mut(&controlled)
                .unwrap();
            transform.map_id = map_id;
            transform.rebuild_for_manager();
        }
        server.attach_player("u1", controlled, false);
        server.players.set_current_tick(GameTick::FIRST);
        assert!(server.players.join_game("u1"));

        let child = server.entities.create_entity(Some("item"));
        server.entities.initialize_entity(child);
        let _ = server.entities.inner.apply_transform_state(
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
        sekai::EntityLookupSystem.update_bounds(&mut server.entities.inner, child);

        server.tick_update(0.016);

        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        feed_server_states_to_client(&mut client, &mut server, "u1");
        client.tick_update();
        client.flush_to_server(&mut server);

        let client_grid_uid = client
            .entities
            .inner
            .map_grid_components
            .iter()
            .find(|(_, component)| component.grid_index == grid_id)
            .map(|(uid, _)| *uid)
            .unwrap();
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

        server.current_tick = GameTick::new(2);
        server.entities.set_current_tick(GameTick::new(2));
        server.players.set_current_tick(GameTick::new(2));
        server.maps.set_current_tick(GameTick::new(2));
        assert!(
            server
                .entities
                .remove_component_by_net_id(grid_uid, MAP_GRID_NET_ID)
        );
        assert!(
            server
                .entities
                .remove_component_by_net_id(child, TRANSFORM_NET_ID)
        );

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
        let map_id = server
            .maps
            .create_map(&mut server.entities.inner, Some(MapId::new(4)));
        let map_uid = server.maps.get_map_entity_id(map_id);
        let grid_id =
            server
                .maps
                .create_grid(&mut server.entities.inner, map_id, Some(GridId::new(12)), 4);

        server.maps.set_current_tick(GameTick::new(1));
        assert!(server.maps.set_tile(
            &mut server.entities.inner,
            grid_id,
            Vector2i::new(0, 0),
            sekai::Tile::new(3, sekai::TileRenderFlag(0), 0),
        ));

        let controlled = server.entities.create_entity(Some("mob"));
        server.entities.initialize_entity(controlled);
        {
            let transform = server
                .entities
                .inner
                .transforms
                .get_mut(&controlled)
                .unwrap();
            transform.map_id = map_id;
            transform.rebuild_for_manager();
        }
        server.attach_player("u1", controlled, false);
        server.players.set_current_tick(GameTick::FIRST);
        assert!(server.players.join_game("u1"));
        server.tick_update(0.016);

        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        feed_server_states_to_client(&mut client, &mut server, "u1");
        client.tick_update();
        client.flush_to_server(&mut server);

        let client_map_uid = client.entities.map_entity_for(map_id).unwrap();
        let client_grid_uid = client
            .entities
            .inner
            .map_grid_components
            .iter()
            .find(|(_, component)| component.grid_index == grid_id)
            .map(|(uid, _)| *uid)
            .unwrap();
        assert!(
            client
                .entities
                .inner
                .broadphases
                .contains_key(&client_map_uid)
        );
        assert!(
            client
                .entities
                .inner
                .physics_maps
                .contains_key(&client_map_uid)
        );
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

        server.current_tick = GameTick::new(2);
        server.entities.set_current_tick(GameTick::new(2));
        server.players.set_current_tick(GameTick::new(2));
        server.maps.set_current_tick(GameTick::new(2));
        assert!(
            server
                .entities
                .remove_component_by_net_id(map_uid, MAP_NET_ID)
        );

        server.tick_update(0.016);
        feed_server_states_to_client(&mut client, &mut server, "u1");
        client.tick_update();

        assert!(client.entities.map_entity_for(map_id).is_none());
        assert!(
            !client
                .entities
                .inner
                .map_components
                .contains_key(&client_map_uid)
        );
        assert!(
            !client
                .entities
                .inner
                .broadphases
                .contains_key(&client_map_uid)
        );
        assert!(
            !client
                .entities
                .inner
                .physics_maps
                .contains_key(&client_map_uid)
        );
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
    fn client_flushes_inputs_and_acks_back_to_server() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        server.players.set_current_tick(GameTick::FIRST);
        assert!(server.players.join_game("u1"));

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
        for message in server.network.take_outbox("u1") {
            if let daikoku::OutboundMessage::State(state) = message {
                client.network.receive_state(state);
            }
        }
        client.tick_update();
        client.flush_to_server(&mut server);
        server.tick_update(0.016);
        assert_eq!(
            server
                .players
                .get_session("u1")
                .unwrap()
                .last_processed_input,
            1
        );
    }

    #[test]
    fn client_replays_pending_inputs_after_authoritative_state() {
        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        let uid = client.entities.create_entity(None, EntityUid::new(40));
        client.entities.initialize_entity(uid);
        client
            .players
            .local_player_mut()
            .unwrap()
            .attach_entity(uid);

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
        client.entities.initialize_entity(uid);
        client
            .players
            .local_player_mut()
            .unwrap()
            .attach_entity(uid);

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
            Vector2::new(crate::InputSystem::MOVE_SPEED, 0.0)
        );
        assert_eq!(client.network.take_inputs().len(), 1);
    }

    #[test]
    fn base_client_continues_predicting_held_input_across_frames() {
        let mut client = BaseClient::new(ClientOptions {
            username: "pedel".to_string(),
        });
        client.startup("u1");
        let uid = client.entities.create_entity(None, EntityUid::new(42));
        client.entities.initialize_entity(uid);
        client
            .players
            .local_player_mut()
            .unwrap()
            .attach_entity(uid);

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
            Vector2::new(2.0, 0.0)
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
            server.entities.received_system_messages,
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
        for message in server.network.take_outbox("u1") {
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
        client.entities.initialize_entity(uid);
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
