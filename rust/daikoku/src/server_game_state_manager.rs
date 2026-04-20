use crate::{PlayerManager, PvsSystem, ServerEntityManager};
use jikan::GameTick;
use sekai::{
    ChunkDatum, GameState, GameStateMapData, GridDatum, RobustSerializer, SerializedEntityState,
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
        let last = self.acked_states.entry(user_id.to_string()).or_insert(GameTick::ZERO);
        if state_acked > *last || state_acked == GameTick::ZERO {
            *last = state_acked;
        }
    }

    pub fn send_game_state_update(
        &mut self,
        entities: &mut ServerEntityManager,
        players: &PlayerManager,
        cur_tick: GameTick,
    ) -> Vec<(String, GameState)> {
        let mut updates = Vec::new();
        let mut oldest_ack = GameTick::MAX_VALUE;

        for session in players.in_game_sessions() {
            let last_ack = *self
                .acked_states
                .get(&session.user_id)
                .unwrap_or(&GameTick::ZERO);
            let (visible, deletions) = self.pvs.calculate_visible_entities(entities, session);
            let entity_states = self.collect_entity_states(entities, &visible);
            let state = GameState {
                from_sequence: last_ack,
                to_sequence: cur_tick,
                last_processed_input: session.last_processed_input,
                entity_states,
                player_states: players.get_player_states(),
                entity_deletions: deletions,
                map_data: Some(self.collect_map_data(entities)),
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
        }

        updates
    }

    fn collect_entity_states(&mut self, entities: &mut ServerEntityManager, ids: &[sekai::EntityUid]) -> Vec<SerializedEntityState> {
        ids.iter()
            .copied()
            .filter_map(|uid| entities.build_entity_state(&mut self.serializer, uid))
            .collect()
    }

    fn collect_map_data(&self, entities: &ServerEntityManager) -> GameStateMapData {
        let mut grid_data = HashMap::new();
        for grid in entities.inner.map_grids.values() {
            let chunk_data = grid
                .get_all_tiles(false)
                .chunks((grid.chunk_size as usize).max(1))
                .enumerate()
                .map(|(index, tiles)| {
                    let tile_data = tiles.iter().map(|tile| tile.tile).collect();
                    ChunkDatum::create_modified(keisan::Vector2i::new(index as i32, 0), tile_data)
                })
                .collect();

            grid_data.insert(
                grid.index,
                GridDatum {
                    coordinates: sekai::MapCoordinates::new(grid.world_position, grid.parent_map_id),
                    angle: grid.world_rotation,
                    chunk_data,
                },
            );
        }
        GameStateMapData { grid_data }
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
    use sekai::SessionStatus;

    #[test]
    fn server_game_state_manager_builds_updates_for_in_game_sessions() {
        let mut states = ServerGameStateManager::new();
        states.initialize();
        states.handle_client_connected("u1");

        let mut players = PlayerManager::new(4);
        players.connect("u1", "pedel");
        players.get_session_mut("u1").unwrap().status = SessionStatus::InGame;

        let mut entities = ServerEntityManager::new();
        let uid = entities.create_entity(Some("mob"));
        entities.initialize_entity(uid);

        let updates = states.send_game_state_update(&mut entities, &players, GameTick::new(10));
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].1.to_sequence, GameTick::new(10));
    }
}
