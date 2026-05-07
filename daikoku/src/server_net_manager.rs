use crate::FullInputCmdMessage;
use crate::status_endpoint::ServerQueueStats;
use jikan::GameTick;
use sekai::{EntityUid, GameState, MsgPlayerList};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeliveryMethod {
    Unreliable,
    ReliableUnordered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityMessageType {
    Error,
    ComponentMessage,
    SystemMessage,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MsgState {
    pub state: GameState,
    pub payload_size: usize,
}

impl MsgState {
    #[cfg(test)]
    pub(crate) const RELIABLE_THRESHOLD: usize = 1300;

    pub fn new(state: GameState) -> Self {
        let payload_size = bincode::serialize(&state)
            .map(|bytes| bytes.len())
            .unwrap_or(0);
        Self {
            state,
            payload_size,
        }
    }

    #[cfg(test)]
    pub(crate) fn should_send_reliably(&self) -> bool {
        self.payload_size > Self::RELIABLE_THRESHOLD
    }

    #[cfg(test)]
    pub(crate) fn delivery_method(&self) -> DeliveryMethod {
        if self.should_send_reliably() {
            DeliveryMethod::ReliableUnordered
        } else {
            DeliveryMethod::Unreliable
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MsgStateAck {
    pub sequence: GameTick,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MsgEntity {
    pub message_type: EntityMessageType,
    pub system_message: Option<String>,
    pub component_message: Option<String>,
    pub entity_uid: EntityUid,
    pub net_id: u32,
    pub sequence: u32,
    pub source_tick: GameTick,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OutboundMessage {
    State(MsgState),
    Entity(MsgEntity),
    PlayerList(MsgPlayerList),
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ServerNetManager {
    channels: HashSet<String>,
    outbox: HashMap<String, Vec<OutboundMessage>>,
    inbound_inputs: HashMap<String, Vec<FullInputCmdMessage>>,
    inbound_entities: HashMap<String, Vec<MsgEntity>>,
    inbound_player_list_requests: HashMap<String, usize>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SessionInboundBatch {
    pub(crate) player_list_requests: usize,
    pub(crate) inputs: Vec<FullInputCmdMessage>,
    pub(crate) entities: Vec<MsgEntity>,
}

impl ServerNetManager {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn connect(&mut self, user_id: impl Into<String>) -> bool {
        let user_id = user_id.into();
        let newly_connected = !self.channels.contains(&user_id);
        self.channels.insert(user_id.clone());
        self.outbox.entry(user_id.clone()).or_default();
        self.inbound_inputs.entry(user_id.clone()).or_default();
        self.inbound_entities.entry(user_id.clone()).or_default();
        self.inbound_player_list_requests
            .entry(user_id)
            .or_default();
        newly_connected
    }

    pub(crate) fn disconnect(&mut self, user_id: &str) {
        self.channels.remove(user_id);
        self.outbox.remove(user_id);
        self.inbound_inputs.remove(user_id);
        self.inbound_entities.remove(user_id);
        self.inbound_player_list_requests.remove(user_id);
    }

    fn is_connected(&self, user_id: &str) -> bool {
        self.channels.contains(user_id)
    }

    pub(crate) fn send_state(&mut self, user_id: &str, state: GameState) -> bool {
        if !self.is_connected(user_id) {
            return false;
        }
        self.outbox
            .entry(user_id.to_string())
            .or_default()
            .push(OutboundMessage::State(MsgState::new(state)));
        true
    }

    #[cfg(test)]
    pub(crate) fn send_entity(&mut self, user_id: &str, message: MsgEntity) -> bool {
        if !self.is_connected(user_id) {
            return false;
        }
        self.outbox
            .entry(user_id.to_string())
            .or_default()
            .push(OutboundMessage::Entity(message));
        true
    }

    pub(crate) fn take_outbox(&mut self, user_id: &str) -> Vec<OutboundMessage> {
        self.outbox.remove(user_id).unwrap_or_default()
    }

    pub(crate) fn queue_input(&mut self, user_id: &str, message: FullInputCmdMessage) -> bool {
        if !self.is_connected(user_id) {
            return false;
        }
        self.inbound_inputs
            .entry(user_id.to_string())
            .or_default()
            .push(message);
        true
    }

    pub(crate) fn queue_entity(&mut self, user_id: &str, message: MsgEntity) -> bool {
        if !self.is_connected(user_id) {
            return false;
        }
        self.inbound_entities
            .entry(user_id.to_string())
            .or_default()
            .push(message);
        true
    }

    pub(crate) fn queue_player_list_request(&mut self, user_id: &str) -> bool {
        if !self.is_connected(user_id) {
            return false;
        }
        *self
            .inbound_player_list_requests
            .entry(user_id.to_string())
            .or_default() += 1;
        true
    }

    pub(crate) fn take_session_inbound(&mut self, user_id: &str) -> SessionInboundBatch {
        SessionInboundBatch {
            player_list_requests: self
                .inbound_player_list_requests
                .remove(user_id)
                .unwrap_or_default(),
            inputs: self.inbound_inputs.remove(user_id).unwrap_or_default(),
            entities: self.inbound_entities.remove(user_id).unwrap_or_default(),
        }
    }

    pub(crate) fn send_player_list(&mut self, user_id: &str, list: MsgPlayerList) -> bool {
        if !self.is_connected(user_id) {
            return false;
        }
        self.outbox
            .entry(user_id.to_string())
            .or_default()
            .push(OutboundMessage::PlayerList(list));
        true
    }

    pub(crate) fn queue_stats(&self) -> ServerQueueStats {
        ServerQueueStats {
            sessions: self.channels.len(),
            outbound_messages: self.outbox.values().map(Vec::len).sum(),
            queued_inputs: self.inbound_inputs.values().map(Vec::len).sum(),
            queued_entities: self.inbound_entities.values().map(Vec::len).sum(),
            queued_player_list_requests: self.inbound_player_list_requests.values().sum(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EntityMessageType, MsgEntity, MsgState, OutboundMessage, ServerNetManager};
    use crate::{BoundKeyState, FullInputCmdMessage};
    use jikan::GameTick;
    use keisan::Vector2;
    use sekai::GameState;
    use sekai::{
        EntityCoordinates, EntityUid, MsgPlayerList, PlayerState, ScreenCoordinates, SessionStatus,
        WindowId,
    };

    #[test]
    fn net_manager_tracks_connections_and_outbound_messages() {
        let mut net = ServerNetManager::new();
        assert!(net.connect("u1"));
        let state = GameState {
            from_sequence: GameTick::ZERO,
            to_sequence: GameTick::FIRST,
            last_processed_input: 0,
            entity_states: Vec::new(),
            player_states: Vec::new(),
            entity_deletions: Vec::new(),
            map_data: None,
            extrapolated: false,
            payload_size: 0,
        };
        assert!(net.send_state("u1", state));
        net.send_entity(
            "u1",
            MsgEntity {
                message_type: EntityMessageType::SystemMessage,
                system_message: Some("ping".to_string()),
                component_message: None,
                entity_uid: sekai::EntityUid::INVALID,
                net_id: 0,
                sequence: 0,
                source_tick: GameTick::FIRST,
            },
        );
        let outbox = net.take_outbox("u1");
        assert_eq!(outbox.len(), 2);
        if let super::OutboundMessage::State(message) = &outbox[0] {
            assert_eq!(
                message.delivery_method(),
                MsgState::new(message.state.clone()).delivery_method()
            );
        }

        assert!(net.queue_input(
            "u1",
            FullInputCmdMessage::new(
                GameTick::new(1),
                0,
                4,
                "Use",
                BoundKeyState::Down,
                EntityCoordinates::new(EntityUid::new(5), Vector2::ZERO),
                ScreenCoordinates::new_xy(1.0, 1.0, WindowId::MAIN),
            ),
        ));
        assert!(net.queue_entity(
            "u1",
            MsgEntity {
                message_type: EntityMessageType::ComponentMessage,
                system_message: None,
                component_message: Some("poke".to_string()),
                entity_uid: EntityUid::new(6),
                net_id: 9,
                sequence: 4,
                source_tick: GameTick::FIRST,
            },
        ));
        assert!(net.queue_player_list_request("u1"));
        let inbound = net.take_session_inbound("u1");
        assert_eq!(inbound.inputs.len(), 1);
        assert_eq!(inbound.entities.len(), 1);
        assert_eq!(inbound.player_list_requests, 1);
        assert!(net.send_player_list(
            "u1",
            MsgPlayerList {
                plyrs: vec![PlayerState {
                    user_id: "u1".to_string(),
                    name: "pedel".to_string(),
                    status: SessionStatus::Connected,
                    ping: 1,
                    controlled_entity: None,
                }],
            },
        ));
        assert!(matches!(
            net.take_outbox("u1").pop().unwrap(),
            OutboundMessage::PlayerList(_)
        ));
        assert_eq!(net.queue_stats().sessions, 1);
    }
}
