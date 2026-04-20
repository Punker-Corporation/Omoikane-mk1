use crate::{
    ActorSystem, EntityMessageType, FullInputCmdMessage, InputSystem, JointSystem, MsgEntity,
    MsgStateAck, PlayerManager, ServerEntityManager, ServerGameStateManager, ServerNetManager,
};
use jikan::GameTick;
use sekai::{MapManager, MsgPlayerList, TimerSystem};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerOptions {
    pub server_name: String,
    pub max_players: usize,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            server_name: "Omoikane".to_string(),
            max_players: 32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerState {
    Stopped,
    Running,
    ShuttingDown,
}

pub struct DaikokuServer {
    pub options: ServerOptions,
    pub state: ServerState,
    pub entities: ServerEntityManager,
    pub players: PlayerManager,
    pub game_states: ServerGameStateManager,
    pub joints: JointSystem,
    pub network: ServerNetManager,
    pub actors: ActorSystem,
    pub input: InputSystem,
    pub maps: MapManager,
    pub current_tick: GameTick,
    shutdown_reason: Option<String>,
}

impl DaikokuServer {
    pub fn new(options: ServerOptions) -> Self {
        let max_players = options.max_players;
        Self {
            options,
            state: ServerState::Stopped,
            entities: ServerEntityManager::new(),
            players: PlayerManager::new(max_players),
            game_states: ServerGameStateManager::new(),
            joints: JointSystem::new(),
            network: ServerNetManager::new(),
            actors: ActorSystem::new(),
            input: InputSystem::new(),
            maps: MapManager::new(),
            current_tick: GameTick::ZERO,
            shutdown_reason: None,
        }
    }

    pub fn start(&mut self) {
        self.players.initialize(self.options.max_players);
        self.entities.initialize();
        self.game_states.initialize();
        self.network.initialize();
        self.input.initialize();
        self.maps.initialize();
        self.maps.startup(&mut self.entities.inner);
        self.state = ServerState::Running;
    }

    pub fn shutdown(&mut self, reason: Option<String>) {
        self.shutdown_reason = reason;
        self.state = ServerState::ShuttingDown;
        self.players.shutdown();
        self.maps.shutdown(&mut self.entities.inner);
        self.state = ServerState::Stopped;
    }

    pub fn restart(&mut self) {
        self.shutdown(Some("restart".to_string()));
        self.start();
    }

    pub fn connect_player(&mut self, user_id: impl Into<String>, username: impl Into<String>) -> bool {
        let user_id = user_id.into();
        let username = username.into();
        let connected = self.players.connect(user_id.clone(), username);
        if connected {
            let _ = self.network.connect(user_id.clone());
            self.input.handle_player_connected(user_id.clone());
            self.game_states.handle_client_connected(user_id);
        }
        connected
    }

    pub fn disconnect_player(&mut self, user_id: &str) {
        let _ = self.actors.detach_player(&mut self.entities, &mut self.players, user_id);
        self.players.disconnect(user_id);
        self.network.disconnect(user_id);
        self.input.handle_player_disconnected(user_id);
        self.game_states.handle_client_disconnect(user_id);
    }

    pub fn attach_player(&mut self, user_id: &str, entity: sekai::EntityUid, force: bool) -> bool {
        self.actors
            .attach(&mut self.entities, &mut self.players, entity, user_id, force)
            .result
    }

    pub fn queue_player_input(&mut self, user_id: &str, message: FullInputCmdMessage) -> bool {
        self.network.queue_input(user_id, message)
    }

    pub fn receive_input(&mut self, user_id: &str, message: FullInputCmdMessage) -> bool {
        self.queue_player_input(user_id, message)
    }

    pub fn ack_state(&mut self, user_id: &str, ack: MsgStateAck) {
        self.game_states.ack(user_id, ack.sequence);
    }

    pub fn receive_entity_message(&mut self, user_id: &str, message: MsgEntity) -> bool {
        self.network.queue_entity(user_id, message)
    }

    pub fn receive_player_list_request(&mut self, user_id: &str) -> bool {
        self.network.queue_player_list_request(user_id)
    }

    pub fn send_entity_message(&mut self, user_id: &str, message: MsgEntity) -> bool {
        self.network.send_entity(user_id, message)
    }

    pub fn tick_update(&mut self, frame_time: f32) {
        if self.state != ServerState::Running {
            return;
        }

        self.current_tick = self.current_tick + 1;
        let users: Vec<_> = self.players.sessions().map(|session| session.user_id.clone()).collect();
        for user_id in users {
            let requests = self.network.take_player_list_requests(&user_id);
            for _ in 0..requests {
                let _ = self.network.send_player_list(
                    &user_id,
                    MsgPlayerList {
                        plyrs: self.players.get_player_states(),
                    },
                );
            }
            for input in self.network.take_input(&user_id) {
                let _ = self.input.handle_input(&mut self.players, &user_id, input);
            }
            for message in self.network.take_entities(&user_id) {
                match message.message_type {
                    EntityMessageType::ComponentMessage => {
                        self.entities.receive_component_message(
                            user_id.clone(),
                            message.entity_uid,
                            message.net_id,
                            message.component_message.unwrap_or_default(),
                        );
                    }
                    EntityMessageType::SystemMessage => {
                        self.entities
                            .receive_system_message(user_id.clone(), message.system_message.unwrap_or_default());
                    }
                    EntityMessageType::Error => {}
                }
            }
        }
        TimerSystem.update(&mut self.entities.inner, frame_time);
        let updates = self
            .game_states
            .send_game_state_update(&mut self.entities, &self.players, self.current_tick);
        for (user_id, state) in updates {
            let _ = self.network.send_state(&user_id, state);
        }
    }

    pub fn shutdown_reason(&self) -> Option<&str> {
        self.shutdown_reason.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::{DaikokuServer, ServerOptions, ServerState};
    use crate::{BoundKeyState, EntityMessageType, FullInputCmdMessage, MsgEntity};
    use jikan::GameTick;
    use keisan::Vector2;
    use sekai::SessionStatus;
    use sekai::{EntityCoordinates, EntityUid, ScreenCoordinates, WindowId};

    #[test]
    fn base_server_starts_ticks_and_shuts_down() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert_eq!(server.state, ServerState::Running);
        assert!(server.connect_player("u1", "pedel"));
        server.players.get_session_mut("u1").unwrap().status = SessionStatus::InGame;
        server.tick_update(0.016);
        assert_eq!(server.current_tick.value, 1);
        assert_eq!(server.network.take_outbox("u1").len(), 1);
        server.shutdown(Some("done".to_string()));
        assert_eq!(server.state, ServerState::Stopped);
    }

    #[test]
    fn base_server_processes_queued_input_and_actor_attachment() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        let uid = server.entities.create_entity(Some("mob"));
        server.entities.initialize_entity(uid);
        assert!(server.attach_player("u1", uid, false));
        server.players.get_session_mut("u1").unwrap().join_game();

        assert!(server.queue_player_input(
            "u1",
            FullInputCmdMessage::new(
                jikan::GameTick::new(1),
                0,
                6,
                "MoveRight",
                BoundKeyState::Down,
                EntityCoordinates::new(EntityUid::new(5), Vector2::ZERO),
                ScreenCoordinates::new_xy(5.0, 8.0, WindowId::MAIN),
            ),
        ));

        server.tick_update(0.016);
        assert_eq!(server.players.get_session("u1").unwrap().controlled_entity, Some(uid));
        assert_eq!(server.players.get_session("u1").unwrap().last_processed_input, 6);
    }

    #[test]
    fn base_server_processes_entity_messages_from_clients() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        assert!(server.receive_entity_message(
            "u1",
            MsgEntity {
                message_type: EntityMessageType::SystemMessage,
                system_message: Some("jump".to_string()),
                component_message: None,
                entity_uid: EntityUid::new(4),
                net_id: 0,
                sequence: 1,
                source_tick: GameTick::FIRST,
            }
        ));
        assert!(server.receive_entity_message(
            "u1",
            MsgEntity {
                message_type: EntityMessageType::ComponentMessage,
                system_message: None,
                component_message: Some("poke".to_string()),
                entity_uid: EntityUid::new(4),
                net_id: 8,
                sequence: 2,
                source_tick: GameTick::FIRST,
            }
        ));
        server.tick_update(0.016);
        assert_eq!(server.entities.received_system_messages, vec![("u1".to_string(), "jump".to_string())]);
        assert_eq!(server.entities.received_component_messages.len(), 1);
    }

    #[test]
    fn base_server_responds_to_player_list_requests() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        assert!(server.receive_player_list_request("u1"));
        server.tick_update(0.016);
        let outbox = server.network.take_outbox("u1");
        assert!(outbox.iter().any(|message| matches!(message, crate::OutboundMessage::PlayerList(_))));
    }
}
