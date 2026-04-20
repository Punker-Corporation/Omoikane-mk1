use daikoku::{BoundKeyFunction, BoundKeyState, FullInputCmdMessage, PlayerCommandStates};
use jikan::GameTick;
use keisan::Vector2;
use sekai::{EntityCoordinates, EntityUid, MapCoordinates, ScreenCoordinates, WindowId};

use crate::{ClientGameStateManager, ClientNetManager, PlayerManager};

#[derive(Debug, Clone, Default)]
pub struct InputSystem {
    cmd_states: PlayerCommandStates,
    pub predicted: bool,
}

impl InputSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cmd_states(&self) -> &PlayerCommandStates {
        &self.cmd_states
    }

    pub fn handle_input_command(
        &mut self,
        players: &PlayerManager,
        state_manager: &mut ClientGameStateManager,
        net: &mut ClientNetManager,
        function: impl Into<BoundKeyFunction>,
        mut message: FullInputCmdMessage,
        replay: bool,
    ) -> bool {
        let function = function.into();
        if !replay {
            if self.cmd_states.get_state(&function) == message.state {
                return false;
            }
            self.cmd_states.set_state(function.clone(), message.state);
        }

        if players.local_player().is_none() {
            return false;
        }

        message.input_function_id = function;
        state_manager.input_command_dispatched(net, message);
        false
    }

    pub fn predict_input_command(&mut self, input_cmd: &FullInputCmdMessage) {
        self.predicted = true;
        self.cmd_states
            .set_state(input_cmd.input_function_id.clone(), input_cmd.state);
        self.predicted = false;
    }

    pub fn clear(&mut self) {
        self.cmd_states = PlayerCommandStates::default();
        self.predicted = false;
    }

    pub fn build_local_input(
        &self,
        tick: GameTick,
        input_sequence: u32,
        function: impl Into<BoundKeyFunction>,
        state: BoundKeyState,
        attached: EntityUid,
        world_offset: Vector2,
    ) -> FullInputCmdMessage {
        let coords = EntityCoordinates::new(attached, world_offset);
        let _map_coords = MapCoordinates::new(world_offset, sekai::MapId::NULLSPACE);
        FullInputCmdMessage::new(
            tick,
            0,
            input_sequence,
            function,
            state,
            coords,
            ScreenCoordinates::new_xy(world_offset.x, world_offset.y, WindowId::MAIN),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::InputSystem;
    use crate::{ClientGameStateManager, ClientNetManager, PlayerManager};
    use daikoku::{BoundKeyState, FullInputCmdMessage};
    use jikan::GameTick;
    use keisan::Vector2;
    use sekai::{EntityCoordinates, EntityUid, ScreenCoordinates, WindowId};

    #[test]
    fn input_system_tracks_local_state_and_prediction() {
        let mut players = PlayerManager::new();
        players.startup("u1", "pedel");
        let mut state = ClientGameStateManager::new();
        let mut net = ClientNetManager::new();
        net.connect();
        let mut input = InputSystem::new();
        let msg = FullInputCmdMessage::new(
            GameTick::FIRST,
            0,
            1,
            "MoveUp",
            BoundKeyState::Down,
            EntityCoordinates::new(EntityUid::new(4), Vector2::ZERO),
            ScreenCoordinates::new_xy(0.0, 0.0, WindowId::MAIN),
        );
        input.handle_input_command(&players, &mut state, &mut net, "MoveUp", msg.clone(), false);
        assert!(input.cmd_states().is_down(&"MoveUp".into()));
        input.predict_input_command(&msg);
        assert!(input.cmd_states().is_down(&"MoveUp".into()));
    }
}
