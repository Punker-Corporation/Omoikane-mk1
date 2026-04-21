use daikoku::{BoundKeyFunction, BoundKeyState, FullInputCmdMessage, PlayerCommandStates};
use butsuri::BodyType;
use jikan::GameTick;
use keisan::Vector2;
use sekai::{EntityCoordinates, EntityUid, MapCoordinates, ScreenCoordinates, WindowId};

use crate::{ClientEntityManager, ClientGameStateManager, ClientNetManager, PlayerManager};

#[derive(Debug, Clone, Default)]
pub struct InputSystem {
    cmd_states: PlayerCommandStates,
    pub predicted: bool,
}

impl InputSystem {
    pub const MOVE_STEP: f32 = 1.0;
    pub const MOVE_SPEED: f32 = 62.5;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn cmd_states(&self) -> &PlayerCommandStates {
        &self.cmd_states
    }

    pub fn handle_input_command(
        &mut self,
        entities: &mut ClientEntityManager,
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
        if replay {
            self.predict_input_command(entities, players, &message);
        } else {
            state_manager.input_command_dispatched(net, message.clone());
            self.predict_input_command(entities, players, &message);
        }
        false
    }

    pub fn predict_input_command(
        &mut self,
        entities: &mut ClientEntityManager,
        players: &PlayerManager,
        input_cmd: &FullInputCmdMessage,
    ) {
        self.predicted = true;
        self.cmd_states
            .set_state(input_cmd.input_function_id.clone(), input_cmd.state);
        self.apply_input_effect(entities, players, input_cmd);
        self.predicted = false;
    }

    pub fn replay_pending_inputs(
        &mut self,
        entities: &mut ClientEntityManager,
        players: &PlayerManager,
        pending_inputs: &[FullInputCmdMessage],
    ) {
        for input in pending_inputs {
            self.predict_input_command(entities, players, input);
        }
    }

    fn apply_input_effect(
        &self,
        entities: &mut ClientEntityManager,
        players: &PlayerManager,
        input_cmd: &FullInputCmdMessage,
    ) {
        let Some(local_player) = players.local_player() else {
            return;
        };
        let Some(controlled) = local_player.controlled_entity else {
            return;
        };

        if input_cmd.coordinates.entity_id != controlled {
            return;
        }

        self.sync_predicted_body_velocity(
            entities,
            controlled,
            Self::desired_velocity(&self.cmd_states),
        );
        if input_cmd.state != BoundKeyState::Down {
            return;
        }
        let Some(transform) = entities.inner.transforms.get_mut(&controlled) else {
            return;
        };

        let delta = Self::movement_delta(&input_cmd.input_function_id, input_cmd.state);

        if delta == Vector2::ZERO {
            return;
        }

        transform.local_position = transform.local_position + delta;
        transform.rebuild_for_manager();
    }

    pub fn apply_held_movement_state(
        &self,
        entities: &mut ClientEntityManager,
        players: &PlayerManager,
    ) -> bool {
        let Some(controlled) = players
            .local_player()
            .and_then(|player| player.controlled_entity)
        else {
            return false;
        };

        let velocity = Self::desired_velocity(&self.cmd_states);
        if velocity == Vector2::ZERO && !entities.inner.physics.contains_key(&controlled) {
            return false;
        }

        self.sync_predicted_body_velocity(entities, controlled, velocity);
        true
    }

    pub fn movement_delta(function: &BoundKeyFunction, state: BoundKeyState) -> Vector2 {
        if state != BoundKeyState::Down {
            return Vector2::ZERO;
        }

        match function.function_name.as_str() {
            "MoveUp" => Vector2::new(0.0, Self::MOVE_STEP),
            "MoveDown" => Vector2::new(0.0, -Self::MOVE_STEP),
            "MoveLeft" => Vector2::new(-Self::MOVE_STEP, 0.0),
            "MoveRight" => Vector2::new(Self::MOVE_STEP, 0.0),
            _ => Vector2::ZERO,
        }
    }

    fn sync_predicted_body_velocity(
        &self,
        entities: &mut ClientEntityManager,
        controlled: EntityUid,
        velocity: Vector2,
    ) {
        let body = entities.inner.ensure_physics(controlled);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        body.predict = true;
        body.linear_velocity = velocity;
        body.set_awake(velocity != Vector2::ZERO);
        let map_id = entities
            .inner
            .transforms
            .get(&controlled)
            .map(|transform| transform.map_id)
            .unwrap_or(sekai::MapId::NULLSPACE);
        if map_id != sekai::MapId::NULLSPACE {
            entities.sync_map_physics_runtime(map_id);
        }
    }

    pub fn desired_velocity(states: &PlayerCommandStates) -> Vector2 {
        let mut velocity = Vector2::ZERO;
        if states.is_down(&"MoveUp".into()) {
            velocity.y += Self::MOVE_SPEED;
        }
        if states.is_down(&"MoveDown".into()) {
            velocity.y -= Self::MOVE_SPEED;
        }
        if states.is_down(&"MoveLeft".into()) {
            velocity.x -= Self::MOVE_SPEED;
        }
        if states.is_down(&"MoveRight".into()) {
            velocity.x += Self::MOVE_SPEED;
        }
        velocity
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
    use crate::{ClientEntityManager, ClientGameStateManager, ClientNetManager, PlayerManager};
    use daikoku::{BoundKeyState, FullInputCmdMessage};
    use jikan::GameTick;
    use keisan::Vector2;
    use sekai::{EntityCoordinates, EntityUid, ScreenCoordinates, WindowId};

    #[test]
    fn input_system_tracks_local_state_and_prediction() {
        let mut entities = ClientEntityManager::new();
        let mut players = PlayerManager::new();
        players.startup("u1", "pedel");
        let uid = entities.create_entity(None, EntityUid::new(4));
        entities.initialize_entity(uid);
        players.local_player_mut().unwrap().attach_entity(uid);
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
        input.handle_input_command(&mut entities, &players, &mut state, &mut net, "MoveUp", msg.clone(), false);
        assert!(input.cmd_states().is_down(&"MoveUp".into()));
        input.predict_input_command(&mut entities, &players, &msg);
        assert!(input.cmd_states().is_down(&"MoveUp".into()));
        assert_eq!(entities.inner.transforms.get(&uid).unwrap().local_position, Vector2::new(0.0, 2.0));
        assert_eq!(
            entities.inner.physics.get(&uid).unwrap().linear_velocity,
            Vector2::new(0.0, InputSystem::MOVE_SPEED)
        );
        assert_eq!(
            InputSystem::movement_delta(&"MoveRight".into(), BoundKeyState::Down),
            Vector2::new(1.0, 0.0)
        );
    }

    #[test]
    fn input_system_applies_held_movement_state_after_press() {
        let mut entities = ClientEntityManager::new();
        let mut players = PlayerManager::new();
        players.startup("u1", "pedel");
        let uid = entities.create_entity(None, EntityUid::new(6));
        entities.initialize_entity(uid);
        players.local_player_mut().unwrap().attach_entity(uid);
        let mut input = InputSystem::new();

        input.predict_input_command(
            &mut entities,
            &players,
            &FullInputCmdMessage::new(
                GameTick::FIRST,
                0,
                1,
                "MoveRight",
                BoundKeyState::Down,
                EntityCoordinates::new(uid, Vector2::ZERO),
                ScreenCoordinates::new_xy(0.0, 0.0, WindowId::MAIN),
            ),
        );

        assert!(input.apply_held_movement_state(&mut entities, &players));
        assert_eq!(
            entities.inner.physics.get(&uid).unwrap().linear_velocity,
            Vector2::new(InputSystem::MOVE_SPEED, 0.0)
        );
    }

    #[test]
    fn input_system_keeps_predicted_map_awake_set_in_sync_without_frame_motion() {
        let mut entities = ClientEntityManager::new();
        let mut players = PlayerManager::new();
        players.startup("u1", "pedel");
        let map_owner = entities.ensure_map_entity(sekai::MapId::new(4));
        entities.inner.ensure_broadphase(map_owner);
        entities.inner.ensure_physics_map(map_owner);

        let uid = entities.create_entity(None, EntityUid::new(14));
        entities.initialize_entity(uid);
        players.local_player_mut().unwrap().attach_entity(uid);
        let _ = entities.inner.apply_transform_state(
            uid,
            sekai::TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: keisan::Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: sekai::MapId::new(4),
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        entities.inner.ensure_fixtures(uid).insert_fixture(butsuri::Fixture::new(
            "main",
            butsuri::PhysShape::Aabb(butsuri::AabbShape::new(
                keisan::Box2::new(-0.5, -0.5, 0.5, 0.5),
                0.0,
            )),
        ));

        let mut state = ClientGameStateManager::new();
        let mut net = ClientNetManager::new();
        net.connect();
        let mut input = InputSystem::new();

        input.handle_input_command(
            &mut entities,
            &players,
            &mut state,
            &mut net,
            "MoveRight",
            FullInputCmdMessage::new(
                GameTick::FIRST,
                0,
                1,
                "MoveRight",
                BoundKeyState::Down,
                EntityCoordinates::new(uid, Vector2::ZERO),
                ScreenCoordinates::new_xy(0.0, 0.0, WindowId::MAIN),
            ),
            false,
        );
        let physics_map = entities.inner.physics_maps.get(&map_owner).unwrap();
        assert!(physics_map.bodies.contains(&uid));
        assert!(physics_map.awake_bodies.contains(&uid));

        input.handle_input_command(
            &mut entities,
            &players,
            &mut state,
            &mut net,
            "MoveRight",
            FullInputCmdMessage::new(
                GameTick::FIRST,
                0,
                2,
                "MoveRight",
                BoundKeyState::Up,
                EntityCoordinates::new(uid, Vector2::ZERO),
                ScreenCoordinates::new_xy(0.0, 0.0, WindowId::MAIN),
            ),
            false,
        );
        let physics_map = entities.inner.physics_maps.get(&map_owner).unwrap();
        assert!(physics_map.bodies.contains(&uid));
        assert!(!physics_map.awake_bodies.contains(&uid));
    }
}
