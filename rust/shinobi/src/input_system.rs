use butsuri::BodyType;
use daikoku::{BoundKeyFunction, BoundKeyState, FullInputCmdMessage, PlayerCommandStates};
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
        let Some(controlled) = players.controlled_entity() else {
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
        let delta = Self::movement_delta(&input_cmd.input_function_id, input_cmd.state);

        if delta == Vector2::ZERO {
            return;
        }

        let _ =
            entities
                .inner
                .offset_local_transform_immediate(controlled, delta, keisan::Angle::ZERO);
    }

    pub fn apply_held_movement_state(
        &self,
        entities: &mut ClientEntityManager,
        players: &PlayerManager,
    ) -> bool {
        let Some(controlled) = players.controlled_entity() else {
            return false;
        };

        let velocity = Self::desired_velocity(&self.cmd_states);
        if velocity == Vector2::ZERO {
            return false;
        }

        self.sync_predicted_body_velocity(entities, controlled, velocity);
        true
    }

    fn movement_delta(function: &BoundKeyFunction, state: BoundKeyState) -> Vector2 {
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
        let has_external_dynamics = entities
            .inner
            .map_gravity(entities.inner.map_id_for(controlled))
            .is_some_and(|gravity| gravity != Vector2::ZERO)
            || entities
                .inner
                .physics
                .get(&controlled)
                .is_some_and(|body| body.force != Vector2::ZERO || body.torque != 0.0);
        let _ = entities
            .inner
            .mutate_physics_and_reconcile(controlled, |body| {
                body.can_collide = true;
                body.set_body_type(BodyType::Dynamic);
                body.predict = true;
                body.linear_velocity = velocity;
                body.set_awake(velocity != Vector2::ZERO || has_external_dynamics);
            });
    }

    fn desired_velocity(states: &PlayerCommandStates) -> Vector2 {
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
    use crate::{
        ClientEntityManager, ClientGameStateManager, ClientNetManager, PhysicsSystem, PlayerManager,
    };
    use daikoku::{BoundKeyState, FullInputCmdMessage};
    use jikan::GameTick;
    use keisan::{Angle, Box2, Vector2, Vector2i};
    use sekai::{
        ChunkDatum, EntityCoordinates, EntityUid, GameStateMapData, GridDatum, GridId,
        MapCoordinates, MapId, ScreenCoordinates, Tile, TileRenderFlag, WindowId,
    };

    #[test]
    fn input_system_tracks_local_state_and_prediction() {
        let mut entities = ClientEntityManager::new();
        let mut players = PlayerManager::new();
        players.startup("u1", "pedel");
        let uid = entities.create_entity(None, EntityUid::new(4));
        entities.inner.initialize_entity(uid);
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
        input.handle_input_command(
            &mut entities,
            &players,
            &mut state,
            &mut net,
            "MoveUp",
            msg.clone(),
            false,
        );
        assert!(input.cmd_states.is_down(&"MoveUp".into()));
        input.predict_input_command(&mut entities, &players, &msg);
        assert!(input.cmd_states.is_down(&"MoveUp".into()));
        assert_eq!(
            entities.inner.transforms.get(&uid).unwrap().local_position,
            Vector2::new(0.0, 2.0)
        );
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
        entities.inner.initialize_entity(uid);
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
        entities.inner.initialize_entity(uid);
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
        let _ = entities.inner.insert_fixture_and_reconcile(
            uid,
            butsuri::Fixture::new(
                "main",
                butsuri::PhysShape::Aabb(butsuri::AabbShape::new(
                    keisan::Box2::new(-0.5, -0.5, 0.5, 0.5),
                    0.0,
                )),
            ),
        );

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
        assert!(entities.inner.map_contains_body(sekai::MapId::new(4), uid));
        assert!(
            entities
                .inner
                .map_contains_awake_body(sekai::MapId::new(4), uid)
        );

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
        assert!(entities.inner.map_contains_body(sekai::MapId::new(4), uid));
        assert!(
            !entities
                .inner
                .map_contains_awake_body(sekai::MapId::new(4), uid)
        );
    }

    #[test]
    fn input_system_creates_predicted_map_runtime_on_demand() {
        let mut entities = ClientEntityManager::new();
        let mut players = PlayerManager::new();
        players.startup("u1", "pedel");
        let _map_owner = entities.ensure_map_entity(sekai::MapId::new(5));
        assert!(!entities.inner.has_map_broadphase(sekai::MapId::new(5)));
        assert!(!entities.inner.has_map_physics_runtime(sekai::MapId::new(5)));

        let uid = entities.create_entity(None, EntityUid::new(15));
        entities.inner.initialize_entity(uid);
        players.local_player_mut().unwrap().attach_entity(uid);
        let _ = entities.inner.apply_transform_state(
            uid,
            sekai::TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: keisan::Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: sekai::MapId::new(5),
                grid_id: sekai::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let _ = entities.inner.insert_fixture_and_reconcile(
            uid,
            butsuri::Fixture::new(
                "main",
                butsuri::PhysShape::Aabb(butsuri::AabbShape::new(
                    keisan::Box2::new(-0.5, -0.5, 0.5, 0.5),
                    0.0,
                )),
            ),
        );

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

        assert!(entities.inner.has_map_broadphase(sekai::MapId::new(5)));
        assert!(entities.inner.has_map_physics_runtime(sekai::MapId::new(5)));
        assert!(entities.inner.map_contains_body(sekai::MapId::new(5), uid));
        assert!(
            entities
                .inner
                .map_contains_awake_body(sekai::MapId::new(5), uid)
        );
    }

    #[test]
    fn input_system_updates_lookup_and_broadphase_immediately_after_predicted_move() {
        let mut entities = ClientEntityManager::new();
        entities.inner.apply_game_state_map_data(&GameStateMapData {
            grid_data: std::iter::once((
                GridId::new(6),
                GridDatum {
                    coordinates: MapCoordinates::new(Vector2::ZERO, MapId::new(6)),
                    angle: Angle::ZERO,
                    chunk_data: vec![ChunkDatum::create_modified(
                        Vector2i::new(0, 0),
                        vec![Tile::new(1, TileRenderFlag(0), 0)],
                    )],
                },
            ))
            .collect(),
            deleted_grids: Vec::new(),
        });

        let mut players = PlayerManager::new();
        players.startup("u1", "pedel");
        let uid = entities.create_entity(None, EntityUid::new(24));
        entities.inner.initialize_entity(uid);
        players.local_player_mut().unwrap().attach_entity(uid);
        let grid_uid = entities.inner.grid_entity_for(GridId::new(6)).unwrap();
        let _ = entities.inner.apply_transform_state(
            uid,
            sekai::TransformComponentState {
                local_position: Vector2::new(0.5, 0.5),
                rotation: Angle::ZERO,
                parent_id: grid_uid,
                map_id: MapId::new(6),
                grid_id: GridId::new(6),
                no_local_rotation: false,
                anchored: false,
            },
        );
        let _ = entities.inner.insert_fixture_and_reconcile(
            uid,
            butsuri::Fixture::new(
                "main",
                butsuri::PhysShape::Aabb(butsuri::AabbShape::new(
                    Box2::new(-0.5, -0.5, 0.5, 0.5),
                    0.0,
                )),
            ),
        );
        entities.inner.rebuild_runtime_state();

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

        assert_eq!(
            entities
                .inner
                .entities_at_tile(GridId::new(6), Vector2i::new(1, 0)),
            vec![uid]
        );
        let _physics = PhysicsSystem::new();
        assert_eq!(
            entities
                .inner
                .entities_in_map_aabb(MapId::new(6), Box2::new(1.0, -0.5, 2.5, 1.5)),
            vec![uid]
        );
    }

    #[test]
    fn input_system_updates_contacts_immediately_after_predicted_move() {
        let mut entities = ClientEntityManager::new();
        entities.inner.apply_game_state_map_data(&GameStateMapData {
            grid_data: std::iter::once((
                GridId::new(7),
                GridDatum {
                    coordinates: MapCoordinates::new(Vector2::ZERO, MapId::new(7)),
                    angle: Angle::ZERO,
                    chunk_data: Vec::new(),
                },
            ))
            .collect(),
            deleted_grids: Vec::new(),
        });

        let mut players = PlayerManager::new();
        players.startup("u1", "pedel");
        let first = entities.create_entity(None, EntityUid::new(25));
        entities.inner.initialize_entity(first);
        players.local_player_mut().unwrap().attach_entity(first);
        let _ = entities.inner.apply_transform_state(
            first,
            sekai::TransformComponentState {
                local_position: Vector2::new(0.0, 0.0),
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(7),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let _ = entities.inner.insert_fixture_and_reconcile(
            first,
            butsuri::Fixture::new(
                "first",
                butsuri::PhysShape::Aabb(butsuri::AabbShape::new(
                    Box2::new(-0.5, -0.5, 0.5, 0.5),
                    0.0,
                )),
            ),
        );

        let second = entities.create_entity(None, EntityUid::new(26));
        entities.inner.initialize_entity(second);
        let _ = entities.inner.apply_transform_state(
            second,
            sekai::TransformComponentState {
                local_position: Vector2::new(1.5, 0.0),
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(7),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(second);
        body.can_collide = true;
        body.set_body_type(butsuri::BodyType::Dynamic);
        body.awake = true;
        let _ = entities.inner.insert_fixture_and_reconcile(
            second,
            butsuri::Fixture::new(
                "second",
                butsuri::PhysShape::Aabb(butsuri::AabbShape::new(
                    Box2::new(-0.5, -0.5, 0.5, 0.5),
                    0.0,
                )),
            ),
        );
        entities.inner.rebuild_runtime_state();

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
                EntityCoordinates::new(first, Vector2::ZERO),
                ScreenCoordinates::new_xy(0.0, 0.0, WindowId::MAIN),
            ),
            false,
        );

        assert_eq!(entities.inner.map_contact_count(MapId::new(7)), 1);
        let contacts = entities.inner.map_contacts_snapshot(MapId::new(7));
        assert!(contacts[0].is_touching);
    }
}
