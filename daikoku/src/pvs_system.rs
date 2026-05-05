use crate::{player_manager::PlayerSession, server_entity_manager::ServerEntityManager};
use sekai::{EntityUid, TransformResolver};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub(crate) struct PvsSystem {
    pub(crate) culling_enabled: bool,
    pub(crate) view_size: f32,
    player_visible_sets: HashMap<String, HashSet<EntityUid>>,
    player_seen_sets: HashMap<String, HashSet<EntityUid>>,
}

impl PvsSystem {
    pub(crate) fn new() -> Self {
        Self {
            culling_enabled: true,
            view_size: 16.0,
            player_visible_sets: HashMap::new(),
            player_seen_sets: HashMap::new(),
        }
    }

    pub(crate) fn add_session(&mut self, user_id: impl Into<String>) {
        let user_id = user_id.into();
        self.player_visible_sets.entry(user_id.clone()).or_default();
        self.player_seen_sets.entry(user_id).or_default();
    }

    pub(crate) fn remove_session(&mut self, user_id: &str) {
        self.player_visible_sets.remove(user_id);
        self.player_seen_sets.remove(user_id);
    }

    fn map_entity_for_world_map(
        entities: &ServerEntityManager,
        map_id: sekai::MapId,
    ) -> Option<EntityUid> {
        entities.inner.map_entity_for(map_id)
    }

    fn augment_with_support_entities(
        &self,
        entities: &ServerEntityManager,
        current: &mut HashSet<EntityUid>,
        controlled: Option<EntityUid>,
    ) {
        if let Some(controlled) = controlled {
            let map_id = entities.inner.map_id_for(controlled);
            if let Some(map_uid) = Self::map_entity_for_world_map(entities, map_id) {
                current.insert(map_uid);
            }
        }

        let existing: Vec<_> = current.iter().copied().collect();
        for entity in existing {
            if let Some(grid) = entities.inner.map_grids.get(&entity) {
                if let Some(map_uid) = Self::map_entity_for_world_map(entities, grid.parent_map_id)
                {
                    current.insert(map_uid);
                }
                continue;
            }

            let map_id = entities.inner.map_id_for(entity);
            if let Some(map_uid) = Self::map_entity_for_world_map(entities, map_id) {
                current.insert(map_uid);
            }
        }
    }

    pub(crate) fn calculate_visible_entities(
        &mut self,
        entities: &ServerEntityManager,
        session: &PlayerSession,
    ) -> (Vec<EntityUid>, Vec<EntityUid>, Vec<EntityUid>) {
        let mut current = if !self.culling_enabled {
            entities
                .inner
                .entity_uids(true)
                .into_iter()
                .collect::<HashSet<_>>()
        } else if let Some(controlled) = session.controlled_entity {
            let controlled_map = entities.inner.map_id_for(controlled);
            let center = entities
                .inner
                .world_transform(controlled)
                .map(|world| world.world_position)
                .unwrap_or(keisan::Vector2::ZERO);
            entities
                .inner
                .entities_in_map_radius(controlled_map, center, self.view_size, true, true)
                .into_iter()
                .collect::<HashSet<_>>()
        } else {
            entities
                .inner
                .entity_uids(true)
                .into_iter()
                .collect::<HashSet<_>>()
        };
        self.augment_with_support_entities(entities, &mut current, session.controlled_entity);

        let previous = self
            .player_visible_sets
            .entry(session.user_id.clone())
            .or_default()
            .clone();

        let deletions = previous.difference(&current).copied().collect::<Vec<_>>();

        let newly_visible = current.difference(&previous).copied().collect::<Vec<_>>();

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
    use crate::{player_manager::PlayerManager, server_entity_manager::ServerEntityManager};

    #[test]
    fn pvs_system_tracks_visible_entities_per_player() {
        let mut players = PlayerManager::new(4);
        players.connect("u1", "pedel");
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        assert!(players.set_attached_entity("u1", Some(uid)));
        let session = players.get_session("u1").unwrap();

        let mut pvs = PvsSystem::new();
        pvs.add_session("u1");
        let (visible, deletions, gained) = pvs.calculate_visible_entities(&entities, session);
        assert!(visible.contains(&uid));
        assert!(deletions.is_empty());
        assert!(gained.contains(&uid));
    }

    #[test]
    fn pvs_system_marks_entities_entering_visibility_as_newly_visible() {
        let mut players = PlayerManager::new(4);
        players.connect("u1", "pedel");
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let controlled = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(controlled);
        assert!(players.set_attached_entity("u1", Some(controlled)));
        let session = players.get_session("u1").unwrap();

        let other = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(other);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(other, |transform| {
                    transform.local_position = keisan::Vector2::new(100.0, 0.0);
                })
        );

        let mut pvs = PvsSystem::new();
        pvs.view_size = 8.0;
        pvs.add_session("u1");
        let (visible, _, gained) = pvs.calculate_visible_entities(&entities, session);
        assert!(gained.contains(&controlled));
        assert!(!visible.contains(&other));
        assert!(!gained.contains(&other));

        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(other, |transform| {
                    transform.local_position = keisan::Vector2::new(1.0, 0.0);
                })
        );

        let (visible, deletions, gained) = pvs.calculate_visible_entities(&entities, session);
        assert!(visible.contains(&other));
        assert!(deletions.is_empty());
        assert_eq!(gained, vec![other]);
    }

    #[test]
    fn pvs_system_keeps_map_entities_visible_for_visible_runtime_entities() {
        let mut players = PlayerManager::new(4);
        players.connect("u1", "pedel");
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let map_uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(map_uid);
        entities.inner.ensure_map(sekai::MapId::new(2), map_uid);

        let controlled = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(controlled);
        assert!(players.set_attached_entity("u1", Some(controlled)));
        let session = players.get_session("u1").unwrap();
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(controlled, |transform| {
                    transform.map_id = sekai::MapId::new(2);
                })
        );

        let mut pvs = PvsSystem::new();
        pvs.add_session("u1");
        let (visible, _, gained) = pvs.calculate_visible_entities(&entities, session);
        assert!(visible.contains(&controlled));
        assert!(visible.contains(&map_uid));
        assert!(gained.contains(&map_uid));
    }

    #[test]
    fn pvs_system_does_not_pull_entities_from_other_maps_by_distance_only() {
        let mut players = PlayerManager::new(4);
        players.connect("u1", "pedel");
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let controlled = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(controlled);
        assert!(players.set_attached_entity("u1", Some(controlled)));
        let session = players.get_session("u1").unwrap();
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(controlled, |transform| {
                    transform.map_id = sekai::MapId::new(2);
                })
        );

        let other = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(other);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(other, |transform| {
                    transform.map_id = sekai::MapId::new(3);
                    transform.local_position = keisan::Vector2::new(1.0, 0.0);
                })
        );

        let mut pvs = PvsSystem::new();
        pvs.view_size = 8.0;
        pvs.add_session("u1");
        let (visible, _, _) = pvs.calculate_visible_entities(&entities, session);
        assert!(visible.contains(&controlled));
        assert!(!visible.contains(&other));
    }
}
