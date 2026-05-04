use crate::{PlayerManager, PvsSystem, ServerEntityManager};
use jikan::GameTick;
use sekai::{
    ChunkDatum, GameState, GameStateMapData, GridDatum, MapManager, RobustSerializer,
    SerializedEntityState,
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ServerGameStateManager {
    acked_states: HashMap<String, GameTick>,
    last_oldest_ack: GameTick,
    serializer: RobustSerializer,
    pub pvs: PvsSystem,
}

impl ServerGameStateManager {
    pub fn new() -> Self {
        Self {
            acked_states: HashMap::new(),
            last_oldest_ack: GameTick::ZERO,
            serializer: RobustSerializer::new(),
            pvs: PvsSystem::new(),
        }
    }

    pub fn initialize(&mut self) {
        self.serializer.initialize();
    }

    pub fn handle_client_connected(&mut self, user_id: impl Into<String>) {
        let user_id = user_id.into();
        self.acked_states.insert(user_id.clone(), GameTick::ZERO);
        self.pvs.add_session(user_id);
    }

    pub fn handle_client_disconnect(&mut self, user_id: &str) {
        self.acked_states.remove(user_id);
        self.pvs.remove_session(user_id);
    }

    pub fn ack(&mut self, user_id: &str, state_acked: GameTick) {
        let last = self
            .acked_states
            .entry(user_id.to_string())
            .or_insert(GameTick::ZERO);
        if state_acked > *last || state_acked == GameTick::ZERO {
            *last = state_acked;
        }
    }

    pub fn send_game_state_update(
        &mut self,
        entities: &mut ServerEntityManager,
        maps: &mut MapManager,
        players: &mut PlayerManager,
        cur_tick: GameTick,
    ) -> Vec<(String, GameState)> {
        let mut updates = Vec::new();
        let mut oldest_ack = GameTick::MAX_VALUE;

        for session in players.in_game_sessions() {
            let last_ack = *self
                .acked_states
                .get(&session.user_id)
                .unwrap_or(&GameTick::ZERO);
            let (visible, deletions, newly_visible) =
                self.pvs.calculate_visible_entities(entities, session);
            let entity_states =
                self.collect_entity_states(entities, &visible, &newly_visible, last_ack);
            let map_data = self.collect_map_data(
                entities,
                maps,
                &visible,
                &deletions,
                &newly_visible,
                last_ack,
            );
            let state = GameState {
                from_sequence: last_ack,
                to_sequence: cur_tick,
                last_processed_input: session.last_processed_input,
                entity_states,
                player_states: players.get_player_states_since(last_ack),
                entity_deletions: deletions,
                map_data,
                extrapolated: false,
                payload_size: 0,
            };
            if last_ack < oldest_ack {
                oldest_ack = last_ack;
            }
            updates.push((session.user_id.clone(), state));
        }

        if oldest_ack > self.last_oldest_ack {
            self.last_oldest_ack = oldest_ack;
            entities.cull_deletion_history(oldest_ack);
            players.cull_disconnected_history(oldest_ack);
            maps.cull_deleted_grid_history(oldest_ack);
        }

        for map_id in maps.get_all_map_ids() {
            maps.clear_moved_grids(map_id);
        }

        updates
    }

    fn collect_entity_states(
        &mut self,
        entities: &mut ServerEntityManager,
        ids: &[sekai::EntityUid],
        newly_visible: &[sekai::EntityUid],
        from_tick: GameTick,
    ) -> Vec<SerializedEntityState> {
        let mut states = Vec::new();
        let newly_visible = newly_visible
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        for uid in ids.iter().copied() {
            if newly_visible.contains(&uid) {
                if let Some(state) = entities.build_entity_state(&mut self.serializer, uid)
                    && !state.component_changes.is_empty()
                {
                    states.push(state);
                }
                continue;
            }
            if !entities.entity_dirty_since(uid, from_tick) {
                continue;
            }
            if let Some(state) =
                entities.build_entity_state_since(&mut self.serializer, uid, from_tick)
                && !state.component_changes.is_empty()
            {
                states.push(state);
            }
        }
        states
    }

    fn collect_map_data(
        &self,
        entities: &ServerEntityManager,
        maps: &MapManager,
        visible: &[sekai::EntityUid],
        _deletions: &[sekai::EntityUid],
        newly_visible: &[sekai::EntityUid],
        from_tick: GameTick,
    ) -> Option<GameStateMapData> {
        let mut grid_data = HashMap::new();
        let mut deleted_grids = Vec::new();
        let all_grids = from_tick == GameTick::ZERO;
        let visible_grids = visible
            .iter()
            .filter_map(|uid| {
                entities
                    .inner
                    .map_grid_components
                    .get(uid)
                    .map(|component| component.grid_index)
            })
            .collect::<std::collections::HashSet<_>>();
        let newly_visible_grids = newly_visible
            .iter()
            .filter_map(|uid| {
                entities
                    .inner
                    .map_grid_components
                    .get(uid)
                    .map(|component| component.grid_index)
            })
            .collect::<std::collections::HashSet<_>>();
        let moved = maps
            .get_all_map_ids()
            .into_iter()
            .flat_map(|map_id| maps.get_moved_grids(map_id).into_iter())
            .collect::<std::collections::HashSet<_>>();

        for grid in entities.inner.map_grids.values() {
            if all_grids {
                if !visible_grids.contains(&grid.index) {
                    continue;
                }
            } else {
                let moved_match =
                    moved.contains(&grid.index) && visible_grids.contains(&grid.index);
                let newly_visible_match = newly_visible_grids.contains(&grid.index);
                let changed_chunks = maps.get_changed_chunks_since(grid.index, from_tick);
                let chunk_changed_match =
                    !changed_chunks.is_empty() && visible_grids.contains(&grid.index);
                if !moved_match && !newly_visible_match && !chunk_changed_match {
                    continue;
                }
            }
            let mut chunk_data = Vec::new();
            if all_grids || newly_visible_grids.contains(&grid.index) {
                for chunk_index in grid.chunk_indices() {
                    let Some(chunk) = grid.try_get_chunk(chunk_index) else {
                        continue;
                    };
                    let mut tile_data = Vec::with_capacity((grid.chunk_size as usize).pow(2));
                    for x in 0..grid.chunk_size {
                        for y in 0..grid.chunk_size {
                            tile_data.push(chunk.get_tile(x, y));
                        }
                    }
                    chunk_data.push(ChunkDatum::create_modified(chunk_index, tile_data));
                }
            } else {
                for (chunk_index, deleted) in maps.get_changed_chunks_since(grid.index, from_tick) {
                    if deleted {
                        chunk_data.push(ChunkDatum::create_deleted(chunk_index));
                        continue;
                    }
                    let Some(chunk) = grid.try_get_chunk(chunk_index) else {
                        continue;
                    };
                    let mut tile_data = Vec::with_capacity((grid.chunk_size as usize).pow(2));
                    for x in 0..grid.chunk_size {
                        for y in 0..grid.chunk_size {
                            tile_data.push(chunk.get_tile(x, y));
                        }
                    }
                    chunk_data.push(ChunkDatum::create_modified(chunk_index, tile_data));
                }
            }

            grid_data.insert(
                grid.index,
                GridDatum {
                    coordinates: sekai::MapCoordinates::new(
                        grid.world_position,
                        grid.parent_map_id,
                    ),
                    angle: grid.world_rotation,
                    chunk_data,
                },
            );
        }
        for map_id in maps.get_all_map_ids() {
            deleted_grids.extend(maps.get_deleted_grids_since(map_id, from_tick));
        }
        if grid_data.is_empty() && deleted_grids.is_empty() {
            None
        } else {
            Some(GameStateMapData {
                grid_data,
                deleted_grids,
            })
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
    use crate::{PlayerManager, ServerEntityManager};
    use jikan::GameTick;
    use sekai::{GridId, MapId, MapManager};

    #[test]
    fn server_game_state_manager_builds_updates_for_in_game_sessions() {
        let mut states = ServerGameStateManager::new();
        states.initialize();
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
        let uid = entities.create_entity(Some("mob"));
        entities.initialize_entity(uid);

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

        entities.set_current_tick(GameTick::new(12));
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
    fn server_game_state_manager_sends_full_grid_data_for_newly_visible_grids() {
        let mut states = ServerGameStateManager::new();
        states.initialize();
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

        let controlled = entities.create_entity(Some("mob"));
        entities.initialize_entity(controlled);
        {
            let transform = entities.inner.transforms.get_mut(&controlled).unwrap();
            transform.map_id = map_id;
            transform.rebuild_for_manager();
        }
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
            let transform = entities.inner.transforms.get_mut(&far_uid).unwrap();
            transform.local_position = keisan::Vector2::new(100.0, 0.0);
            transform.rebuild_for_manager();
        }
        entities
            .inner
            .map_grids
            .get_mut(&far_uid)
            .unwrap()
            .world_position = keisan::Vector2::new(100.0, 0.0);
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
        {
            let transform = entities.inner.transforms.get_mut(&far_uid).unwrap();
            transform.local_position = keisan::Vector2::new(1.0, 0.0);
            transform.rebuild_for_manager();
        }
        entities
            .inner
            .map_grids
            .get_mut(&far_uid)
            .unwrap()
            .world_position = keisan::Vector2::new(1.0, 0.0);

        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(3));
        let map_data = updates[0].1.map_data.as_ref().unwrap();
        assert!(map_data.grid_data.contains_key(&far_grid));
        assert!(!map_data.grid_data[&far_grid].chunk_data.is_empty());
    }

    #[test]
    fn server_game_state_manager_preserves_real_chunk_indices_in_map_data() {
        let mut states = ServerGameStateManager::new();
        states.initialize();
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
        let controlled = entities.create_entity(Some("mob"));
        entities.initialize_entity(controlled);
        {
            let transform = entities.inner.transforms.get_mut(&controlled).unwrap();
            transform.map_id = map_id;
            transform.rebuild_for_manager();
        }
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
        states.initialize();
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

        let controlled = entities.create_entity(Some("mob"));
        entities.initialize_entity(controlled);
        {
            let transform = entities.inner.transforms.get_mut(&controlled).unwrap();
            transform.map_id = map_id;
            transform.rebuild_for_manager();
        }
        assert!(players.set_attached_entity("u1", Some(controlled)));

        let near_grid = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(9)), 8);
        let far_grid = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(10)), 8);
        let far_uid = maps.get_grid_euid(far_grid).unwrap();
        {
            let transform = entities.inner.transforms.get_mut(&far_uid).unwrap();
            transform.local_position = keisan::Vector2::new(100.0, 0.0);
            transform.rebuild_for_manager();
        }
        entities
            .inner
            .map_grids
            .get_mut(&far_uid)
            .unwrap()
            .world_position = keisan::Vector2::new(100.0, 0.0);

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
        states.initialize();
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
        states.initialize();
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
        let controlled = entities.create_entity(Some("mob"));
        entities.initialize_entity(controlled);
        {
            let transform = entities.inner.transforms.get_mut(&controlled).unwrap();
            transform.map_id = map_id;
            transform.rebuild_for_manager();
        }
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
        states.initialize();
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
        let controlled = entities.create_entity(Some("mob"));
        entities.initialize_entity(controlled);
        {
            let transform = entities.inner.transforms.get_mut(&controlled).unwrap();
            transform.map_id = map_id;
            transform.rebuild_for_manager();
        }
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
        states.initialize();
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
        states.initialize();
        states.handle_client_connected("u1");
        states.pvs.view_size = 8.0;

        let mut players = PlayerManager::new(4);
        players.set_current_tick(GameTick::new(1));
        players.connect("u1", "pedel");
        assert!(players.join_game("u1"));

        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);

        let controlled = entities.create_entity(Some("mob"));
        entities.initialize_entity(controlled);
        assert!(players.set_attached_entity("u1", Some(controlled)));

        let other = entities.create_entity(Some("crate"));
        entities.initialize_entity(other);
        {
            let transform = entities.inner.transforms.get_mut(&other).unwrap();
            transform.local_position = keisan::Vector2::new(100.0, 0.0);
            transform.rebuild_for_manager();
        }

        let updates =
            states.send_game_state_update(&mut entities, &mut maps, &mut players, GameTick::new(1));
        assert_eq!(updates[0].1.entity_states.len(), 1);
        assert_eq!(updates[0].1.entity_states[0].uid, controlled);

        states.ack("u1", GameTick::new(1));
        players.set_current_tick(GameTick::new(2));
        {
            let transform = entities.inner.transforms.get_mut(&other).unwrap();
            transform.local_position = keisan::Vector2::new(1.0, 0.0);
            transform.rebuild_for_manager();
        }

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
}
