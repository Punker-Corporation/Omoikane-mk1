use sekai::{EntityUid, PlayerState, SessionStatus};
use std::collections::{HashMap, HashSet};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerData {
    pub user_id: String,
    pub username: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSession {
    pub user_id: String,
    pub username: String,
    pub status: SessionStatus,
    pub ping: i16,
    pub controlled_entity: Option<EntityUid>,
    pub last_processed_input: u32,
    pub connected_time: Option<SystemTime>,
    pub view_subscriptions: HashSet<EntityUid>,
    pub visibility_mask: i32,
}

impl PlayerSession {
    pub fn new(user_id: impl Into<String>, username: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            username: username.into(),
            status: SessionStatus::Connecting,
            ping: 0,
            controlled_entity: None,
            last_processed_input: 0,
            connected_time: None,
            view_subscriptions: HashSet::new(),
            visibility_mask: 1,
        }
    }

    pub fn join_game(&mut self) {
        self.status = SessionStatus::InGame;
    }

    pub fn on_connect(&mut self) {
        self.connected_time = Some(SystemTime::now());
        self.status = SessionStatus::Connected;
    }

    pub fn on_disconnect(&mut self) {
        self.status = SessionStatus::Disconnected;
        self.controlled_entity = None;
        self.view_subscriptions.clear();
    }

    pub fn detach_from_entity(&mut self) {
        self.controlled_entity = None;
    }

    pub fn set_attached_entity(&mut self, entity: Option<EntityUid>) {
        self.controlled_entity = entity;
    }

    pub fn add_view_subscription(&mut self, entity: EntityUid) {
        self.view_subscriptions.insert(entity);
    }

    pub fn remove_view_subscription(&mut self, entity: EntityUid) {
        self.view_subscriptions.remove(&entity);
    }

    pub fn view_subscription_count(&self) -> usize {
        self.view_subscriptions.len()
    }

    pub fn to_player_state(&self) -> PlayerState {
        PlayerState {
            user_id: self.user_id.clone(),
            name: self.username.clone(),
            status: self.status,
            ping: self.ping,
            controlled_entity: self.controlled_entity,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct PlayerManager {
    sessions: HashMap<String, PlayerSession>,
    player_data: HashMap<String, PlayerData>,
    max_players: usize,
}

impl PlayerManager {
    pub fn new(max_players: usize) -> Self {
        Self {
            sessions: HashMap::new(),
            player_data: HashMap::new(),
            max_players,
        }
    }

    pub fn initialize(&mut self, max_players: usize) {
        self.max_players = max_players;
    }

    pub fn shutdown(&mut self) {
        self.sessions.clear();
    }

    pub fn max_players(&self) -> usize {
        self.max_players
    }

    pub fn player_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn connect(&mut self, user_id: impl Into<String>, username: impl Into<String>) -> bool {
        if self.sessions.len() >= self.max_players {
            return false;
        }
        let user_id = user_id.into();
        let username = username.into();
        self.player_data.insert(
            user_id.clone(),
            PlayerData {
                user_id: user_id.clone(),
                username: username.clone(),
            },
        );
        self.sessions
            .insert(user_id.clone(), PlayerSession::new(user_id.clone(), username));
        if let Some(session) = self.sessions.get_mut(&user_id) {
            session.on_connect();
        }
        true
    }

    pub fn disconnect(&mut self, user_id: &str) {
        if let Some(session) = self.sessions.get_mut(user_id) {
            session.on_disconnect();
        }
        self.sessions.remove(user_id);
    }

    pub fn get_session(&self, user_id: &str) -> Option<&PlayerSession> {
        self.sessions.get(user_id)
    }

    pub fn get_session_mut(&mut self, user_id: &str) -> Option<&mut PlayerSession> {
        self.sessions.get_mut(user_id)
    }

    pub fn try_get_session_by_username(&self, username: &str) -> Option<&PlayerSession> {
        self.sessions.values().find(|session| session.username == username)
    }

    pub fn sessions(&self) -> impl Iterator<Item = &PlayerSession> {
        self.sessions.values()
    }

    pub fn in_game_sessions(&self) -> impl Iterator<Item = &PlayerSession> {
        self.sessions.values().filter(|session| session.status == SessionStatus::InGame)
    }

    pub fn get_player_states(&self) -> Vec<PlayerState> {
        self.sessions.values().map(PlayerSession::to_player_state).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::PlayerManager;
    use sekai::{EntityUid, SessionStatus};

    #[test]
    fn player_manager_tracks_connections_and_states() {
        let mut manager = PlayerManager::new(2);
        assert!(manager.connect("u1", "pedel"));
        let session = manager.get_session_mut("u1").unwrap();
        session.join_game();
        session.controlled_entity = Some(EntityUid::new(7));
        assert_eq!(manager.player_count(), 1);
        assert_eq!(manager.get_player_states()[0].status, SessionStatus::InGame);
        assert!(manager.get_session("u1").unwrap().connected_time.is_some());
    }
}
