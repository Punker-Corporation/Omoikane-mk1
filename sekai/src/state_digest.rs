use crate::{
    ChunkDatum, GameState, GameStateMapData, GridDatum, PlayerState, SerializableComponentState,
    SerializedComponentChange, SerializedEntityState, SessionStatus, Tile,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

const GAME_STATE_DOMAIN: &[u8] = b"omoikane.game-state.digest.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StateDigest([u8; Self::BYTES]);

impl StateDigest {
    pub const BYTES: usize = 32;

    pub const fn from_bytes(bytes: [u8; Self::BYTES]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; Self::BYTES] {
        &self.0
    }

    pub fn to_hex(self) -> String {
        let mut value = String::with_capacity(Self::BYTES * 2);
        for byte in self.0 {
            use std::fmt::Write as _;
            let _ = write!(value, "{byte:02x}");
        }
        value
    }
}

impl fmt::Display for StateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct StateDigestBuilder {
    hasher: Sha256,
}

impl StateDigestBuilder {
    pub fn new(domain: &'static [u8]) -> Self {
        let mut builder = Self {
            hasher: Sha256::new(),
        };
        builder.write_bytes(domain);
        builder
    }

    pub fn game_state() -> Self {
        Self::new(GAME_STATE_DOMAIN)
    }

    pub fn finish(self) -> StateDigest {
        StateDigest(self.hasher.finalize().into())
    }

    pub fn write_game_state(&mut self, state: &GameState) {
        self.write_field(b"from_sequence");
        self.write_u32(state.from_sequence.value);
        self.write_field(b"to_sequence");
        self.write_u32(state.to_sequence.value);
        self.write_field(b"last_processed_input");
        self.write_u32(state.last_processed_input);

        self.write_field(b"entity_states");
        self.write_entity_states(&state.entity_states);
        self.write_field(b"player_states");
        self.write_player_states(&state.player_states);
        self.write_field(b"entity_deletions");
        self.write_entity_deletions(&state.entity_deletions);
        self.write_field(b"map_data");
        self.write_map_data(state.map_data.as_ref());
    }

    fn write_entity_states(&mut self, states: &[SerializedEntityState]) {
        let mut indexed = states.iter().enumerate().collect::<Vec<_>>();
        indexed.sort_by_key(|(index, state)| (state.uid.raw(), *index));

        self.write_len(indexed.len());
        for (_, state) in indexed {
            self.write_i32(state.uid.raw());
            self.write_component_changes(&state.component_changes);
        }
    }

    fn write_component_changes(&mut self, changes: &[SerializedComponentChange]) {
        let mut indexed = changes.iter().enumerate().collect::<Vec<_>>();
        indexed.sort_by_key(|(index, change)| {
            (
                change.net_id,
                change.created,
                change.deleted,
                change.state.as_ref().map(|state| state.type_name.as_str()),
                *index,
            )
        });

        self.write_len(indexed.len());
        for (_, change) in indexed {
            self.write_u16(change.net_id);
            self.write_bool(change.created);
            self.write_bool(change.deleted);
            self.write_component_state(change.state.as_ref());
        }
    }

    fn write_component_state(&mut self, state: Option<&SerializableComponentState>) {
        self.write_option(state.is_some());
        if let Some(state) = state {
            self.write_str(&state.type_name);
            self.write_bytes(&state.payload);
        }
    }

    fn write_player_states(&mut self, players: &[PlayerState]) {
        let mut indexed = players.iter().enumerate().collect::<Vec<_>>();
        indexed.sort_by_key(|(index, player)| (&player.user_id, &player.name, *index));

        self.write_len(indexed.len());
        for (_, player) in indexed {
            self.write_str(&player.user_id);
            self.write_str(&player.name);
            self.write_session_status(player.status);
            self.write_i16(player.ping);
            self.write_option(player.controlled_entity.is_some());
            if let Some(uid) = player.controlled_entity {
                self.write_i32(uid.raw());
            }
        }
    }

    fn write_entity_deletions(&mut self, deletions: &[crate::EntityUid]) {
        let mut deletions = deletions.to_vec();
        deletions.sort_by_key(|uid| uid.raw());

        self.write_len(deletions.len());
        for uid in deletions {
            self.write_i32(uid.raw());
        }
    }

    fn write_map_data(&mut self, map_data: Option<&GameStateMapData>) {
        self.write_option(map_data.is_some());
        let Some(map_data) = map_data else {
            return;
        };

        let mut grids = map_data.grid_data.iter().collect::<Vec<_>>();
        grids.sort_by_key(|(grid_id, _)| grid_id.raw());
        self.write_len(grids.len());
        for (grid_id, grid) in grids {
            self.write_i32(grid_id.raw());
            self.write_grid_datum(grid);
        }

        let mut deleted_grids = map_data.deleted_grids.clone();
        deleted_grids.sort_by_key(|grid| grid.raw());
        self.write_len(deleted_grids.len());
        for grid in deleted_grids {
            self.write_i32(grid.raw());
        }
    }

    fn write_grid_datum(&mut self, grid: &GridDatum) {
        self.write_f32(grid.coordinates.position.x);
        self.write_f32(grid.coordinates.position.y);
        self.write_i32(grid.coordinates.map_id.raw());
        self.write_f64(grid.angle.theta);
        self.write_chunk_data(&grid.chunk_data);
    }

    fn write_chunk_data(&mut self, chunks: &[ChunkDatum]) {
        let mut indexed = chunks.iter().enumerate().collect::<Vec<_>>();
        indexed.sort_by_key(|(index, chunk)| (chunk.index.x, chunk.index.y, *index));

        self.write_len(indexed.len());
        for (_, chunk) in indexed {
            self.write_i32(chunk.index.x);
            self.write_i32(chunk.index.y);
            self.write_option(chunk.tile_data.is_some());
            if let Some(tiles) = &chunk.tile_data {
                self.write_tiles(tiles);
            }
        }
    }

    fn write_tiles(&mut self, tiles: &[Tile]) {
        self.write_len(tiles.len());
        for tile in tiles {
            self.write_u32(tile.pack());
        }
    }

    fn write_session_status(&mut self, status: SessionStatus) {
        let value = match status {
            SessionStatus::Connecting => 0_u8,
            SessionStatus::Connected => 1,
            SessionStatus::InGame => 2,
            SessionStatus::Disconnected => 3,
        };
        self.write_u8(value);
    }

    fn write_field(&mut self, field: &'static [u8]) {
        self.write_bytes(field);
    }

    fn write_option(&mut self, present: bool) {
        self.write_bool(present);
    }

    fn write_str(&mut self, value: &str) {
        self.write_bytes(value.as_bytes());
    }

    fn write_bytes(&mut self, value: &[u8]) {
        self.write_len(value.len());
        self.hasher.update(value);
    }

    fn write_bool(&mut self, value: bool) {
        self.write_u8(u8::from(value));
    }

    fn write_u8(&mut self, value: u8) {
        self.hasher.update([value]);
    }

    fn write_i16(&mut self, value: i16) {
        self.hasher.update(value.to_le_bytes());
    }

    fn write_u16(&mut self, value: u16) {
        self.hasher.update(value.to_le_bytes());
    }

    fn write_i32(&mut self, value: i32) {
        self.hasher.update(value.to_le_bytes());
    }

    fn write_u32(&mut self, value: u32) {
        self.hasher.update(value.to_le_bytes());
    }

    fn write_u64(&mut self, value: u64) {
        self.hasher.update(value.to_le_bytes());
    }

    fn write_len(&mut self, value: usize) {
        self.write_u64(value as u64);
    }

    fn write_f32(&mut self, value: f32) {
        let value = if value == 0.0 {
            0
        } else if value.is_nan() {
            f32::NAN.to_bits()
        } else {
            value.to_bits()
        };
        self.write_u32(value);
    }

    fn write_f64(&mut self, value: f64) {
        let value = if value == 0.0 {
            0
        } else if value.is_nan() {
            f64::NAN.to_bits()
        } else {
            value.to_bits()
        };
        self.write_u64(value);
    }
}

pub fn digest_game_state(state: &GameState) -> StateDigest {
    let mut builder = StateDigestBuilder::game_state();
    builder.write_game_state(state);
    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::digest_game_state;
    use crate::{
        ChunkDatum, GameState, GameStateMapData, GridDatum, MapCoordinates, MapId, PlayerState,
        SerializableComponentState, SerializedComponentChange, SerializedEntityState,
        SessionStatus, Tile, TileRenderFlag,
    };
    use jikan::GameTick;
    use keisan::{Angle, Vector2, Vector2i};
    use std::collections::HashMap;

    fn sample_state(order: bool) -> GameState {
        let mut grid_data = HashMap::new();
        let first = (
            crate::GridId::new(1),
            GridDatum {
                coordinates: MapCoordinates::new(Vector2::new(1.0, 2.0), MapId::new(9)),
                angle: Angle::from_degrees(90.0),
                chunk_data: vec![ChunkDatum::create_modified(
                    Vector2i::new(0, 0),
                    vec![Tile::new(1, TileRenderFlag(2), 3)],
                )],
            },
        );
        let second = (
            crate::GridId::new(2),
            GridDatum {
                coordinates: MapCoordinates::new(Vector2::new(3.0, 4.0), MapId::new(9)),
                angle: Angle::ZERO,
                chunk_data: vec![ChunkDatum::create_deleted(Vector2i::new(1, 0))],
            },
        );
        if order {
            grid_data.insert(first.0, first.1);
            grid_data.insert(second.0, second.1);
        } else {
            grid_data.insert(second.0, second.1);
            grid_data.insert(first.0, first.1);
        }

        let mut player_states = vec![
            PlayerState {
                user_id: "b".to_string(),
                name: "Beta".to_string(),
                status: SessionStatus::Connected,
                ping: 8,
                controlled_entity: None,
            },
            PlayerState {
                user_id: "a".to_string(),
                name: "Alpha".to_string(),
                status: SessionStatus::InGame,
                ping: 4,
                controlled_entity: Some(crate::EntityUid::new(5)),
            },
        ];
        if !order {
            player_states.reverse();
        }

        GameState {
            from_sequence: GameTick::new(1),
            to_sequence: GameTick::new(2),
            last_processed_input: 44,
            entity_states: vec![SerializedEntityState {
                uid: crate::EntityUid::new(7),
                component_changes: vec![SerializedComponentChange::new(
                    3,
                    true,
                    false,
                    Some(SerializableComponentState::new("Transform", vec![1, 2, 3])),
                )],
            }],
            player_states,
            entity_deletions: if order {
                vec![crate::EntityUid::new(9), crate::EntityUid::new(8)]
            } else {
                vec![crate::EntityUid::new(8), crate::EntityUid::new(9)]
            },
            map_data: Some(GameStateMapData {
                grid_data,
                deleted_grids: if order {
                    vec![crate::GridId::new(4), crate::GridId::new(3)]
                } else {
                    vec![crate::GridId::new(3), crate::GridId::new(4)]
                },
            }),
            extrapolated: order,
            payload_size: if order { 10 } else { 20 },
        }
    }

    #[test]
    fn digest_is_stable_for_canonical_state_ordering() {
        let left = digest_game_state(&sample_state(true));
        let right = digest_game_state(&sample_state(false));

        assert_eq!(left, right);
        assert_eq!(left.to_hex().len(), 64);
    }

    #[test]
    fn digest_changes_when_authoritative_payload_changes() {
        let mut state = sample_state(true);
        let before = digest_game_state(&state);

        state.entity_states[0].component_changes[0]
            .state
            .as_mut()
            .unwrap()
            .payload
            .push(99);

        assert_ne!(before, digest_game_state(&state));
    }
}
