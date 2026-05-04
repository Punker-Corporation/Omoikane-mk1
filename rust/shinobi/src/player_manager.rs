use sekai::{EntityUid, PlayerState, SessionStatus};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClientSession {
    pub(crate) user_id: String,
    pub(crate) name: String,
    pub(crate) status: SessionStatus,
    pub(crate) ping: i16,
    pub(crate) attached_entity: Option<EntityUid>,
}

impl ClientSession {
    fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            name: String::new(),
            status: SessionStatus::Connecting,
            ping: 0,
            attached_entity: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalPlayer {
    pub(crate) controlled_entity: Option<EntityUid>,
    pub(crate) user_id: String,
    pub(crate) name: String,
    pub(crate) session: ClientSession,
}

impl LocalPlayer {
    fn new(user_id: impl Into<String>, name: impl Into<String>) -> Self {
        let user_id = user_id.into();
        let name = name.into();
        Self {
            controlled_entity: None,
            user_id: user_id.clone(),
            name: name.clone(),
            session: ClientSession {
                user_id,
                name,
                status: SessionStatus::Connecting,
                ping: 0,
                attached_entity: None,
            },
        }
    }

    fn attach_entity(&mut self, entity: EntityUid) {
        self.detach_entity();
        self.controlled_entity = Some(entity);
        self.session.attached_entity = Some(entity);
    }

    fn detach_entity(&mut self) {
        if self.controlled_entity.take().is_some() {
            self.session.attached_entity = None;
        }
    }

    fn switch_state(&mut self, new_status: SessionStatus) {
        let old = self.session.status;
        if old != new_status {
            self.session.status = new_status;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct PlayerManager {
    sessions: HashMap<String, ClientSession>,
    local_player: Option<LocalPlayer>,
}

impl PlayerManager {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn startup(&mut self, user_id: impl Into<String>, name: impl Into<String>) {
        let user_id = user_id.into();
        let name = name.into();
        self.local_player = Some(LocalPlayer::new(user_id, name));
    }

    pub(crate) fn shutdown(&mut self) {
        self.local_player = None;
        self.sessions.clear();
    }

    pub(crate) fn local_player(&self) -> Option<&LocalPlayer> {
        self.local_player.as_ref()
    }

    pub(crate) fn controlled_entity(&self) -> Option<EntityUid> {
        self.local_player
            .as_ref()
            .and_then(|player| player.controlled_entity)
    }

    #[cfg(test)]
    pub(crate) fn attach_local_entity(&mut self, entity: EntityUid) -> bool {
        let Some(player) = self.local_player.as_mut() else {
            return false;
        };
        player.attach_entity(entity);
        true
    }

    #[cfg(test)]
    pub(crate) fn session(&self, user_id: &str) -> Option<&ClientSession> {
        self.sessions.get(user_id)
    }

    pub(crate) fn apply_player_states(&mut self, states: &[PlayerState], full_snapshot: bool) {
        if states.is_empty() {
            return;
        }

        let local_id = self
            .local_player
            .as_ref()
            .map(|player| player.user_id.clone());
        let mut seen = Vec::new();

        for state in states {
            seen.push(state.user_id.clone());
            if state.status == SessionStatus::Disconnected {
                self.sessions.remove(&state.user_id);
                if Some(&state.user_id) == local_id.as_ref() {
                    let player = self.local_player.as_mut().expect("local player exists");
                    player.detach_entity();
                    player.switch_state(SessionStatus::Disconnected);
                }
                continue;
            }
            let entry = self
                .sessions
                .entry(state.user_id.clone())
                .or_insert_with(|| ClientSession::new(state.user_id.clone()));
            entry.name = state.name.clone();
            entry.status = state.status;
            entry.ping = state.ping;
            entry.attached_entity = state.controlled_entity;

            if Some(&state.user_id) == local_id.as_ref() {
                let player = self.local_player.as_mut().expect("local player exists");
                player.name = state.name.clone();
                player.session.name = state.name.clone();
                player.session.ping = state.ping;
                if player.controlled_entity != state.controlled_entity {
                    if let Some(entity) = state.controlled_entity {
                        player.attach_entity(entity);
                    } else {
                        player.detach_entity();
                    }
                }
                player.switch_state(state.status);
            }
        }

        if full_snapshot {
            self.sessions.retain(|user_id, _| seen.contains(user_id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PlayerManager;
    use sekai::{EntityUid, PlayerState, SessionStatus};

    #[test]
    fn player_manager_tracks_local_player_and_remote_sessions() {
        let mut manager = PlayerManager::new();
        manager.startup("u1", "pedel");
        manager.apply_player_states(
            &[
                PlayerState {
                    user_id: "u1".to_string(),
                    name: "pedel".to_string(),
                    status: SessionStatus::InGame,
                    ping: 12,
                    controlled_entity: Some(EntityUid::new(9)),
                },
                PlayerState {
                    user_id: "u2".to_string(),
                    name: "rika".to_string(),
                    status: SessionStatus::Connected,
                    ping: 34,
                    controlled_entity: None,
                },
            ],
            true,
        );
        let local = manager.local_player().unwrap();
        assert_eq!(local.controlled_entity, Some(EntityUid::new(9)));
        assert_eq!(manager.controlled_entity(), Some(EntityUid::new(9)));
        assert_eq!(local.session.status, SessionStatus::InGame);
        assert_eq!(manager.session("u2").unwrap().name, "rika");
    }

    #[test]
    fn player_manager_applies_incremental_updates_without_pruning() {
        let mut manager = PlayerManager::new();
        manager.startup("u1", "pedel");
        manager.apply_player_states(
            &[
                PlayerState {
                    user_id: "u1".to_string(),
                    name: "pedel".to_string(),
                    status: SessionStatus::InGame,
                    ping: 12,
                    controlled_entity: Some(EntityUid::new(9)),
                },
                PlayerState {
                    user_id: "u2".to_string(),
                    name: "rika".to_string(),
                    status: SessionStatus::Connected,
                    ping: 34,
                    controlled_entity: None,
                },
            ],
            true,
        );

        manager.apply_player_states(
            &[PlayerState {
                user_id: "u1".to_string(),
                name: "pedel".to_string(),
                status: SessionStatus::InGame,
                ping: 18,
                controlled_entity: Some(EntityUid::new(9)),
            }],
            false,
        );

        assert_eq!(manager.session("u2").unwrap().name, "rika");

        manager.apply_player_states(
            &[PlayerState {
                user_id: "u2".to_string(),
                name: "rika".to_string(),
                status: SessionStatus::Disconnected,
                ping: 34,
                controlled_entity: None,
            }],
            false,
        );

        assert!(manager.session("u2").is_none());
    }
}
