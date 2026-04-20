use crate::{PlayerManager, PlayerSession, ServerEntityManager};
use sekai::{EntityUid, SessionStatus, TransformResolver};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct PvsSystem {
    pub culling_enabled: bool,
    pub view_size: f32,
    seen_all: HashSet<String>,
    player_visible_sets: HashMap<String, HashSet<EntityUid>>,
    player_seen_sets: HashMap<String, HashSet<EntityUid>>,
}

impl PvsSystem {
    pub fn new() -> Self {
        Self {
            culling_enabled: true,
            view_size: 16.0,
            seen_all: HashSet::new(),
            player_visible_sets: HashMap::new(),
            player_seen_sets: HashMap::new(),
        }
    }

    pub fn handle_player_status_changed(&mut self, session: &PlayerSession) {
        if session.status == SessionStatus::InGame {
            self.player_visible_sets
                .entry(session.user_id.clone())
                .or_default();
            self.player_seen_sets.entry(session.user_id.clone()).or_default();
        } else if session.status == SessionStatus::Disconnected {
            self.player_visible_sets.remove(&session.user_id);
            self.player_seen_sets.remove(&session.user_id);
            self.seen_all.remove(&session.user_id);
        }
    }

    pub fn add_session(&mut self, user_id: impl Into<String>) {
        let user_id = user_id.into();
        self.player_visible_sets.entry(user_id.clone()).or_default();
        self.player_seen_sets.entry(user_id).or_default();
    }

    pub fn remove_session(&mut self, user_id: &str) {
        self.player_visible_sets.remove(user_id);
        self.player_seen_sets.remove(user_id);
        self.seen_all.remove(user_id);
    }

    pub fn cleanup(&mut self, players: &PlayerManager) {
        if !self.culling_enabled {
            self.seen_all = players.sessions().map(|s| s.user_id.clone()).collect();
        } else {
            self.seen_all.clear();
        }
    }

    pub fn calculate_visible_entities(
        &mut self,
        entities: &ServerEntityManager,
        session: &PlayerSession,
    ) -> (Vec<EntityUid>, Vec<EntityUid>) {
        let current = if !self.culling_enabled {
            entities.inner.entities.iter().copied().collect::<HashSet<_>>()
        } else if let Some(controlled) = session.controlled_entity {
            let center = entities
                .inner
                .world_transform(controlled)
                .map(|world| world.world_position)
                .unwrap_or(keisan::Vector2::ZERO);
            entities
                .inner
                .entities
                .iter()
                .copied()
                .filter(|entity| {
                    entities
                        .inner
                        .world_transform(*entity)
                        .map(|world| (world.world_position - center).length_squared() <= self.view_size * self.view_size)
                        .unwrap_or(false)
                })
                .collect::<HashSet<_>>()
        } else {
            entities.inner.entities.iter().copied().collect::<HashSet<_>>()
        };

        let previous = self
            .player_visible_sets
            .entry(session.user_id.clone())
            .or_default()
            .clone();

        let deletions = previous
            .difference(&current)
            .copied()
            .collect::<Vec<_>>();

        self.player_visible_sets
            .insert(session.user_id.clone(), current.clone());
        self.player_seen_sets
            .entry(session.user_id.clone())
            .or_default()
            .extend(current.iter().copied());

        (current.into_iter().collect(), deletions)
    }
}

impl Default for PvsSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::PvsSystem;
    use crate::{PlayerManager, ServerEntityManager};
    use sekai::SessionStatus;

    #[test]
    fn pvs_system_tracks_visible_entities_per_player() {
        let mut players = PlayerManager::new(4);
        players.connect("u1", "pedel");
        let session = players.get_session_mut("u1").unwrap();
        session.status = SessionStatus::InGame;

        let mut entities = ServerEntityManager::new();
        let uid = entities.create_entity(None);
        entities.initialize_entity(uid);
        session.controlled_entity = Some(uid);

        let mut pvs = PvsSystem::new();
        pvs.handle_player_status_changed(session);
        let (visible, deletions) = pvs.calculate_visible_entities(&entities, session);
        assert!(visible.contains(&uid));
        assert!(deletions.is_empty());
    }
}
