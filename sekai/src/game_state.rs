use crate::{EntityUid, GridId, MapCoordinates, Tile};
use jikan::GameTick;
use keisan::{Angle, Vector2i};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::entity_state::SerializedEntityState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Connecting,
    Connected,
    InGame,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerState {
    pub user_id: String,
    pub name: String,
    pub status: SessionStatus,
    pub ping: i16,
    pub controlled_entity: Option<EntityUid>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    pub from_sequence: GameTick,
    pub to_sequence: GameTick,
    pub last_processed_input: u32,
    pub entity_states: Vec<SerializedEntityState>,
    pub player_states: Vec<PlayerState>,
    pub entity_deletions: Vec<EntityUid>,
    pub map_data: Option<GameStateMapData>,
    #[serde(skip)]
    pub extrapolated: bool,
    #[serde(skip)]
    pub payload_size: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GameStateMapData {
    pub grid_data: HashMap<GridId, GridDatum>,
    pub deleted_grids: Vec<GridId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridDatum {
    pub coordinates: MapCoordinates,
    pub angle: Angle,
    pub chunk_data: Vec<ChunkDatum>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkDatum {
    pub index: Vector2i,
    pub tile_data: Option<Vec<Tile>>,
}

impl ChunkDatum {
    pub fn create_modified(index: Vector2i, tile_data: Vec<Tile>) -> Self {
        Self {
            index,
            tile_data: Some(tile_data),
        }
    }

    pub fn create_deleted(index: Vector2i) -> Self {
        Self {
            index,
            tile_data: None,
        }
    }

    pub fn is_deleted(&self) -> bool {
        self.tile_data.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::{ChunkDatum, GameState, GameStateMapData, GridDatum, PlayerState, SessionStatus};
    use crate::{EntityUid, GridId, MapCoordinates, MapId, Tile, TileRenderFlag};
    use jikan::GameTick;
    use keisan::{Angle, Vector2, Vector2i};
    use std::collections::HashMap;

    #[test]
    fn game_state_holds_players_entities_and_map_data() {
        let mut grids = HashMap::new();
        grids.insert(
            GridId::new(5),
            GridDatum {
                coordinates: MapCoordinates::new(Vector2::new(1.0, 2.0), MapId::new(3)),
                angle: Angle::from_degrees(90.0),
                chunk_data: vec![ChunkDatum::create_modified(
                    Vector2i::new(0, 0),
                    vec![Tile::new(1, TileRenderFlag(0), 0)],
                )],
            },
        );

        let state = GameState {
            from_sequence: GameTick::new(1),
            to_sequence: GameTick::new(2),
            last_processed_input: 7,
            entity_states: Vec::new(),
            player_states: vec![PlayerState {
                user_id: "u1".to_string(),
                name: "pedel".to_string(),
                status: SessionStatus::InGame,
                ping: 12,
                controlled_entity: Some(EntityUid::new(9)),
            }],
            entity_deletions: vec![EntityUid::new(11)],
            map_data: Some(GameStateMapData {
                grid_data: grids,
                deleted_grids: vec![GridId::new(7)],
            }),
            extrapolated: false,
            payload_size: 0,
        };

        assert_eq!(state.player_states.len(), 1);
        assert_eq!(state.entity_deletions[0], EntityUid::new(11));
        let map_data = state.map_data.unwrap();
        assert!(map_data.grid_data.contains_key(&GridId::new(5)));
        assert_eq!(map_data.deleted_grids, vec![GridId::new(7)]);
    }
}
