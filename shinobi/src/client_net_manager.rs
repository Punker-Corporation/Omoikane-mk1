use daikoku::{FullInputCmdMessage, MsgEntity, MsgState, MsgStateAck};
use sekai::MsgPlayerList;
use std::collections::VecDeque;

#[derive(Debug, Clone, Default)]
pub(crate) struct ClientNetManager {
    connected: bool,
    inbound_states: VecDeque<MsgState>,
    inbound_entities: VecDeque<MsgEntity>,
    inbound_player_lists: VecDeque<MsgPlayerList>,
    outbound_acks: Vec<MsgStateAck>,
    outbound_inputs: Vec<FullInputCmdMessage>,
    outbound_entities: Vec<MsgEntity>,
    outbound_player_list_requests: usize,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ClientInboundBatch {
    pub(crate) states: Vec<MsgState>,
    pub(crate) entities: Vec<MsgEntity>,
    pub(crate) player_lists: Vec<MsgPlayerList>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ClientOutboundBatch {
    pub(crate) acks: Vec<MsgStateAck>,
    pub(crate) inputs: Vec<FullInputCmdMessage>,
    pub(crate) entities: Vec<MsgEntity>,
    pub(crate) player_list_requests: usize,
}

impl ClientNetManager {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn connect(&mut self) {
        self.connected = true;
    }

    pub(crate) fn disconnect(&mut self) {
        self.connected = false;
        self.inbound_states.clear();
        self.inbound_entities.clear();
        self.inbound_player_lists.clear();
        self.outbound_acks.clear();
        self.outbound_inputs.clear();
        self.outbound_entities.clear();
        self.outbound_player_list_requests = 0;
    }

    pub(crate) fn receive_state(&mut self, state: MsgState) {
        if self.connected {
            self.inbound_states.push_back(state);
        }
    }

    #[cfg(test)]
    pub(crate) fn next_state(&mut self) -> Option<MsgState> {
        self.inbound_states.pop_front()
    }

    pub(crate) fn receive_entity(&mut self, message: MsgEntity) {
        if self.connected {
            self.inbound_entities.push_back(message);
        }
    }

    pub(crate) fn receive_player_list(&mut self, message: MsgPlayerList) {
        if self.connected {
            self.inbound_player_lists.push_back(message);
        }
    }

    pub(crate) fn take_inbound_batch(&mut self) -> ClientInboundBatch {
        ClientInboundBatch {
            states: self.inbound_states.drain(..).collect(),
            entities: self.inbound_entities.drain(..).collect(),
            player_lists: self.inbound_player_lists.drain(..).collect(),
        }
    }

    pub(crate) fn send_ack(&mut self, ack: MsgStateAck) {
        if self.connected {
            self.outbound_acks.push(ack);
        }
    }

    pub(crate) fn dispatch_input(&mut self, input: FullInputCmdMessage) {
        if self.connected {
            self.outbound_inputs.push(input);
        }
    }

    #[cfg(test)]
    pub(crate) fn send_entity(&mut self, message: MsgEntity) {
        if self.connected {
            self.outbound_entities.push(message);
        }
    }

    pub(crate) fn request_player_list(&mut self) {
        if self.connected {
            self.outbound_player_list_requests += 1;
        }
    }

    pub(crate) fn take_outbound_batch(&mut self) -> ClientOutboundBatch {
        ClientOutboundBatch {
            acks: std::mem::take(&mut self.outbound_acks),
            inputs: std::mem::take(&mut self.outbound_inputs),
            entities: std::mem::take(&mut self.outbound_entities),
            player_list_requests: std::mem::take(&mut self.outbound_player_list_requests),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ClientNetManager;
    use daikoku::{
        BoundKeyState, EntityMessageType, FullInputCmdMessage, MsgEntity, MsgState, MsgStateAck,
    };
    use jikan::GameTick;
    use keisan::Vector2;
    use sekai::{
        EntityCoordinates, EntityUid, GameState, PlayerState, ScreenCoordinates, SessionStatus,
        WindowId,
    };

    #[test]
    fn client_net_manager_buffers_state_inputs_and_acks() {
        let mut net = ClientNetManager::new();
        net.connect();
        net.request_player_list();
        assert_eq!(net.take_outbound_batch().player_list_requests, 1);

        net.receive_state(MsgState::new(GameState {
            from_sequence: GameTick::ZERO,
            to_sequence: GameTick::FIRST,
            last_processed_input: 0,
            entity_states: Vec::new(),
            player_states: Vec::new(),
            entity_deletions: Vec::new(),
            map_data: None,
            extrapolated: false,
            payload_size: 0,
        }));
        assert!(net.next_state().is_some());

        net.send_ack(MsgStateAck {
            sequence: GameTick::FIRST,
        });
        net.dispatch_input(FullInputCmdMessage::new(
            GameTick::FIRST,
            0,
            1,
            "Use",
            BoundKeyState::Down,
            EntityCoordinates::new(EntityUid::new(1), Vector2::ZERO),
            ScreenCoordinates::new_xy(1.0, 1.0, WindowId::MAIN),
        ));
        let outbound = net.take_outbound_batch();
        assert_eq!(outbound.acks.len(), 1);
        assert_eq!(outbound.inputs.len(), 1);
        net.receive_entity(MsgEntity {
            message_type: EntityMessageType::SystemMessage,
            system_message: Some("ping".to_string()),
            component_message: None,
            entity_uid: EntityUid::new(3),
            net_id: 0,
            sequence: 5,
            source_tick: GameTick::FIRST,
        });
        net.receive_player_list(sekai::MsgPlayerList {
            plyrs: vec![PlayerState {
                user_id: "u1".to_string(),
                name: "pedel".to_string(),
                status: SessionStatus::Connected,
                ping: 5,
                controlled_entity: None,
            }],
        });
        let batch = net.take_inbound_batch();
        assert_eq!(batch.entities.len(), 1);
        assert_eq!(batch.player_lists[0].plyrs.len(), 1);
    }
}
