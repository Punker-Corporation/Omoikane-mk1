use crate::{
    EntityMessageType, FullInputCmdMessage, MsgEntity, OutboundMessage, ServerEntityManager,
    actor_system::ActorSystem, input_system::InputSystem, physics_system::PhysicsSystem,
    player_manager::PlayerManager, server_game_state_manager::ServerGameStateManager,
    server_net_manager::ServerNetManager, transform_system::TransformSystem,
};
use butsuri::{Fixture, Joint};
use jikan::GameTick;
use keisan::{Vector2, Vector2i};
use sekai::{
    BodyStatus, BodyType, EntityUid, GridId, MapId, MapManager, MsgPlayerList, Tile,
    TransformComponentState,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerOptions {
    pub server_name: String,
    pub max_players: usize,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            server_name: "Omoikane".to_string(),
            max_players: 32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerState {
    Stopped,
    Running,
    ShuttingDown,
}

pub struct DaikokuServer {
    pub(crate) options: ServerOptions,
    state: ServerState,
    pub entities: ServerEntityManager,
    pub(crate) players: PlayerManager,
    pub(crate) game_states: ServerGameStateManager,
    pub(crate) network: ServerNetManager,
    pub(crate) actors: ActorSystem,
    pub(crate) input: InputSystem,
    pub(crate) physics: PhysicsSystem,
    pub(crate) transforms: TransformSystem,
    pub maps: MapManager,
    current_tick: GameTick,
    shutdown_reason: Option<String>,
}

impl DaikokuServer {
    pub fn new(options: ServerOptions) -> Self {
        let max_players = options.max_players;
        Self {
            options,
            state: ServerState::Stopped,
            entities: ServerEntityManager::new(),
            players: PlayerManager::new(max_players),
            game_states: ServerGameStateManager::new(),
            network: ServerNetManager::new(),
            actors: ActorSystem::new(),
            input: InputSystem::new(),
            physics: PhysicsSystem::new(),
            transforms: TransformSystem::new(),
            maps: MapManager::new(),
            current_tick: GameTick::ZERO,
            shutdown_reason: None,
        }
    }

    pub fn start(&mut self) {
        self.players.initialize(self.options.max_players);
        self.maps.startup(&mut self.entities.inner);
        self.state = ServerState::Running;
    }

    pub fn shutdown(&mut self, reason: Option<String>) {
        self.shutdown_reason = reason;
        self.state = ServerState::ShuttingDown;
        self.players.shutdown();
        self.maps.shutdown(&mut self.entities.inner);
        self.state = ServerState::Stopped;
    }

    pub fn restart(&mut self) {
        self.shutdown(Some("restart".to_string()));
        self.start();
    }

    pub fn connect_player(
        &mut self,
        user_id: impl Into<String>,
        username: impl Into<String>,
    ) -> bool {
        let user_id = user_id.into();
        let username = username.into();
        let connected = self.players.connect(user_id.clone(), username);
        if connected {
            let _ = self.network.connect(user_id.clone());
            self.input.handle_player_connected(user_id.clone());
            self.game_states.handle_client_connected(user_id);
        }
        connected
    }

    pub fn disconnect_player(&mut self, user_id: &str) {
        let _ = self
            .actors
            .detach_player(&mut self.entities, &mut self.players, user_id);
        self.players.disconnect(user_id);
        self.network.disconnect(user_id);
        self.input.handle_player_disconnected(user_id);
        self.game_states.handle_client_disconnect(user_id);
    }

    pub fn attach_player(&mut self, user_id: &str, entity: sekai::EntityUid, force: bool) -> bool {
        self.actors
            .attach(
                &mut self.entities,
                &mut self.players,
                entity,
                user_id,
                force,
            )
            .result
    }

    pub fn set_current_tick(&mut self, current_tick: GameTick) {
        self.current_tick = current_tick;
        self.entities.inner.current_tick = current_tick;
        self.players.set_current_tick(current_tick);
        self.maps.set_current_tick(current_tick);
    }

    pub fn set_replication_tick(&mut self, current_tick: GameTick) {
        self.players.set_current_tick(current_tick);
        self.maps.set_current_tick(current_tick);
    }

    pub fn join_player(&mut self, user_id: &str) -> bool {
        self.players.join_game(user_id)
    }

    pub fn player_controlled_entity(&self, user_id: &str) -> Option<EntityUid> {
        self.players
            .get_session(user_id)
            .and_then(|session| session.controlled_entity)
    }

    pub fn last_processed_input(&self, user_id: &str) -> Option<u32> {
        self.players
            .get_session(user_id)
            .map(|session| session.last_processed_input)
    }

    pub fn received_system_messages(&self) -> &[(String, String)] {
        &self.entities.received_system_messages
    }

    pub fn received_component_message_count(&self) -> usize {
        self.entities.received_component_messages.len()
    }

    pub fn ack_state(&mut self, user_id: &str, state_acked: GameTick) {
        self.game_states.ack(user_id, state_acked);
    }

    pub fn queue_input(&mut self, user_id: &str, message: FullInputCmdMessage) -> bool {
        self.network.queue_input(user_id, message)
    }

    pub fn queue_entity(&mut self, user_id: &str, message: MsgEntity) -> bool {
        self.network.queue_entity(user_id, message)
    }

    pub fn queue_player_list_request(&mut self, user_id: &str) -> bool {
        self.network.queue_player_list_request(user_id)
    }

    pub fn take_outbox(&mut self, user_id: &str) -> Vec<OutboundMessage> {
        self.network.take_outbox(user_id)
    }

    pub fn remove_component_by_net_id(&mut self, uid: EntityUid, net_id: u16) -> bool {
        self.entities.remove_component_by_net_id(uid, net_id)
    }

    pub fn create_entity_uninitialized(&mut self, prototype: Option<&str>) -> EntityUid {
        self.entities.inner.create_entity_uninitialized(prototype)
    }

    pub fn initialize_entity(&mut self, uid: EntityUid) -> sekai::EntityInitializedMessage {
        self.entities.inner.initialize_entity(uid)
    }

    pub fn apply_transform_state(
        &mut self,
        uid: EntityUid,
        state: TransformComponentState,
    ) -> bool {
        self.entities.inner.apply_transform_state(uid, state)
    }

    pub fn configure_physics_body(
        &mut self,
        uid: EntityUid,
        body_type: Option<BodyType>,
        awake: Option<bool>,
        can_collide: Option<bool>,
        predict: Option<bool>,
    ) -> bool {
        self.entities
            .inner
            .configure_physics_body(uid, body_type, awake, can_collide, predict)
    }

    pub fn insert_fixture(&mut self, uid: EntityUid, fixture: Fixture) -> bool {
        if !self.entities.inner.entity_exists(uid) {
            return false;
        }
        let _ = self
            .entities
            .inner
            .insert_fixture_and_reconcile(uid, fixture);
        true
    }

    pub fn add_joint_between(&mut self, joint: Joint) -> bool {
        self.entities.inner.add_joint_between(joint)
    }

    pub fn create_map(&mut self, map_id: Option<MapId>) -> MapId {
        self.maps.create_map(&mut self.entities.inner, map_id)
    }

    pub fn create_grid(
        &mut self,
        map_id: MapId,
        grid_id: Option<GridId>,
        chunk_size: u16,
    ) -> GridId {
        self.maps
            .create_grid(&mut self.entities.inner, map_id, grid_id, chunk_size)
    }

    pub fn map_entity_for(&self, map_id: MapId) -> Option<EntityUid> {
        self.entities.inner.map_entity_for(map_id)
    }

    pub fn grid_entity_for(&self, grid_id: GridId) -> Option<EntityUid> {
        self.entities.inner.grid_entity_for(grid_id)
    }

    pub fn set_map_paused(&mut self, map_id: MapId, paused: bool) -> bool {
        let Some(map_uid) = self.entities.inner.map_entity_for(map_id) else {
            return false;
        };
        self.entities.inner.set_map_paused(map_uid, paused)
    }

    pub fn has_map_broadphase(&self, map_id: MapId) -> bool {
        self.entities.inner.has_map_broadphase(map_id)
    }

    pub fn map_contains_body(&self, map_id: MapId, uid: EntityUid) -> bool {
        self.entities.inner.map_contains_body(map_id, uid)
    }

    pub fn set_map_tick(&mut self, current_tick: GameTick) {
        self.maps.set_current_tick(current_tick);
    }

    pub fn set_tile(&mut self, grid_id: GridId, indices: Vector2i, tile: Tile) -> bool {
        self.maps
            .set_tile(&mut self.entities.inner, grid_id, indices, tile)
    }

    pub fn remove_chunk(&mut self, grid_id: GridId, chunk: Vector2i) -> bool {
        self.maps
            .remove_chunk(&mut self.entities.inner, grid_id, chunk)
    }

    pub fn set_map_gravity(&mut self, map_id: MapId, gravity: Vector2) -> bool {
        self.physics
            .set_map_gravity(&mut self.entities, &self.maps, map_id, gravity)
    }

    pub fn set_map_auto_clear_forces(&mut self, map_id: MapId, enabled: bool) -> bool {
        self.physics
            .set_auto_clear_forces(&mut self.entities, &self.maps, map_id, enabled)
    }

    pub fn set_body_awake(&mut self, uid: EntityUid, awake: bool) -> bool {
        self.physics.set_awake(&mut self.entities, uid, awake)
    }

    pub fn set_body_sleeping_allowed(&mut self, uid: EntityUid, sleeping_allowed: bool) -> bool {
        self.physics
            .set_sleeping_allowed(&mut self.entities, uid, sleeping_allowed)
    }

    pub fn set_body_fixed_rotation(&mut self, uid: EntityUid, fixed_rotation: bool) -> bool {
        self.physics
            .set_fixed_rotation(&mut self.entities, uid, fixed_rotation)
    }

    pub fn set_body_status(&mut self, uid: EntityUid, status: BodyStatus) -> bool {
        self.physics
            .set_body_status(&mut self.entities, uid, status)
    }

    pub fn set_body_linear_velocity(&mut self, uid: EntityUid, velocity: Vector2) -> bool {
        self.entities
            .inner
            .mutate_physics_and_reconcile(uid, |body| body.linear_velocity = velocity)
    }

    pub fn set_body_angular_velocity(&mut self, uid: EntityUid, velocity: f32) -> bool {
        self.entities
            .inner
            .mutate_physics_and_reconcile(uid, |body| body.angular_velocity = velocity)
    }

    pub fn set_collision_wake_enabled(&mut self, uid: EntityUid, enabled: bool) -> bool {
        let _ = self.entities.inner.ensure_collision_wake(uid);
        if self.entities.inner.set_collision_wake_enabled(uid, enabled) {
            return true;
        }
        self.entities
            .inner
            .collision_wakes
            .get(&uid)
            .is_some_and(|component| component.enabled == enabled)
    }

    #[cfg(test)]
    pub(crate) fn apply_force(&mut self, uid: EntityUid, force: Vector2) -> bool {
        self.physics.apply_force(&mut self.entities, uid, force)
    }

    #[cfg(test)]
    pub(crate) fn apply_linear_impulse(&mut self, uid: EntityUid, impulse: Vector2) -> bool {
        self.physics
            .apply_linear_impulse(&mut self.entities, uid, impulse)
    }

    #[cfg(test)]
    pub(crate) fn apply_angular_impulse(&mut self, uid: EntityUid, impulse: f32) -> bool {
        self.physics
            .apply_angular_impulse(&mut self.entities, uid, impulse)
    }

    pub fn tick_update(&mut self, frame_time: f32) {
        if self.state != ServerState::Running {
            return;
        }

        self.set_current_tick(self.current_tick + 1);
        let users: Vec<_> = self
            .players
            .sessions()
            .map(|session| session.user_id.clone())
            .collect();
        for user_id in users {
            self.process_session_inbound(&user_id);
        }
        let _ = self
            .physics
            .step_simulation(&mut self.entities, &mut self.transforms, frame_time);
        let _ = self
            .transforms
            .process_deferred_moves(&mut self.entities, &mut self.maps);
        let _ = self
            .physics
            .sync_map_physics(&mut self.entities, &self.maps);
        self.entities.inner.update_timer_runtime(frame_time);
        let updates = self.game_states.send_game_state_update(
            &mut self.entities,
            &mut self.maps,
            &mut self.players,
            self.current_tick,
        );
        for (user_id, state) in updates {
            let _ = self.network.send_state(&user_id, state);
        }
    }

    fn process_session_inbound(&mut self, user_id: &str) {
        let batch = self.network.take_session_inbound(user_id);
        self.process_player_list_requests(user_id, batch.player_list_requests);
        self.process_input_batch(user_id, batch.inputs);
        self.process_entity_messages(user_id, batch.entities);
    }

    fn process_player_list_requests(&mut self, user_id: &str, requests: usize) {
        for _ in 0..requests {
            let _ = self.network.send_player_list(
                user_id,
                MsgPlayerList {
                    plyrs: self.players.get_player_states(),
                },
            );
        }
    }

    fn process_input_batch(&mut self, user_id: &str, inputs: Vec<FullInputCmdMessage>) {
        for input in inputs {
            let _ = self
                .input
                .handle_input(&mut self.players, user_id, input.clone());
            let _ = self.input.apply_movement_command(
                &mut self.entities,
                &self.players,
                &mut self.transforms,
                user_id,
                &input,
            );
            let _ = self.input.apply_movement_state(
                &mut self.entities,
                &self.players,
                &self.physics,
                user_id,
            );
        }
    }

    fn process_entity_messages(&mut self, user_id: &str, messages: Vec<MsgEntity>) {
        for message in messages {
            match message.message_type {
                EntityMessageType::ComponentMessage => {
                    self.entities.receive_component_message(
                        user_id.to_string(),
                        message.entity_uid,
                        message.net_id,
                        message.component_message.unwrap_or_default(),
                    );
                }
                EntityMessageType::SystemMessage => {
                    self.entities.receive_system_message(
                        user_id.to_string(),
                        message.system_message.unwrap_or_default(),
                    );
                }
                EntityMessageType::Error => {}
            }
        }
    }

    pub fn shutdown_reason(&self) -> Option<&str> {
        self.shutdown_reason.as_deref()
    }

    pub fn state(&self) -> ServerState {
        self.state
    }

    pub fn current_tick(&self) -> GameTick {
        self.current_tick
    }
}

#[cfg(test)]
mod tests {
    use super::{DaikokuServer, ServerOptions, ServerState};
    use crate::{BoundKeyState, EntityMessageType, FullInputCmdMessage, MsgEntity};
    use butsuri::{AabbShape, Fixture, PhysShape};
    use jikan::GameTick;
    use keisan::{Box2, Vector2};
    use sekai::{BodyType, EntityCoordinates, EntityUid, MapId, ScreenCoordinates, WindowId};

    #[test]
    fn base_server_starts_ticks_and_shuts_down() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert_eq!(server.state(), ServerState::Running);
        assert!(server.connect_player("u1", "pedel"));
        server.set_replication_tick(GameTick::FIRST);
        assert!(server.join_player("u1"));
        server.tick_update(0.016);
        assert_eq!(server.current_tick().value, 1);
        assert_eq!(server.take_outbox("u1").len(), 1);
        server.shutdown(Some("done".to_string()));
        assert_eq!(server.state(), ServerState::Stopped);
    }

    #[test]
    fn base_server_processes_queued_input_and_actor_attachment() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        let uid = server.create_entity_uninitialized(Some("mob"));
        server.initialize_entity(uid);
        assert!(server.attach_player("u1", uid, false));
        server.set_replication_tick(GameTick::FIRST);
        assert!(server.join_player("u1"));

        assert!(server.queue_input(
            "u1",
            FullInputCmdMessage::new(
                jikan::GameTick::new(1),
                0,
                6,
                "MoveRight",
                BoundKeyState::Down,
                EntityCoordinates::new(EntityUid::new(5), Vector2::ZERO),
                ScreenCoordinates::new_xy(5.0, 8.0, WindowId::MAIN),
            ),
        ));

        server.tick_update(0.016);
        assert_eq!(server.player_controlled_entity("u1"), Some(uid));
        assert_eq!(server.last_processed_input("u1"), Some(6));
        assert_eq!(
            server
                .entities
                .inner
                .transforms
                .get(&uid)
                .unwrap()
                .local_position,
            Vector2::new(1.9968, 0.0)
        );
        assert_eq!(
            server
                .entities
                .inner
                .physics
                .get(&uid)
                .unwrap()
                .linear_velocity,
            Vector2::new(62.3, 0.0)
        );
    }

    #[test]
    fn base_server_processes_entity_messages_from_clients() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        assert!(server.queue_entity(
            "u1",
            MsgEntity {
                message_type: EntityMessageType::SystemMessage,
                system_message: Some("jump".to_string()),
                component_message: None,
                entity_uid: EntityUid::new(4),
                net_id: 0,
                sequence: 1,
                source_tick: GameTick::FIRST,
            }
        ));
        assert!(server.queue_entity(
            "u1",
            MsgEntity {
                message_type: EntityMessageType::ComponentMessage,
                system_message: None,
                component_message: Some("poke".to_string()),
                entity_uid: EntityUid::new(4),
                net_id: 8,
                sequence: 2,
                source_tick: GameTick::FIRST,
            }
        ));
        server.tick_update(0.016);
        assert_eq!(
            server.received_system_messages(),
            vec![("u1".to_string(), "jump".to_string())]
        );
        assert_eq!(server.received_component_message_count(), 1);
    }

    #[test]
    fn base_server_responds_to_player_list_requests() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        assert!(server.connect_player("u1", "pedel"));
        assert!(server.queue_player_list_request("u1"));
        server.tick_update(0.016);
        let outbox = server.take_outbox("u1");
        assert!(
            outbox
                .iter()
                .any(|message| matches!(message, crate::OutboundMessage::PlayerList(_)))
        );
    }

    #[test]
    fn base_server_syncs_map_physics_world_each_tick() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        let map_id = server.create_map(Some(MapId::new(2)));
        let uid = server.create_entity_uninitialized(Some("mob"));
        server.initialize_entity(uid);
        assert!(
            server
                .entities
                .inner
                .mutate_transform_and_reconcile(uid, |transform| {
                    transform.map_id = map_id;
                })
        );
        assert!(server.configure_physics_body(
            uid,
            Some(BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        assert!(server.insert_fixture(
            uid,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        ));

        server.tick_update(0.016);
        assert!(server.has_map_broadphase(map_id));
        assert!(server.map_contains_body(map_id, uid));
        assert_eq!(
            server
                .entities
                .inner
                .entities_in_map_aabb(map_id, Box2::new(-2.0, -2.0, 2.0, 2.0)),
            vec![uid]
        );
    }

    #[test]
    fn base_server_applies_map_gravity_and_auto_clear_forces_during_tick() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        let map_id = server.create_map(Some(MapId::new(3)));
        assert!(server.set_map_gravity(map_id, Vector2::new(0.0, -10.0)));
        assert!(server.set_map_auto_clear_forces(map_id, true));

        let uid = server.create_entity_uninitialized(Some("mob"));
        server.initialize_entity(uid);
        assert!(
            server
                .entities
                .inner
                .mutate_transform_and_reconcile(uid, |transform| {
                    transform.map_id = map_id;
                })
        );
        assert!(server.configure_physics_body(
            uid,
            Some(BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        assert!(
            server
                .entities
                .inner
                .mutate_physics_and_reconcile(uid, |body| {
                    body.force = Vector2::new(2.0, 0.0);
                    body.torque = 4.0;
                })
        );
        assert!(server.insert_fixture(
            uid,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        ));

        server.tick_update(0.5);
        let body = server.entities.inner.physics.get(&uid).unwrap();
        assert_eq!(body.linear_velocity, Vector2::new(0.9, -4.5));
        assert_eq!(body.angular_velocity, 1.8);
        assert_eq!(body.force, Vector2::ZERO);
        assert_eq!(body.torque, 0.0);
        let transform = server.entities.inner.transforms.get(&uid).unwrap();
        assert_eq!(transform.local_position, Vector2::new(0.45, -2.25));
        assert!((transform.local_rotation.theta - 0.9).abs() < 0.0001);
    }

    #[test]
    fn base_server_steps_force_and_impulse_applications_into_authoritative_motion() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        let map_id = server.create_map(Some(MapId::new(33)));

        let uid = server.create_entity_uninitialized(Some("mob"));
        server.initialize_entity(uid);
        assert!(
            server
                .entities
                .inner
                .mutate_transform_and_reconcile(uid, |transform| {
                    transform.map_id = map_id;
                })
        );
        assert!(server.configure_physics_body(
            uid,
            Some(BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));

        assert!(server.apply_force(uid, Vector2::new(2.0, 0.0)));
        assert!(server.apply_linear_impulse(uid, Vector2::new(1.0, 0.0)));
        assert!(server.apply_angular_impulse(uid, 4.0));

        server.tick_update(0.5);
        let body = server.entities.inner.physics.get(&uid).unwrap();
        assert_eq!(body.linear_velocity, Vector2::new(1.8, 0.0));
        assert_eq!(body.angular_velocity, 3.6);
        assert_eq!(body.force, Vector2::new(2.0, 0.0));
        assert_eq!(body.torque, 0.0);
        let transform = server.entities.inner.transforms.get(&uid).unwrap();
        assert_eq!(transform.local_position, Vector2::new(0.9, 0.0));
        assert!((transform.local_rotation.theta - 1.8).abs() < 0.0001);
    }
}
