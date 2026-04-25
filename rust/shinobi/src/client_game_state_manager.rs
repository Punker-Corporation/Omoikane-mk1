use crate::{ClientEntityManager, ClientGameStateProcessor, ClientNetManager, PlayerManager};
use daikoku::{FullInputCmdMessage, MsgState, MsgStateAck};
use jikan::GameTick;
use sekai::{EntityUid, GameState, RobustSerializer};

#[derive(Debug, Clone, PartialEq)]
pub struct GameStateAppliedArgs {
    pub applied_state: GameState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameStateRuntimeApplyContext {
    pub pending_inputs: Vec<FullInputCmdMessage>,
    pub local_controlled: Option<EntityUid>,
    pub pending_for_local: bool,
}

#[derive(Debug, Clone)]
pub struct ClientGameStateManager {
    processor: ClientGameStateProcessor,
    serializer: RobustSerializer,
    next_input_cmd_seq: u32,
    pending_inputs: Vec<FullInputCmdMessage>,
    pub applied_states: Vec<GameStateAppliedArgs>,
    pub last_processed_seq: u32,
    pub last_processed_tick: GameTick,
}

impl Default for ClientGameStateManager {
    fn default() -> Self {
        Self {
            processor: ClientGameStateProcessor::new(),
            serializer: RobustSerializer::new(),
            next_input_cmd_seq: 1,
            pending_inputs: Vec::new(),
            applied_states: Vec::new(),
            last_processed_seq: 0,
            last_processed_tick: GameTick::ZERO,
        }
    }
}

impl ClientGameStateManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.processor.reset();
        self.pending_inputs.clear();
        self.applied_states.clear();
        self.last_processed_seq = 0;
        self.last_processed_tick = GameTick::ZERO;
        self.next_input_cmd_seq = 1;
    }

    pub fn input_command_dispatched(
        &mut self,
        net: &mut ClientNetManager,
        mut message: FullInputCmdMessage,
    ) -> u32 {
        message.input_sequence = self.next_input_cmd_seq;
        self.next_input_cmd_seq += 1;
        net.dispatch_input(message.clone());
        self.pending_inputs.push(message);
        self.next_input_cmd_seq - 1
    }

    pub fn handle_state_message(&mut self, net: &mut ClientNetManager, message: MsgState) {
        let sequence = message.state.to_sequence;
        self.processor.add_new_state(message.state);
        net.send_ack(MsgStateAck { sequence });
    }

    pub fn pump_network(&mut self, net: &mut ClientNetManager) {
        while let Some(state) = net.next_state() {
            self.handle_state_message(net, state);
        }
    }

    pub fn apply_next_state(
        &mut self,
        entities: &mut ClientEntityManager,
        players: &mut PlayerManager,
    ) -> Option<GameState> {
        let state = self.processor.pop_next_state()?;
        self.apply_state(entities, players, &state);
        Some(state)
    }

    pub fn apply_next_state_with_runtime_context(
        &mut self,
        entities: &mut ClientEntityManager,
        players: &mut PlayerManager,
    ) -> Option<(GameState, GameStateRuntimeApplyContext)> {
        let state = self.processor.pop_next_state()?;
        self.apply_state(entities, players, &state);
        let context = self.runtime_apply_context(players);
        Some((state, context))
    }

    pub fn pending_inputs(&self) -> &[FullInputCmdMessage] {
        &self.pending_inputs
    }

    pub fn pending_inputs_snapshot(&self) -> Vec<FullInputCmdMessage> {
        self.pending_inputs.clone()
    }

    pub fn has_pending_input_for(&self, entity: EntityUid) -> bool {
        self.pending_inputs
            .iter()
            .any(|input| input.coordinates.entity_id == entity)
    }

    pub fn runtime_apply_context(&self, players: &PlayerManager) -> GameStateRuntimeApplyContext {
        let pending_inputs = self.pending_inputs_snapshot();
        let local_controlled = players.controlled_entity();
        let pending_for_local =
            local_controlled.is_some_and(|controlled| self.has_pending_input_for(controlled));
        GameStateRuntimeApplyContext {
            pending_inputs,
            local_controlled,
            pending_for_local,
        }
    }

    pub fn applied_entity_exists(
        &self,
        entities: &ClientEntityManager,
        uid: EntityUid,
    ) -> bool {
        entities.entity_exists(uid)
    }

    fn apply_state(
        &mut self,
        entities: &mut ClientEntityManager,
        players: &mut PlayerManager,
        state: &GameState,
    ) {
        let _created_entities = entities.apply_game_state(&mut self.serializer, state);
        players.apply_player_states(&state.player_states, state.from_sequence == GameTick::ZERO);
        self.last_processed_tick = state.to_sequence;
        self.last_processed_seq = self.last_processed_seq.max(state.last_processed_input);
        self.pending_inputs
            .retain(|input| input.input_sequence > self.last_processed_seq);
        self.applied_states.push(GameStateAppliedArgs {
            applied_state: state.clone(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{ClientGameStateManager, GameStateRuntimeApplyContext};
    use crate::{ClientEntityManager, ClientNetManager, PlayerManager};
    use daikoku::MsgState;
    use jikan::GameTick;
    use keisan::{Angle, Vector2, Vector2i};
    use sekai::{
        ChunkDatum,
        EntityUid, GameState, MetaDataComponentState, PlayerState, RobustSerializer,
        SerializedComponentChange, SerializedEntityState, SessionStatus, Tile, TileRenderFlag,
        GameStateMapData, GridDatum, GridId, MapCoordinates, MapId,
    };

    #[test]
    fn client_game_state_manager_applies_state_and_acks_it() {
        let mut manager = ClientGameStateManager::new();
        let mut entities = ClientEntityManager::new();
        let mut players = PlayerManager::new();
        players.startup("u1", "pedel");
        let mut net = ClientNetManager::new();
        net.connect();

        let mut serializer = RobustSerializer::new();
        net.receive_state(MsgState::new(GameState {
            from_sequence: GameTick::ZERO,
            to_sequence: GameTick::new(3),
            last_processed_input: 2,
            entity_states: vec![SerializedEntityState {
                uid: EntityUid::new(55),
                component_changes: vec![SerializedComponentChange::new(
                    1,
                    true,
                    false,
                    Some(
                        serializer
                            .serialize_component_state(&MetaDataComponentState {
                                name: Some("mob".to_string()),
                                description: None,
                                prototype_id: Some("mob".to_string()),
                            })
                            .unwrap(),
                    ),
                )],
            }],
            player_states: vec![PlayerState {
                user_id: "u1".to_string(),
                name: "pedel".to_string(),
                status: SessionStatus::InGame,
                ping: 8,
                controlled_entity: Some(EntityUid::new(55)),
            }],
            entity_deletions: Vec::new(),
            map_data: Some(GameStateMapData {
                grid_data: std::iter::once((
                    GridId::new(8),
                    GridDatum {
                        coordinates: MapCoordinates::new(Vector2::new(3.0, 4.0), MapId::new(2)),
                        angle: Angle::ZERO,
                        chunk_data: vec![ChunkDatum::create_modified(
                            Vector2i::new(0, 0),
                            vec![Tile::new(4, TileRenderFlag(0), 0)],
                        )],
                    },
                ))
                .collect(),
                deleted_grids: Vec::new(),
            }),
            extrapolated: false,
            payload_size: 0,
        }));
        net.receive_state(MsgState::new(GameState {
            from_sequence: GameTick::new(3),
            to_sequence: GameTick::new(4),
            last_processed_input: 2,
            entity_states: Vec::new(),
            player_states: vec![PlayerState {
                user_id: "u1".to_string(),
                name: "pedel".to_string(),
                status: SessionStatus::InGame,
                ping: 8,
                controlled_entity: Some(EntityUid::new(55)),
            }],
            entity_deletions: Vec::new(),
            map_data: None,
            extrapolated: false,
            payload_size: 0,
        }));
        net.receive_state(MsgState::new(GameState {
            from_sequence: GameTick::new(4),
            to_sequence: GameTick::new(5),
            last_processed_input: 2,
            entity_states: Vec::new(),
            player_states: vec![PlayerState {
                user_id: "u1".to_string(),
                name: "pedel".to_string(),
                status: SessionStatus::InGame,
                ping: 8,
                controlled_entity: Some(EntityUid::new(55)),
            }],
            entity_deletions: Vec::new(),
            map_data: None,
            extrapolated: false,
            payload_size: 0,
        }));

        manager.pump_network(&mut net);
        assert_eq!(net.take_acks().len(), 3);
        let applied = manager.apply_next_state(&mut entities, &mut players).unwrap();
        assert_eq!(applied.to_sequence, GameTick::new(3));
        assert!(entities.entity_exists(EntityUid::new(55)));
        assert_eq!(players.local_player().unwrap().controlled_entity, Some(EntityUid::new(55)));
        let grid_uid = entities.inner.grid_entity_for(GridId::new(8)).unwrap();
        assert_eq!(
            entities
                .inner
                .map_grids
                .get(&grid_uid)
                .unwrap()
                .get_tile_ref(Vector2i::new(0, 0))
                .tile
                .type_id,
            4
        );
        assert_eq!(
            manager.runtime_apply_context(&players),
            GameStateRuntimeApplyContext {
                pending_inputs: Vec::new(),
                local_controlled: Some(EntityUid::new(55)),
                pending_for_local: false,
            }
        );
    }

    #[test]
    fn client_game_state_manager_applies_incremental_player_deltas_without_pruning() {
        let mut manager = ClientGameStateManager::new();
        let mut entities = ClientEntityManager::new();
        let mut players = PlayerManager::new();
        players.startup("u1", "pedel");
        let mut net = ClientNetManager::new();
        net.connect();

        net.receive_state(MsgState::new(GameState {
            from_sequence: GameTick::ZERO,
            to_sequence: GameTick::new(1),
            last_processed_input: 0,
            entity_states: Vec::new(),
            player_states: vec![
                PlayerState {
                    user_id: "u1".to_string(),
                    name: "pedel".to_string(),
                    status: SessionStatus::InGame,
                    ping: 8,
                    controlled_entity: None,
                },
                PlayerState {
                    user_id: "u2".to_string(),
                    name: "rika".to_string(),
                    status: SessionStatus::Connected,
                    ping: 18,
                    controlled_entity: None,
                },
            ],
            entity_deletions: Vec::new(),
            map_data: None,
            extrapolated: false,
            payload_size: 0,
        }));
        manager.pump_network(&mut net);
        let _ = manager.apply_next_state(&mut entities, &mut players).unwrap();
        assert!(players.session("u2").is_some());

        net.receive_state(MsgState::new(GameState {
            from_sequence: GameTick::new(1),
            to_sequence: GameTick::new(2),
            last_processed_input: 0,
            entity_states: Vec::new(),
            player_states: vec![PlayerState {
                user_id: "u1".to_string(),
                name: "pedel".to_string(),
                status: SessionStatus::InGame,
                ping: 12,
                controlled_entity: None,
            }],
            entity_deletions: Vec::new(),
            map_data: None,
            extrapolated: false,
            payload_size: 0,
        }));
        manager.pump_network(&mut net);
        let _ = manager.apply_next_state(&mut entities, &mut players).unwrap();
        assert_eq!(players.session("u2").unwrap().name, "rika");

        net.receive_state(MsgState::new(GameState {
            from_sequence: GameTick::new(2),
            to_sequence: GameTick::new(3),
            last_processed_input: 0,
            entity_states: Vec::new(),
            player_states: vec![PlayerState {
                user_id: "u2".to_string(),
                name: "rika".to_string(),
                status: SessionStatus::Disconnected,
                ping: 18,
                controlled_entity: None,
            }],
            entity_deletions: Vec::new(),
            map_data: None,
            extrapolated: false,
            payload_size: 0,
        }));
        manager.pump_network(&mut net);
        let _ = manager.apply_next_state(&mut entities, &mut players).unwrap();
        assert!(players.session("u2").is_none());
    }
}
