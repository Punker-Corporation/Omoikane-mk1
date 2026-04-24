use daikoku::{FullInputCmdMessage, MsgEntity, MsgState, MsgStateAck};
use sekai::MsgPlayerList;
use std::collections::VecDeque;

#[derive(Debug, Clone, Default)]
pub struct ClientNetManager {
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
pub struct ClientInboundBatch {
    pub states: Vec<MsgState>,
    pub entities: Vec<MsgEntity>,
    pub player_lists: Vec<MsgPlayerList>,
}

#[derive(Debug, Clone, Default)]
pub struct ClientOutboundBatch {
    pub acks: Vec<MsgStateAck>,
    pub inputs: Vec<FullInputCmdMessage>,
    pub entities: Vec<MsgEntity>,
    pub player_list_requests: usize,
}

impl ClientNetManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn connect(&mut self) {
        self.connected = true;
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
        self.inbound_states.clear();
        self.inbound_entities.clear();
        self.inbound_player_lists.clear();
        self.outbound_acks.clear();
        self.outbound_inputs.clear();
        self.outbound_entities.clear();
        self.outbound_player_list_requests = 0;
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn receive_state(&mut self, state: MsgState) {
        if self.connected {
            self.inbound_states.push_back(state);
        }
    }

    pub fn next_state(&mut self) -> Option<MsgState> {
        self.inbound_states.pop_front()
    }

    pub fn receive_entity(&mut self, message: MsgEntity) {
        if self.connected {
            self.inbound_entities.push_back(message);
        }
    }

    pub fn next_entity(&mut self) -> Option<MsgEntity> {
        self.inbound_entities.pop_front()
    }

    pub fn receive_player_list(&mut self, message: MsgPlayerList) {
        if self.connected {
            self.inbound_player_lists.push_back(message);
        }
    }

    pub fn next_player_list(&mut self) -> Option<MsgPlayerList> {
        self.inbound_player_lists.pop_front()
    }

    pub fn take_inbound_batch(&mut self) -> ClientInboundBatch {
        ClientInboundBatch {
            states: self.inbound_states.drain(..).collect(),
            entities: self.inbound_entities.drain(..).collect(),
            player_lists: self.inbound_player_lists.drain(..).collect(),
        }
    }

    pub fn send_ack(&mut self, ack: MsgStateAck) {
        if self.connected {
            self.outbound_acks.push(ack);
        }
    }

    pub fn dispatch_input(&mut self, input: FullInputCmdMessage) {
        if self.connected {
            self.outbound_inputs.push(input);
        }
    }

    pub fn send_entity(&mut self, message: MsgEntity) {
        if self.connected {
            self.outbound_entities.push(message);
        }
    }

    pub fn request_player_list(&mut self) {
        if self.connected {
            self.outbound_player_list_requests += 1;
        }
    }

    pub fn take_acks(&mut self) -> Vec<MsgStateAck> {
        std::mem::take(&mut self.outbound_acks)
    }

    pub fn take_inputs(&mut self) -> Vec<FullInputCmdMessage> {
        std::mem::take(&mut self.outbound_inputs)
    }

    pub fn take_entities(&mut self) -> Vec<MsgEntity> {
        std::mem::take(&mut self.outbound_entities)
    }

    pub fn take_player_list_requests(&mut self) -> usize {
        std::mem::take(&mut self.outbound_player_list_requests)
    }

    pub fn take_outbound_batch(&mut self) -> ClientOutboundBatch {
        ClientOutboundBatch {
            acks: self.take_acks(),
            inputs: self.take_inputs(),
            entities: self.take_entities(),
            player_list_requests: self.take_player_list_requests(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ClientNetManager;
    use daikoku::{BoundKeyState, EntityMessageType, FullInputCmdMessage, MsgEntity, MsgState, MsgStateAck};
    use jikan::GameTick;
    use keisan::Vector2;
    use sekai::{EntityCoordinates, EntityUid, GameState, PlayerState, ScreenCoordinates, SessionStatus, WindowId};

    #[test]
    fn client_net_manager_buffers_state_inputs_and_acks() {
        let mut net = ClientNetManager::new();
        net.connect();
        net.request_player_list();
        assert_eq!(net.take_player_list_requests(), 1);

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
        assert_eq!(net.take_acks().len(), 1);
        assert_eq!(net.take_inputs().len(), 1);
        net.receive_entity(MsgEntity {
            message_type: EntityMessageType::SystemMessage,
            system_message: Some("ping".to_string()),
            component_message: None,
            entity_uid: EntityUid::new(3),
            net_id: 0,
            sequence: 5,
            source_tick: GameTick::FIRST,
        });
        assert!(net.next_entity().is_some());
        net.receive_player_list(sekai::MsgPlayerList {
            plyrs: vec![PlayerState {
                user_id: "u1".to_string(),
                name: "pedel".to_string(),
                status: SessionStatus::Connected,
                ping: 5,
                controlled_entity: None,
            }],
        });
        assert_eq!(net.next_player_list().unwrap().plyrs.len(), 1);
    }
}
