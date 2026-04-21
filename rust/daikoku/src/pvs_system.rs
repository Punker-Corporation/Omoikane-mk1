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

    fn map_entity_for_world_map(
        entities: &ServerEntityManager,
        map_id: sekai::MapId,
    ) -> Option<EntityUid> {
        entities
            .inner
            .map_components
            .iter()
            .find(|(_, component)| component.world_map == map_id)
            .map(|(uid, _)| *uid)
    }

    fn augment_with_support_entities(
        &self,
        entities: &ServerEntityManager,
        current: &mut HashSet<EntityUid>,
        controlled: Option<EntityUid>,
    ) {
        if let Some(controlled) = controlled {
            if let Some(transform) = entities.inner.transforms.get(&controlled) {
                if let Some(map_uid) = Self::map_entity_for_world_map(entities, transform.map_id) {
                    current.insert(map_uid);
                }
            }
        }

        let existing: Vec<_> = current.iter().copied().collect();
        for entity in existing {
            if let Some(grid) = entities.inner.map_grids.get(&entity) {
                if let Some(map_uid) = Self::map_entity_for_world_map(entities, grid.parent_map_id) {
                    current.insert(map_uid);
                }
                continue;
            }

            if let Some(transform) = entities.inner.transforms.get(&entity) {
                if let Some(map_uid) = Self::map_entity_for_world_map(entities, transform.map_id) {
                    current.insert(map_uid);
                }
            }
        }
    }

    pub fn calculate_visible_entities(
        &mut self,
        entities: &ServerEntityManager,
        session: &PlayerSession,
    ) -> (Vec<EntityUid>, Vec<EntityUid>, Vec<EntityUid>) {
        let mut current = if !self.culling_enabled {
            entities.inner.entities.iter().copied().collect::<HashSet<_>>()
        } else if let Some(controlled) = session.controlled_entity {
            let controlled_map = entities
                .inner
                .transforms
                .get(&controlled)
                .map(|transform| transform.map_id)
                .unwrap_or(sekai::MapId::NULLSPACE);
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
                    if entities
                        .inner
                        .map_components
                        .get(entity)
                        .is_some_and(|component| component.world_map == controlled_map)
                    {
                        return true;
                    }
                    entities
                        .inner
                        .world_transform(*entity)
                        .map(|world| {
                            world.map_id == controlled_map
                                && (world.world_position - center).length_squared()
                                    <= self.view_size * self.view_size
                        })
                        .unwrap_or(false)
                })
                .collect::<HashSet<_>>()
        } else {
            entities.inner.entities.iter().copied().collect::<HashSet<_>>()
        };
        self.augment_with_support_entities(entities, &mut current, session.controlled_entity);

        let previous = self
            .player_visible_sets
            .entry(session.user_id.clone())
            .or_default()
            .clone();

        let deletions = previous
            .difference(&current)
            .copied()
            .collect::<Vec<_>>();

        let newly_visible = current
            .difference(&previous)
            .copied()
            .collect::<Vec<_>>();

        self.player_visible_sets
            .insert(session.user_id.clone(), current.clone());
        self.player_seen_sets
            .entry(session.user_id.clone())
            .or_default()
            .extend(current.iter().copied());

        (current.into_iter().collect(), deletions, newly_visible)
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
        let (visible, deletions, gained) = pvs.calculate_visible_entities(&entities, session);
        assert!(visible.contains(&uid));
        assert!(deletions.is_empty());
        assert!(gained.contains(&uid));
    }

    #[test]
    fn pvs_system_marks_entities_entering_visibility_as_newly_visible() {
        let mut players = PlayerManager::new(4);
        players.connect("u1", "pedel");
        let session = players.get_session_mut("u1").unwrap();
        session.status = SessionStatus::InGame;

        let mut entities = ServerEntityManager::new();
        let controlled = entities.create_entity(None);
        entities.initialize_entity(controlled);
        session.controlled_entity = Some(controlled);

        let other = entities.create_entity(None);
        entities.initialize_entity(other);
        {
            let transform = entities.inner.transforms.get_mut(&other).unwrap();
            transform.local_position = keisan::Vector2::new(100.0, 0.0);
            transform.rebuild_for_manager();
        }

        let mut pvs = PvsSystem::new();
        pvs.view_size = 8.0;
        pvs.handle_player_status_changed(session);
        let (visible, _, gained) = pvs.calculate_visible_entities(&entities, session);
        assert!(gained.contains(&controlled));
        assert!(!visible.contains(&other));
        assert!(!gained.contains(&other));

        {
            let transform = entities.inner.transforms.get_mut(&other).unwrap();
            transform.local_position = keisan::Vector2::new(1.0, 0.0);
            transform.rebuild_for_manager();
        }

        let (visible, deletions, gained) = pvs.calculate_visible_entities(&entities, session);
        assert!(visible.contains(&other));
        assert!(deletions.is_empty());
        assert_eq!(gained, vec![other]);
    }

    #[test]
    fn pvs_system_keeps_map_entities_visible_for_visible_runtime_entities() {
        let mut players = PlayerManager::new(4);
        players.connect("u1", "pedel");
        let session = players.get_session_mut("u1").unwrap();
        session.status = SessionStatus::InGame;

        let mut entities = ServerEntityManager::new();
        let map_uid = entities.create_entity(None);
        entities.initialize_entity(map_uid);
        entities.inner.ensure_map(sekai::MapId::new(2), map_uid);

        let controlled = entities.create_entity(None);
        entities.initialize_entity(controlled);
        session.controlled_entity = Some(controlled);
        {
            let transform = entities.inner.transforms.get_mut(&controlled).unwrap();
            transform.map_id = sekai::MapId::new(2);
            transform.rebuild_for_manager();
        }

        let mut pvs = PvsSystem::new();
        pvs.handle_player_status_changed(session);
        let (visible, _, gained) = pvs.calculate_visible_entities(&entities, session);
        assert!(visible.contains(&controlled));
        assert!(visible.contains(&map_uid));
        assert!(gained.contains(&map_uid));
    }

    #[test]
    fn pvs_system_does_not_pull_entities_from_other_maps_by_distance_only() {
        let mut players = PlayerManager::new(4);
        players.connect("u1", "pedel");
        let session = players.get_session_mut("u1").unwrap();
        session.status = SessionStatus::InGame;

        let mut entities = ServerEntityManager::new();
        let controlled = entities.create_entity(None);
        entities.initialize_entity(controlled);
        session.controlled_entity = Some(controlled);
        {
            let transform = entities.inner.transforms.get_mut(&controlled).unwrap();
            transform.map_id = sekai::MapId::new(2);
            transform.rebuild_for_manager();
        }

        let other = entities.create_entity(None);
        entities.initialize_entity(other);
        {
            let transform = entities.inner.transforms.get_mut(&other).unwrap();
            transform.map_id = sekai::MapId::new(3);
            transform.local_position = keisan::Vector2::new(1.0, 0.0);
            transform.rebuild_for_manager();
        }

        let mut pvs = PvsSystem::new();
        pvs.view_size = 8.0;
        pvs.handle_player_status_changed(session);
        let (visible, _, _) = pvs.calculate_visible_entities(&entities, session);
        assert!(visible.contains(&controlled));
        assert!(!visible.contains(&other));
    }
}
