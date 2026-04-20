use crate::{
    ClientEntityManager, ClientGameStateManager, ClientNetManager, InputSystem, MapSystem,
    PhysicsSystem, PlayerManager, TransformSystem,
};
use daikoku::{DaikokuServer, FullInputCmdMessage};

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
        self.game_states
            .input_command_dispatched(&mut self.network, input)
    }

    pub fn tick_update(&mut self) {
        self.game_states.pump_network(&mut self.network);
        while let Some(player_list) = self.network.next_player_list() {
            self.players.apply_player_states(&player_list.plyrs);
        }
        while let Some(message) = self.network.next_entity() {
            self.entities
                .handle_entity_network_message(self.game_states.last_processed_tick, message);
        }
        if let Some(state) = self
            .game_states
            .apply_next_state(&mut self.entities, &mut self.players)
        {
            if let Some(map_data) = &state.map_data {
                self.maps.apply_game_state(&mut self.entities, map_data);
            }
            self.run_level = ClientRunLevel::InGame;
        }
        self.entities.tick_update(self.game_states.last_processed_tick);
        self.physics.update(&mut self.entities, 0.016);
        self.transforms.frame_update(&mut self.entities, 1.0);
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
    use daikoku::{
        BoundKeyState, DaikokuServer, EntityMessageType, FullInputCmdMessage, MsgEntity, MsgState,
        ServerOptions,
    };
    use jikan::GameTick;
    use keisan::{Vector2, Vector2i};
    use sekai::GameState;
    use sekai::{EntityCoordinates, EntityUid, ScreenCoordinates, WindowId};

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
        let grid_uid = server.maps.get_grid_euid(grid_id).unwrap();
        server
            .entities
            .inner
            .map_grids
            .get_mut(&grid_uid)
            .unwrap()
            .set_tile(Vector2i::new(0, 0), sekai::Tile::new(7, sekai::TileRenderFlag(0), 0));
        let uid = server.entities.create_entity(Some("mob"));
        server.entities.initialize_entity(uid);
        server.attach_player("u1", uid, false);
        server.players.get_session_mut("u1").unwrap().join_game();
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
        assert_eq!(client.players.local_player().unwrap().controlled_entity, Some(uid));
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
    }

    #[test]
    fn client_flushes_inputs_and_acks_back_to_server() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        server.players.get_session_mut("u1").unwrap().join_game();

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
        assert_eq!(server.players.get_session("u1").unwrap().last_processed_input, 1);
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
        assert_eq!(server.entities.received_system_messages, vec![("u1".to_string(), "wave".to_string())]);

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
}
