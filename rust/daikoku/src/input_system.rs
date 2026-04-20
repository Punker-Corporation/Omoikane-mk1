use jikan::GameTick;
use sekai::{EntityCoordinates, ScreenCoordinates};
use std::collections::HashMap;

use crate::PlayerManager;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoundKeyFunction {
    pub function_name: String,
}

impl BoundKeyFunction {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            function_name: name.into(),
        }
    }
}

impl From<&str> for BoundKeyFunction {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for BoundKeyFunction {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundKeyState {
    Up = 0,
    Down = 1,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerCommandStates {
    function_states: HashMap<BoundKeyFunction, BoundKeyState>,
}

impl PlayerCommandStates {
    pub fn get_state(&self, function: &BoundKeyFunction) -> BoundKeyState {
        self.function_states.get(function).copied().unwrap_or(BoundKeyState::Up)
    }

    pub fn set_state(&mut self, function: BoundKeyFunction, state: BoundKeyState) {
        self.function_states.insert(function, state);
    }

    pub fn is_down(&self, function: &BoundKeyFunction) -> bool {
        self.get_state(function) == BoundKeyState::Down
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FullInputCmdMessage {
    pub tick: GameTick,
    pub sub_tick: u16,
    pub input_sequence: u32,
    pub input_function_id: BoundKeyFunction,
    pub state: BoundKeyState,
    pub coordinates: EntityCoordinates,
    pub screen_coordinates: ScreenCoordinates,
}

impl FullInputCmdMessage {
    pub fn new(
        tick: GameTick,
        sub_tick: u16,
        input_sequence: u32,
        input_function_id: impl Into<BoundKeyFunction>,
        state: BoundKeyState,
        coordinates: EntityCoordinates,
        screen_coordinates: ScreenCoordinates,
    ) -> Self {
        Self {
            tick,
            sub_tick,
            input_sequence,
            input_function_id: input_function_id.into(),
            state,
            coordinates,
            screen_coordinates,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct InputSystem {
    player_inputs: HashMap<String, PlayerCommandStates>,
    last_processed_input_cmd: HashMap<String, u32>,
}

impl InputSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn initialize(&mut self) {}

    pub fn handle_player_connected(&mut self, user_id: impl Into<String>) {
        let user_id = user_id.into();
        self.player_inputs.entry(user_id.clone()).or_default();
        self.last_processed_input_cmd.entry(user_id).or_insert(0);
    }

    pub fn handle_player_disconnected(&mut self, user_id: &str) {
        self.player_inputs.remove(user_id);
        self.last_processed_input_cmd.remove(user_id);
    }

    pub fn handle_input(
        &mut self,
        players: &mut PlayerManager,
        user_id: &str,
        message: FullInputCmdMessage,
    ) -> bool {
        if message.input_function_id.function_name.is_empty() {
            return false;
        }

        if players.get_session(user_id).is_none() {
            return false;
        }

        let last_processed = self
            .last_processed_input_cmd
            .entry(user_id.to_string())
            .or_insert(0);

        if *last_processed < message.input_sequence {
            *last_processed = message.input_sequence;
        }

        self.player_inputs
            .entry(user_id.to_string())
            .or_default()
            .set_state(message.input_function_id, message.state);

        if let Some(session) = players.get_session_mut(user_id) {
            session.last_processed_input = *last_processed;
        }

        true
    }

    pub fn get_input_states(&self, user_id: &str) -> Option<&PlayerCommandStates> {
        self.player_inputs.get(user_id)
    }

    pub fn get_last_input_command(&self, user_id: &str) -> Option<u32> {
        self.last_processed_input_cmd.get(user_id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::{BoundKeyState, FullInputCmdMessage, InputSystem};
    use crate::PlayerManager;
    use jikan::GameTick;
    use keisan::Vector2;
    use sekai::{EntityCoordinates, EntityUid, ScreenCoordinates, WindowId};

    #[test]
    fn input_system_tracks_player_command_state_and_sequence() {
        let mut players = PlayerManager::new(4);
        players.connect("u1", "pedel");
        let mut system = InputSystem::new();
        system.handle_player_connected("u1");

        let message = FullInputCmdMessage::new(
            GameTick::new(5),
            0,
            9,
            "MoveUp",
            BoundKeyState::Down,
            EntityCoordinates::new(EntityUid::new(7), Vector2::new(2.0, 3.0)),
            ScreenCoordinates::new_xy(10.0, 12.0, WindowId::MAIN),
        );

        assert!(system.handle_input(&mut players, "u1", message));
        assert_eq!(system.get_last_input_command("u1"), Some(9));
        assert!(system
            .get_input_states("u1")
            .unwrap()
            .is_down(&"MoveUp".into()));
        assert_eq!(players.get_session("u1").unwrap().last_processed_input, 9);
    }
}
