use crate::{ServerEntityManager, player_manager::PlayerManager, pvs_system::PvsSystem};
use jikan::GameTick;
use sekai::{GameState, MapManager, RobustSerializer};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(crate) struct ServerGameStateManager {
    acked_states: HashMap<String, GameTick>,
    last_oldest_ack: GameTick,
    serializer: RobustSerializer,
    pvs: PvsSystem,
}

impl ServerGameStateManager {
    pub(crate) fn new() -> Self {
        let mut serializer = RobustSerializer::new();
        serializer.initialize();

        Self {
            acked_states: HashMap::new(),
            last_oldest_ack: GameTick::ZERO,
            serializer,
            pvs: PvsSystem::new(),
        }
    }

    pub(crate) fn handle_client_connected(&mut self, user_id: impl Into<String>) {
        let user_id = user_id.into();
        self.acked_states.insert(user_id.clone(), GameTick::ZERO);
        self.pvs.add_session(user_id);
    }

    pub(crate) fn handle_client_disconnect(&mut self, user_id: &str) {
        self.acked_states.remove(user_id);
        self.pvs.remove_session(user_id);
    }

    pub(crate) fn ack(&mut self, user_id: &str, state_acked: GameTick) {
        let last = self
            .acked_states
            .entry(user_id.to_string())
            .or_insert(GameTick::ZERO);
        if state_acked > *last || state_acked == GameTick::ZERO {
            *last = state_acked;
        }
    }

    fn last_ack_for(&self, user_id: &str) -> GameTick {
        *self.acked_states.get(user_id).unwrap_or(&GameTick::ZERO)
    }

    pub(crate) fn send_game_state_update(
        &mut self,
        entities: &mut ServerEntityManager,
        maps: &mut MapManager,
        players: &mut PlayerManager,
        cur_tick: GameTick,
    ) -> Vec<(String, GameState)> {
        let (updates, oldest_ack) =
            self.build_updates_for_in_game_sessions(entities, maps, players, cur_tick);

        self.post_send_cleanup(entities, maps, players, oldest_ack);

        updates
    }

    fn build_updates_for_in_game_sessions(
        &mut self,
        entities: &mut ServerEntityManager,
        maps: &MapManager,
        players: &PlayerManager,
        cur_tick: GameTick,
    ) -> (Vec<(String, GameState)>, GameTick) {
        let mut updates = Vec::new();
        let mut oldest_ack = GameTick::MAX_VALUE;

        for session in players.in_game_sessions() {
            let (user_id, state, last_ack) =
                self.build_session_update(entities, maps, players, session, cur_tick);
            oldest_ack = oldest_ack.min(last_ack);
            updates.push((user_id, state));
        }

        (updates, oldest_ack)
    }

    fn build_session_update(
        &mut self,
        entities: &mut ServerEntityManager,
        maps: &MapManager,
        players: &PlayerManager,
        session: &crate::player_manager::PlayerSession,
        cur_tick: GameTick,
    ) -> (String, GameState, GameTick) {
        let last_ack = self.last_ack_for(&session.user_id);
        let (visible, deletions, newly_visible) =
            self.pvs.calculate_visible_entities(entities, session);
        let entity_states = entities.build_entity_states_for_sync(
            &mut self.serializer,
            &visible,
            &newly_visible,
            last_ack,
        );
        let map_data =
            maps.collect_game_state_map_data(&entities.inner, &visible, &newly_visible, last_ack);

        (
            session.user_id.clone(),
            GameState {
                from_sequence: last_ack,
                to_sequence: cur_tick,
                last_processed_input: session.last_processed_input,
                entity_states,
                player_states: players.get_player_states_since(last_ack),
                entity_deletions: deletions,
                map_data,
                extrapolated: false,
                payload_size: 0,
            },
            last_ack,
        )
    }

    fn post_send_cleanup(
        &mut self,
        entities: &mut ServerEntityManager,
        maps: &mut MapManager,
        players: &mut PlayerManager,
        oldest_ack: GameTick,
    ) {
        if oldest_ack > self.last_oldest_ack {
            self.last_oldest_ack = oldest_ack;
            entities.cull_deletion_history(oldest_ack);
            players.cull_disconnected_history(oldest_ack);
            maps.cull_deleted_grid_history(oldest_ack);
        }

        for map_id in maps.get_all_map_ids() {
            maps.clear_moved_grids(map_id);
        }
    }
}

impl Default for ServerGameStateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::ServerGameStateManager;
    use crate::{
        ServerEntityManager, player_manager::PlayerManager, transform_system::TransformSystem,
    };
    use jikan::GameTick;
    use sekai::{GridId, MapId, MapManager};

    #[test]
    fn server_game_state_manager_builds_updates_for_in_game_sessions() {
        let mut states = ServerGameStateManager::new();
        states.handle_client_connected("u1");

        let mut players = PlayerManager::new(4);
        players.connect("u1", "pedel");
        players.set_current_tick(GameTick::new(10));
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(2)));
        let grid_id = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(9)), 8);
        let uid = entities.inner.create_entity_uninitialized(Some("mob"));
        entities.inner.initialize_entity(uid);

        let updates = states.send_game_state_update(
            &mut entities,
            &mut maps,
            &mut players,
            GameTick::new(10),
        );
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].1.to_sequence, GameTick::new(10));
        assert!(
            updates[0]
                .1
                .entity_states
                .iter()
                .any(|state| state.uid == uid)
        );
        assert!(updates[0].1.map_data.is_some());

        states.ack("u1", GameTick::new(10));
        let updates = states.send_game_state_update(
            &mut entities,
            &mut maps,
            &mut players,
            GameTick::new(11),
        );
        assert!(updates[0].1.entity_states.is_empty());
        assert!(updates[0].1.map_data.is_none());

        entities.inner.current_tick = GameTick::new(12);
        players.set_current_tick(GameTick::new(12));
        entities.inner.dirty_entity(uid);
        let updates = states.send_game_state_update(
            &mut entities,
            &mut maps,
            &mut players,
            GameTick::new(12),
        );
        assert_eq!(updates[0].1.entity_states.len(), 1);

        maps.mark_grid_moved(map_id, grid_id);
        players.set_current_tick(GameTick::new(13));
        let updates = states.send_game_state_update(
            &mut entities,
            &mut maps,
            &mut players,
            GameTick::new(13),
        );
        assert_eq!(updates[0].1.map_data.as_ref().unwrap().grid_data.len(), 1);
    }

    #[test]
    fn server_game_state_manager_build_game_state_for_session_uses_shared_helpers() {
        let mut states = ServerGameStateManager::new();
        states.handle_client_connected("u1");

        let mut players = PlayerManager::new(4);
        players.connect("u1", "pedel");
        players.set_current_tick(GameTick::new(5));
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(4)));
        let grid_id = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(14)), 8);
        let uid = entities.inner.create_entity_uninitialized(Some("mob"));
        entities.inner.initialize_entity(uid);
        entities.inner.current_tick = GameTick::new(5);
        entities.inner.dirty_entity(uid);
        maps.mark_grid_moved(map_id, grid_id);

        let session = players.get_session("u1").unwrap().clone();
        let (_user_id, state, _last_ack) =
            states.build_session_update(&mut entities, &maps, &players, &session, GameTick::new(5));

        assert_eq!(state.from_sequence, GameTick::ZERO);
        assert_eq!(state.to_sequence, GameTick::new(5));
        assert!(state.entity_states.iter().any(|entity| entity.uid == uid));
        assert!(
            state
                .map_data
                .as_ref()
                .is_some_and(|map_data| map_data.grid_data.contains_key(&grid_id))
        );
    }

    #[test]
    fn server_game_state_manager_sends_full_grid_data_for_newly_visible_grids() {
        let mut states = ServerGameStateManager::new();
        states.handle_client_connected("u1");
        states.pvs.view_size = 8.0;

        let mut players = PlayerManager::new(4);
        players.set_current_tick(GameTick::new(1));
        players.connect("u1", "pedel");
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(2)));

        let controlled = entities.inner.create_entity_uninitialized(Some("mob"));
        entities.inner.initialize_entity(controlled);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(controlled, |transform| {
                    transform.map_id = map_id;
                })
        );
        assert!(players.set_attached_entity("u1", Some(controlled)));

        let near_grid = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(9)), 8);
        maps.set_current_tick(GameTick::new(1));
        assert!(maps.set_tile(
            &mut entities.inner,
            near_grid,
            keisan::Vector2i::new(0, 0),
            sekai::Tile::new(7, sekai::TileRenderFlag(0), 0),
        ));

        let far_grid = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(10)), 8);
        let far_uid = maps.get_grid_euid(far_grid).unwrap();
        {
            assert!(
                entities
                    .inner
                    .mutate_transform_and_reconcile(far_uid, |transform| {
                        transform.local_position = keisan::Vector2::new(100.0, 0.0);
                    })
            );
        }
        maps.set_current_tick(GameTick::new(1));
        assert!(maps.set_tile(
            &mut entities.inner,
            far_grid,
            keisan::Vector2i::new(0, 0),
            sekai::Tile::new(9, sekai::TileRenderFlag(0), 0),
        ));

        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(1));
        let map_data = updates[0].1.map_data.as_ref().unwrap();
        assert!(map_data.grid_data.contains_key(&near_grid));
        assert!(!map_data.grid_data.contains_key(&far_grid));

        states.ack("u1", GameTick::new(1));
        players.set_current_tick(GameTick::new(2));
        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(2));
        assert!(updates[0].1.map_data.is_none());

        states.ack("u1", GameTick::new(2));
        players.set_current_tick(GameTick::new(3));
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(far_uid, |transform| {
                    transform.local_position = keisan::Vector2::new(1.0, 0.0);
                })
        );

        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(3));
        let map_data = updates[0].1.map_data.as_ref().unwrap();
        assert!(map_data.grid_data.contains_key(&far_grid));
        assert!(!map_data.grid_data[&far_grid].chunk_data.is_empty());
    }

    #[test]
    fn server_game_state_manager_preserves_real_chunk_indices_in_map_data() {
        let mut states = ServerGameStateManager::new();
        states.handle_client_connected("u1");

        let mut players = PlayerManager::new(4);
        players.connect("u1", "pedel");
        players.set_current_tick(GameTick::new(1));
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(2)));
        let grid_id = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(9)), 4);
        let controlled = entities.inner.create_entity_uninitialized(Some("mob"));
        entities.inner.initialize_entity(controlled);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(controlled, |transform| {
                    transform.map_id = map_id;
                })
        );
        assert!(players.set_attached_entity("u1", Some(controlled)));

        maps.set_current_tick(GameTick::new(1));
        assert!(maps.set_tile(
            &mut entities.inner,
            grid_id,
            keisan::Vector2i::new(-1, 0),
            sekai::Tile::new(1, sekai::TileRenderFlag(0), 0),
        ));
        assert!(maps.set_tile(
            &mut entities.inner,
            grid_id,
            keisan::Vector2i::new(8, 0),
            sekai::Tile::new(2, sekai::TileRenderFlag(0), 0),
        ));

        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(1));
        let map_data = updates[0].1.map_data.as_ref().unwrap();
        let chunk_indices = map_data.grid_data[&grid_id]
            .chunk_data
            .iter()
            .map(|datum| datum.index)
            .collect::<Vec<_>>();
        assert_eq!(
            chunk_indices,
            vec![keisan::Vector2i::new(-1, 0), keisan::Vector2i::new(2, 0)]
        );
    }

    #[test]
    fn server_game_state_manager_only_sends_moved_grids_that_are_visible() {
        let mut states = ServerGameStateManager::new();
        states.handle_client_connected("u1");
        states.pvs.view_size = 8.0;

        let mut players = PlayerManager::new(4);
        players.set_current_tick(GameTick::new(1));
        players.connect("u1", "pedel");
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(2)));

        let controlled = entities.inner.create_entity_uninitialized(Some("mob"));
        entities.inner.initialize_entity(controlled);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(controlled, |transform| {
                    transform.map_id = map_id;
                })
        );
        assert!(players.set_attached_entity("u1", Some(controlled)));

        let near_grid = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(9)), 8);
        let far_grid = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(10)), 8);
        let far_uid = maps.get_grid_euid(far_grid).unwrap();
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(far_uid, |transform| {
                    transform.local_position = keisan::Vector2::new(100.0, 0.0);
                })
        );

        let _ =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(1));
        states.ack("u1", GameTick::new(1));

        players.set_current_tick(GameTick::new(2));
        maps.mark_grid_moved(map_id, near_grid);
        maps.mark_grid_moved(map_id, far_grid);
        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(2));
        let map_data = updates[0].1.map_data.as_ref().unwrap();
        assert!(map_data.grid_data.contains_key(&near_grid));
        assert!(!map_data.grid_data.contains_key(&far_grid));
    }

    #[test]
    fn server_game_state_manager_sends_deleted_grids_incrementally() {
        let mut states = ServerGameStateManager::new();
        states.handle_client_connected("u1");

        let mut players = PlayerManager::new(4);
        players.set_current_tick(GameTick::new(1));
        players.connect("u1", "pedel");
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(2)));
        let grid_id = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(9)), 8);

        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(1));
        assert!(
            updates[0]
                .1
                .map_data
                .as_ref()
                .unwrap()
                .grid_data
                .contains_key(&grid_id)
        );

        states.ack("u1", GameTick::new(1));
        players.set_current_tick(GameTick::new(2));
        maps.set_current_tick(GameTick::new(2));
        maps.delete_grid(&mut entities.inner, grid_id);
        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(2));
        let map_data = updates[0].1.map_data.as_ref().unwrap();
        assert_eq!(map_data.deleted_grids, vec![grid_id]);
        assert!(map_data.grid_data.is_empty());
    }

    #[test]
    fn server_game_state_manager_sends_only_changed_chunks_after_ack() {
        let mut states = ServerGameStateManager::new();
        states.handle_client_connected("u1");

        let mut players = PlayerManager::new(4);
        players.set_current_tick(GameTick::new(1));
        players.connect("u1", "pedel");
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(2)));
        let grid_id = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(9)), 4);
        let controlled = entities.inner.create_entity_uninitialized(Some("mob"));
        entities.inner.initialize_entity(controlled);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(controlled, |transform| {
                    transform.map_id = map_id;
                })
        );
        assert!(players.set_attached_entity("u1", Some(controlled)));

        maps.set_current_tick(GameTick::new(1));
        assert!(maps.set_tile(
            &mut entities.inner,
            grid_id,
            keisan::Vector2i::new(0, 0),
            sekai::Tile::new(1, sekai::TileRenderFlag(0), 0),
        ));
        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(1));
        let initial = updates[0].1.map_data.as_ref().unwrap();
        assert_eq!(initial.grid_data[&grid_id].chunk_data.len(), 1);
        assert_eq!(
            initial.grid_data[&grid_id].chunk_data[0].index,
            keisan::Vector2i::new(0, 0)
        );

        states.ack("u1", GameTick::new(1));
        players.set_current_tick(GameTick::new(2));
        maps.set_current_tick(GameTick::new(2));
        assert!(maps.set_tile(
            &mut entities.inner,
            grid_id,
            keisan::Vector2i::new(4, 0),
            sekai::Tile::new(2, sekai::TileRenderFlag(0), 0),
        ));
        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(2));
        let changed = updates[0].1.map_data.as_ref().unwrap();
        assert_eq!(changed.grid_data[&grid_id].chunk_data.len(), 1);
        assert_eq!(
            changed.grid_data[&grid_id].chunk_data[0].index,
            keisan::Vector2i::new(1, 0)
        );
    }

    #[test]
    fn server_game_state_manager_sends_deleted_chunks_incrementally() {
        let mut states = ServerGameStateManager::new();
        states.handle_client_connected("u1");

        let mut players = PlayerManager::new(4);
        players.set_current_tick(GameTick::new(1));
        players.connect("u1", "pedel");
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(2)));
        let grid_id = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(9)), 4);
        let controlled = entities.inner.create_entity_uninitialized(Some("mob"));
        entities.inner.initialize_entity(controlled);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(controlled, |transform| {
                    transform.map_id = map_id;
                })
        );
        assert!(players.set_attached_entity("u1", Some(controlled)));

        maps.set_current_tick(GameTick::new(1));
        assert!(maps.set_tile(
            &mut entities.inner,
            grid_id,
            keisan::Vector2i::new(0, 0),
            sekai::Tile::new(1, sekai::TileRenderFlag(0), 0),
        ));
        let _ =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(1));

        states.ack("u1", GameTick::new(1));
        players.set_current_tick(GameTick::new(2));
        maps.set_current_tick(GameTick::new(2));
        assert!(maps.remove_chunk(&mut entities.inner, grid_id, keisan::Vector2i::new(0, 0)));
        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(2));
        let changed = updates[0].1.map_data.as_ref().unwrap();
        assert_eq!(changed.grid_data[&grid_id].chunk_data.len(), 1);
        assert!(changed.grid_data[&grid_id].chunk_data[0].is_deleted());
        assert_eq!(
            changed.grid_data[&grid_id].chunk_data[0].index,
            keisan::Vector2i::new(0, 0)
        );
    }

    #[test]
    fn server_game_state_manager_sends_incremental_player_states_and_disconnects() {
        let mut states = ServerGameStateManager::new();
        states.handle_client_connected("u1");

        let mut players = PlayerManager::new(4);
        players.set_current_tick(GameTick::new(1));
        players.connect("u1", "pedel");
        players.connect("u2", "rika");
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);

        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(1));
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].1.player_states.len(), 2);

        states.ack("u1", GameTick::new(1));
        players.set_current_tick(GameTick::new(2));
        assert!(players.set_ping("u2", 44));
        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(2));
        assert_eq!(updates[0].1.player_states.len(), 1);
        assert_eq!(updates[0].1.player_states[0].user_id, "u2");
        assert_eq!(updates[0].1.player_states[0].ping, 44);

        states.ack("u1", GameTick::new(2));
        players.set_current_tick(GameTick::new(3));
        players.disconnect("u2");
        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(3));
        assert_eq!(updates[0].1.player_states.len(), 1);
        assert_eq!(updates[0].1.player_states[0].user_id, "u2");
        assert_eq!(
            updates[0].1.player_states[0].status,
            sekai::SessionStatus::Disconnected
        );
    }

    #[test]
    fn server_game_state_manager_sends_full_state_for_newly_visible_entities() {
        let mut states = ServerGameStateManager::new();
        states.handle_client_connected("u1");
        states.pvs.view_size = 8.0;

        let mut players = PlayerManager::new(4);
        players.set_current_tick(GameTick::new(1));
        players.connect("u1", "pedel");
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);

        let controlled = entities.inner.create_entity_uninitialized(Some("mob"));
        entities.inner.initialize_entity(controlled);
        assert!(players.set_attached_entity("u1", Some(controlled)));

        let other = entities.inner.create_entity_uninitialized(Some("crate"));
        entities.inner.initialize_entity(other);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(other, |transform| {
                    transform.local_position = keisan::Vector2::new(100.0, 0.0);
                })
        );

        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(1));
        assert_eq!(updates[0].1.entity_states.len(), 1);
        assert_eq!(updates[0].1.entity_states[0].uid, controlled);

        states.ack("u1", GameTick::new(1));
        players.set_current_tick(GameTick::new(2));
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(other, |transform| {
                    transform.local_position = keisan::Vector2::new(1.0, 0.0);
                })
        );

        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(2));
        assert!(
            updates[0]
                .1
                .entity_states
                .iter()
                .any(|state| state.uid == other)
        );
        let full_state = updates[0]
            .1
            .entity_states
            .iter()
            .find(|state| state.uid == other)
            .unwrap();
        assert!(
            full_state
                .component_changes
                .iter()
                .any(|change| change.net_id == 1 && change.created)
        );
    }

    #[test]
    fn server_game_state_manager_sends_runtime_component_deltas_after_immediate_physics_changes() {
        let mut states = ServerGameStateManager::new();
        states.handle_client_connected("u1");

        let mut players = PlayerManager::new(4);
        players.set_current_tick(GameTick::new(1));
        players.connect("u1", "pedel");
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(8)));
        let map_uid = maps.get_map_entity_id(map_id);
        let grid_id = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(12)), 8);
        let grid_uid = maps.get_grid_euid(grid_id).unwrap();

        let controlled = entities.inner.create_entity_uninitialized(Some("mob"));
        entities.inner.initialize_entity(controlled);
        let _ = entities.inner.apply_transform_state(
            controlled,
            sekai::TransformComponentState {
                local_position: keisan::Vector2::ZERO,
                rotation: keisan::Angle::ZERO,
                parent_id: grid_uid,
                map_id,
                grid_id,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(controlled);
        body.can_collide = true;
        body.set_body_type(sekai::BodyType::Dynamic);
        body.awake = true;
        let _ = entities.inner.insert_fixture_and_reconcile(
            controlled,
            butsuri::Fixture::new(
                "main",
                butsuri::PhysShape::Aabb(butsuri::AabbShape::new(
                    keisan::Box2::new(-0.5, -0.5, 0.5, 0.5),
                    0.0,
                )),
            ),
        );
        assert!(players.set_attached_entity("u1", Some(controlled)));

        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(1));
        assert!(
            updates[0]
                .1
                .entity_states
                .iter()
                .any(|state| state.uid == map_uid)
        );

        states.ack("u1", GameTick::new(1));
        entities.inner.current_tick = GameTick::new(2);
        players.set_current_tick(GameTick::new(2));
        maps.set_current_tick(GameTick::new(2));
        let physics = crate::physics_system::PhysicsSystem::new();
        assert!(physics.set_awake(&mut entities, controlled, false));

        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(2));
        let map_state = updates[0]
            .1
            .entity_states
            .iter()
            .find(|state| state.uid == map_uid)
            .unwrap();
        assert!(
            map_state
                .component_changes
                .iter()
                .any(|change| change.net_id == 11)
        );
    }

    #[test]
    fn server_game_state_manager_sends_physics_map_delta_when_linear_velocity_wakes_body() {
        let mut states = ServerGameStateManager::new();
        states.handle_client_connected("u1");

        let mut players = PlayerManager::new(4);
        players.set_current_tick(GameTick::new(1));
        players.connect("u1", "pedel");
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(9)));
        let map_uid = maps.get_map_entity_id(map_id);

        let controlled = entities.inner.create_entity_uninitialized(Some("mob"));
        entities.inner.initialize_entity(controlled);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(controlled, |transform| {
                    transform.map_id = map_id;
                })
        );
        let body = entities.inner.ensure_physics(controlled);
        body.can_collide = true;
        body.set_body_type(sekai::BodyType::Dynamic);
        body.awake = false;
        let _ = entities.inner.insert_fixture_and_reconcile(
            controlled,
            butsuri::Fixture::new(
                "main",
                butsuri::PhysShape::Aabb(butsuri::AabbShape::new(
                    keisan::Box2::new(-0.5, -0.5, 0.5, 0.5),
                    0.0,
                )),
            ),
        );
        assert!(players.set_attached_entity("u1", Some(controlled)));

        let _ =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(1));
        states.ack("u1", GameTick::new(1));

        entities.inner.current_tick = GameTick::new(2);
        players.set_current_tick(GameTick::new(2));
        maps.set_current_tick(GameTick::new(2));
        let physics = crate::physics_system::PhysicsSystem::new();
        assert!(physics.set_linear_velocity(
            &mut entities,
            controlled,
            keisan::Vector2::new(1.0, 0.0)
        ));

        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(2));
        let map_state = updates[0]
            .1
            .entity_states
            .iter()
            .find(|state| state.uid == map_uid)
            .unwrap();
        assert!(
            map_state
                .component_changes
                .iter()
                .any(|change| change.net_id == 11)
        );
    }

    #[test]
    fn server_game_state_manager_sends_physics_map_delta_when_gravity_changes() {
        let mut states = ServerGameStateManager::new();
        states.handle_client_connected("u1");

        let mut players = PlayerManager::new(4);
        players.set_current_tick(GameTick::new(1));
        players.connect("u1", "pedel");
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(10)));
        let map_uid = maps.get_map_entity_id(map_id);

        let controlled = entities.inner.create_entity_uninitialized(Some("mob"));
        entities.inner.initialize_entity(controlled);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(controlled, |transform| {
                    transform.map_id = map_id;
                })
        );
        assert!(players.set_attached_entity("u1", Some(controlled)));

        let _ =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(1));
        states.ack("u1", GameTick::new(1));

        entities.inner.current_tick = GameTick::new(2);
        players.set_current_tick(GameTick::new(2));
        maps.set_current_tick(GameTick::new(2));
        let physics = crate::physics_system::PhysicsSystem::new();
        assert!(physics.set_map_gravity(
            &mut entities,
            &maps,
            map_id,
            keisan::Vector2::new(0.0, -9.8)
        ));

        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(2));
        let map_state = updates[0]
            .1
            .entity_states
            .iter()
            .find(|state| state.uid == map_uid)
            .unwrap();
        assert!(
            map_state
                .component_changes
                .iter()
                .any(|change| change.net_id == 11)
        );
    }

    #[test]
    fn server_game_state_manager_does_not_resend_idle_awake_physics_without_real_change() {
        let mut states = ServerGameStateManager::new();
        states.handle_client_connected("u1");

        let mut players = PlayerManager::new(4);
        players.set_current_tick(GameTick::new(1));
        players.connect("u1", "pedel");
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(32)));

        let controlled = entities.inner.create_entity_uninitialized(Some("mob"));
        entities.inner.initialize_entity(controlled);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(controlled, |transform| {
                    transform.map_id = map_id;
                })
        );
        let body = entities.inner.ensure_physics(controlled);
        body.can_collide = true;
        body.set_body_type(sekai::BodyType::Dynamic);
        body.awake = true;
        body.sleeping_allowed = false;
        assert!(players.set_attached_entity("u1", Some(controlled)));

        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(1));
        assert!(
            updates[0]
                .1
                .entity_states
                .iter()
                .any(|state| state.uid == controlled)
        );

        states.ack("u1", GameTick::new(1));
        entities.inner.current_tick = GameTick::new(2);
        players.set_current_tick(GameTick::new(2));
        maps.set_current_tick(GameTick::new(2));
        let physics = crate::physics_system::PhysicsSystem::new();
        let mut transforms = TransformSystem::new();
        assert_eq!(
            physics.step_simulation(&mut entities, &mut transforms, 0.1),
            0
        );

        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(2));
        assert!(updates[0].1.entity_states.is_empty());
    }

    #[test]
    fn server_game_state_manager_does_not_resend_physics_for_force_or_torque_alone() {
        let mut states = ServerGameStateManager::new();
        states.handle_client_connected("u1");

        let mut players = PlayerManager::new(4);
        players.set_current_tick(GameTick::new(1));
        players.connect("u1", "pedel");
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(34)));

        let controlled = entities.inner.create_entity_uninitialized(Some("mob"));
        entities.inner.initialize_entity(controlled);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(controlled, |transform| {
                    transform.map_id = map_id;
                })
        );
        let body = entities.inner.ensure_physics(controlled);
        body.can_collide = true;
        body.set_body_type(sekai::BodyType::Dynamic);
        body.awake = true;
        assert!(players.set_attached_entity("u1", Some(controlled)));

        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(1));
        assert!(
            updates[0]
                .1
                .entity_states
                .iter()
                .any(|state| state.uid == controlled)
        );

        states.ack("u1", GameTick::new(1));
        entities.inner.current_tick = GameTick::new(2);
        players.set_current_tick(GameTick::new(2));
        maps.set_current_tick(GameTick::new(2));
        let physics = crate::physics_system::PhysicsSystem::new();
        assert!(physics.apply_force(&mut entities, controlled, keisan::Vector2::new(2.0, 0.0)));
        assert!(physics.apply_torque(&mut entities, controlled, 4.0));

        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(2));
        assert!(updates[0].1.entity_states.is_empty());
    }

    #[test]
    fn server_game_state_manager_resends_physics_after_force_drives_velocity_change() {
        let mut states = ServerGameStateManager::new();
        states.handle_client_connected("u1");

        let mut players = PlayerManager::new(4);
        players.set_current_tick(GameTick::new(1));
        players.connect("u1", "pedel");
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(35)));

        let controlled = entities.inner.create_entity_uninitialized(Some("mob"));
        entities.inner.initialize_entity(controlled);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(controlled, |transform| {
                    transform.map_id = map_id;
                })
        );
        let body = entities.inner.ensure_physics(controlled);
        body.can_collide = true;
        body.set_body_type(sekai::BodyType::Dynamic);
        body.awake = true;
        assert!(players.set_attached_entity("u1", Some(controlled)));

        let _ =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(1));
        states.ack("u1", GameTick::new(1));

        entities.inner.current_tick = GameTick::new(2);
        players.set_current_tick(GameTick::new(2));
        maps.set_current_tick(GameTick::new(2));
        let physics = crate::physics_system::PhysicsSystem::new();
        assert!(physics.apply_force(&mut entities, controlled, keisan::Vector2::new(2.0, 0.0)));
        let mut transforms = TransformSystem::new();
        assert_eq!(
            physics.step_simulation(&mut entities, &mut transforms, 0.5),
            1
        );

        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(2));
        let entity_state = updates[0]
            .1
            .entity_states
            .iter()
            .find(|state| state.uid == controlled)
            .unwrap();
        assert!(
            entity_state
                .component_changes
                .iter()
                .any(|change| change.net_id == 5)
        );
    }
}
