use jikan::GameTick;
use sekai::{EntityUid, PlayerState, SessionStatus};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlayerSession {
    pub(crate) user_id: String,
    pub(crate) username: String,
    pub(crate) status: SessionStatus,
    pub(crate) ping: i16,
    pub(crate) controlled_entity: Option<EntityUid>,
    pub(crate) last_processed_input: u32,
    pub(crate) last_modified_tick: GameTick,
}

impl PlayerSession {
    fn new(user_id: impl Into<String>, username: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            username: username.into(),
            status: SessionStatus::Connecting,
            ping: 0,
            controlled_entity: None,
            last_processed_input: 0,
            last_modified_tick: GameTick::ZERO,
        }
    }

    fn join_game(&mut self, current_tick: GameTick) {
        self.status = SessionStatus::InGame;
        self.last_modified_tick = current_tick;
    }

    fn on_connect(&mut self, current_tick: GameTick) {
        self.status = SessionStatus::Connected;
        self.last_modified_tick = current_tick;
    }

    fn on_disconnect(&mut self, current_tick: GameTick) {
        self.status = SessionStatus::Disconnected;
        self.controlled_entity = None;
        self.last_modified_tick = current_tick;
    }

    fn set_attached_entity(&mut self, entity: Option<EntityUid>, current_tick: GameTick) {
        self.controlled_entity = entity;
        self.last_modified_tick = current_tick;
    }

    fn to_player_state(&self) -> PlayerState {
        PlayerState {
            user_id: self.user_id.clone(),
            name: self.username.clone(),
            status: self.status,
            ping: self.ping,
            controlled_entity: self.controlled_entity,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DisconnectedPlayerState {
    state: PlayerState,
    last_modified_tick: GameTick,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct PlayerManager {
    sessions: HashMap<String, PlayerSession>,
    disconnected_states: HashMap<String, DisconnectedPlayerState>,
    max_players: usize,
    current_tick: GameTick,
}

impl PlayerManager {
    pub(crate) fn new(max_players: usize) -> Self {
        Self {
            sessions: HashMap::new(),
            disconnected_states: HashMap::new(),
            max_players,
            current_tick: GameTick::ZERO,
        }
    }

    pub(crate) fn initialize(&mut self, max_players: usize) {
        self.max_players = max_players;
        self.current_tick = GameTick::ZERO;
    }

    pub(crate) fn shutdown(&mut self) {
        self.sessions.clear();
        self.disconnected_states.clear();
    }

    pub(crate) fn set_current_tick(&mut self, current_tick: GameTick) {
        self.current_tick = current_tick;
    }

    pub(crate) fn connect(
        &mut self,
        user_id: impl Into<String>,
        username: impl Into<String>,
    ) -> bool {
        if self.sessions.len() >= self.max_players {
            return false;
        }
        let user_id = user_id.into();
        let username = username.into();
        self.disconnected_states.remove(&user_id);
        self.sessions.insert(
            user_id.clone(),
            PlayerSession::new(user_id.clone(), username),
        );
        if let Some(session) = self.sessions.get_mut(&user_id) {
            session.on_connect(self.current_tick);
        }
        true
    }

    pub(crate) fn disconnect(&mut self, user_id: &str) {
        if let Some(mut session) = self.sessions.remove(user_id) {
            session.on_disconnect(self.current_tick);
            self.disconnected_states.insert(
                user_id.to_string(),
                DisconnectedPlayerState {
                    state: session.to_player_state(),
                    last_modified_tick: self.current_tick,
                },
            );
        }
    }

    pub(crate) fn get_session(&self, user_id: &str) -> Option<&PlayerSession> {
        self.sessions.get(user_id)
    }

    pub(crate) fn join_game(&mut self, user_id: &str) -> bool {
        let Some(session) = self.sessions.get_mut(user_id) else {
            return false;
        };
        session.join_game(self.current_tick);
        true
    }

    pub(crate) fn set_attached_entity(&mut self, user_id: &str, entity: Option<EntityUid>) -> bool {
        let Some(session) = self.sessions.get_mut(user_id) else {
            return false;
        };
        session.set_attached_entity(entity, self.current_tick);
        true
    }

    pub(crate) fn set_last_processed_input(&mut self, user_id: &str, input_sequence: u32) -> bool {
        let Some(session) = self.sessions.get_mut(user_id) else {
            return false;
        };
        session.last_processed_input = input_sequence;
        session.last_modified_tick = self.current_tick;
        true
    }

    #[cfg(test)]
    pub(crate) fn set_ping(&mut self, user_id: &str, ping: i16) -> bool {
        let Some(session) = self.sessions.get_mut(user_id) else {
            return false;
        };
        session.ping = ping;
        session.last_modified_tick = self.current_tick;
        true
    }

    pub(crate) fn sessions(&self) -> impl Iterator<Item = &PlayerSession> {
        self.sessions.values()
    }

    pub(crate) fn in_game_sessions(&self) -> impl Iterator<Item = &PlayerSession> {
        self.sessions
            .values()
            .filter(|session| session.status == SessionStatus::InGame)
    }

    pub(crate) fn get_player_states(&self) -> Vec<PlayerState> {
        self.sessions
            .values()
            .map(PlayerSession::to_player_state)
            .collect()
    }

    pub(crate) fn get_player_states_since(&self, from_tick: GameTick) -> Vec<PlayerState> {
        let mut states = Vec::new();

        for session in self.sessions.values() {
            if from_tick == GameTick::ZERO || session.last_modified_tick > from_tick {
                states.push(session.to_player_state());
            }
        }

        if from_tick != GameTick::ZERO {
            for disconnected in self.disconnected_states.values() {
                if disconnected.last_modified_tick > from_tick {
                    states.push(disconnected.state.clone());
                }
            }
        }

        states
    }

    pub(crate) fn cull_disconnected_history(&mut self, before_or_at: GameTick) {
        self.disconnected_states
            .retain(|_, state| state.last_modified_tick > before_or_at);
    }
}

#[cfg(test)]
mod tests {
    use super::PlayerManager;
    use jikan::GameTick;
    use sekai::{EntityUid, SessionStatus};

    #[test]
    fn player_manager_tracks_connections_and_states() {
        let mut manager = PlayerManager::new(2);
        assert!(manager.connect("u1", "pedel"));
        assert!(manager.join_game("u1"));
        assert!(manager.set_attached_entity("u1", Some(EntityUid::new(7))));
        assert_eq!(manager.sessions().count(), 1);
        assert_eq!(manager.get_player_states()[0].status, SessionStatus::InGame);
        assert_eq!(
            manager.get_session("u1").unwrap().status,
            SessionStatus::InGame
        );
    }

    #[test]
    fn player_manager_tracks_incremental_changes_and_disconnects() {
        let mut manager = PlayerManager::new(2);
        manager.set_current_tick(GameTick::new(1));
        assert!(manager.connect("u1", "pedel"));
        assert_eq!(manager.get_player_states_since(GameTick::ZERO).len(), 1);

        manager.set_current_tick(GameTick::new(2));
        assert!(manager.join_game("u1"));
        let changes = manager.get_player_states_since(GameTick::new(1));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].status, SessionStatus::InGame);

        manager.set_current_tick(GameTick::new(3));
        manager.disconnect("u1");
        let changes = manager.get_player_states_since(GameTick::new(2));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].status, SessionStatus::Disconnected);

        manager.cull_disconnected_history(GameTick::new(3));
        assert!(manager.get_player_states_since(GameTick::new(2)).is_empty());
    }
}
