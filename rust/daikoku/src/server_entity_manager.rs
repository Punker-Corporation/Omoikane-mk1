use crate::ActorComponent;
use jikan::GameTick;
use sekai::{
    EntityManager, EntityUid, NetworkComponentMessage, RobustSerializer, SerializedEntityState,
};
use std::collections::HashMap;

pub struct ServerEntityManager {
    pub inner: EntityManager,
    pub actors: HashMap<EntityUid, ActorComponent>,
    component_deletion_history: HashMap<EntityUid, Vec<(GameTick, u16)>>,
    pub received_component_messages: Vec<NetworkComponentMessage<(), String, String>>,
    pub received_system_messages: Vec<(String, String)>,
}

impl ServerEntityManager {
    pub fn new() -> Self {
        Self {
            inner: EntityManager::new(),
            actors: HashMap::new(),
            component_deletion_history: HashMap::new(),
            received_component_messages: Vec::new(),
            received_system_messages: Vec::new(),
        }
    }

    pub fn set_current_tick(&mut self, tick: GameTick) {
        self.inner.current_tick = tick;
    }

    pub fn alloc_entity(&mut self, prototype_name: Option<&str>, uid: EntityUid) -> EntityUid {
        self.inner.alloc_entity_external(uid, prototype_name);
        uid
    }

    pub fn create_entity(&mut self, prototype_name: Option<&str>) -> EntityUid {
        self.inner.create_entity_uninitialized(prototype_name)
    }

    pub fn initialize_entity(&mut self, entity: EntityUid) {
        let _ = self.inner.initialize_entity(entity);
    }

    pub fn delete_entity(&mut self, entity: EntityUid) {
        self.actors.remove(&entity);
        self.inner.queue_delete_entity(entity);
        self.inner.flush_queued_deletions();
    }

    pub fn note_component_removed(&mut self, uid: EntityUid, tick: GameTick, net_id: u16) {
        self.component_deletion_history
            .entry(uid)
            .or_default()
            .push((tick, net_id));
    }

    pub fn get_deleted_components(&self, uid: EntityUid, from_tick: GameTick) -> Vec<u16> {
        self.component_deletion_history
            .get(&uid)
            .map(|history| {
                history
                    .iter()
                    .filter_map(|(tick, net_id)| (*tick >= from_tick).then_some(*net_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn remove_component_by_net_id(&mut self, uid: EntityUid, net_id: u16) -> bool {
        let joints_present_before_physics_remove =
            net_id == EntityManager::PHYSICS_NET_ID && self.inner.joint_components.contains_key(&uid);
        let removed = self.inner.remove_component_by_net_id(uid, net_id);

        if removed {
            self.note_component_removed(uid, self.inner.current_tick, net_id);
            if joints_present_before_physics_remove && !self.inner.joint_components.contains_key(&uid) {
                self.note_component_removed(uid, self.inner.current_tick, EntityManager::JOINTS_NET_ID);
            }
            if self.inner.entity_exists(uid) {
                self.inner.dirty_entity(uid);
            }
        }

        removed
    }

    pub fn cull_deletion_history(&mut self, oldest_ack: GameTick) {
        self.component_deletion_history.retain(|_, history| {
            history.retain(|(tick, _)| *tick >= oldest_ack);
            !history.is_empty()
        });
    }

    pub fn entity_dirty_since(&self, uid: EntityUid, from_tick: GameTick) -> bool {
        self.inner.entity_dirty_since(uid, from_tick)
    }

    pub fn build_entity_state(
        &mut self,
        serializer: &mut RobustSerializer,
        uid: EntityUid,
    ) -> Option<SerializedEntityState> {
        self.inner.build_serialized_entity_state(serializer, uid)
    }

    pub fn build_entity_state_since(
        &mut self,
        serializer: &mut RobustSerializer,
        uid: EntityUid,
        from_tick: GameTick,
    ) -> Option<SerializedEntityState> {
        let deletion_history = &self.component_deletion_history;
        self.inner.build_serialized_entity_state_with_deleted_callback(
            serializer,
            uid,
            from_tick,
            |uid, from_tick| {
                deletion_history
                    .get(&uid)
                    .map(|history| {
                        history
                            .iter()
                            .filter_map(|(tick, net_id)| (*tick >= from_tick).then_some(*net_id))
                            .collect()
                    })
                    .unwrap_or_default()
            },
        )
    }

    pub fn build_entity_states_for_sync(
        &mut self,
        serializer: &mut RobustSerializer,
        ids: &[EntityUid],
        newly_visible: &[EntityUid],
        from_tick: GameTick,
    ) -> Vec<SerializedEntityState> {
        let deletion_history = &self.component_deletion_history;
        self.inner.build_serialized_entity_states_for_sync(
            serializer,
            ids,
            newly_visible,
            from_tick,
            |uid, from_tick| {
                deletion_history
                    .get(&uid)
                    .map(|history| {
                        history
                            .iter()
                            .filter_map(|(tick, net_id)| (*tick >= from_tick).then_some(*net_id))
                            .collect()
                    })
                    .unwrap_or_default()
            },
        )
    }

    pub fn receive_component_message(
        &mut self,
        user_id: impl Into<String>,
        uid: EntityUid,
        net_id: u32,
        payload: impl Into<String>,
    ) {
        self.received_component_messages.push(NetworkComponentMessage::new(
            (),
            uid,
            net_id,
            payload.into(),
            Some(user_id.into()),
        ));
    }

    pub fn receive_system_message(&mut self, user_id: impl Into<String>, payload: impl Into<String>) {
        self.received_system_messages.push((user_id.into(), payload.into()));
    }
}

impl Default for ServerEntityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::ServerEntityManager;
    use jikan::GameTick;
    use sekai::{
        EntityManager, EntityUid, GridId, MapId, PhysicsComponentState, RobustSerializer, Tile,
        TileRenderFlag,
    };
    use butsuri::{AabbShape, Fixture, Joint, JointType, PhysShape};
    use keisan::{Box2, Vector2i};

    #[test]
    fn server_entity_manager_builds_serialized_entity_states() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.create_entity(Some("mob"));
        entities.initialize_entity(uid);
        entities.inner.ensure_collision_wake(uid);
        entities.inner.ensure_collide_on_anchor(uid);
        let _ = entities.inner.insert_fixture_and_reconcile(
            uid,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
            ),
        );
        entities
            .inner
            .ensure_lookup(uid)
            .add_or_update(uid, Box2::new(-1.0, -1.0, 1.0, 1.0));
        let mut joint = Joint::new(uid.raw(), 999, JointType::Distance);
        joint.id = "rope".to_string();
        assert!(entities.inner.add_joint_between(joint));
        entities.inner.ensure_broadphase(uid);
        entities.inner.ensure_physics_map(uid).add_body(uid, true);
        let mut serializer = RobustSerializer::new();
        let state = entities.build_entity_state(&mut serializer, uid).unwrap();
        assert_eq!(state.uid, uid);
        assert!(state.component_changes.iter().any(|change| change.net_id == 7));
        assert!(state.component_changes.iter().any(|change| change.net_id == 8));
        assert!(state.component_changes.iter().any(|change| change.net_id == 9));
        assert!(state.component_changes.iter().any(|change| change.net_id == 10));
        assert!(state.component_changes.iter().any(|change| change.net_id == 11));
        assert!(state.component_changes.iter().any(|change| change.net_id == 12));
        assert!(state.component_changes.iter().any(|change| change.net_id == 13));
        entities.note_component_removed(uid, GameTick::new(5), 7);
        assert_eq!(entities.get_deleted_components(uid, GameTick::new(4)), vec![7]);
        let empty = entities
            .build_entity_state_since(&mut serializer, uid, GameTick::new(100))
            .unwrap();
        assert!(empty.component_changes.is_empty());
    }

    #[test]
    fn server_entity_manager_emits_deleted_component_changes_incrementally() {
        let mut entities = ServerEntityManager::new();
        entities.set_current_tick(GameTick::new(8));
        let uid = entities.create_entity(Some("mob"));
        entities.initialize_entity(uid);
        assert!(entities.inner.set_appearance_data(uid, "mode", 1u8));
        assert!(entities.remove_component_by_net_id(uid, EntityManager::APPEARANCE_NET_ID));
        let mut serializer = RobustSerializer::new();
        let state = entities
            .build_entity_state_since(&mut serializer, uid, GameTick::new(7))
            .unwrap();
        assert!(state
            .component_changes
            .iter()
            .any(|change| change.net_id == EntityManager::APPEARANCE_NET_ID && change.deleted));
    }

    #[test]
    fn server_entity_manager_removes_transform_and_grid_components_with_runtime_cleanup() {
        let mut entities = ServerEntityManager::new();
        let map = entities.create_entity(None);
        entities.initialize_entity(map);
        entities.inner.ensure_map(MapId::new(1), map);

        let grid = entities.create_entity(None);
        entities.initialize_entity(grid);
        let grid_component = entities
            .inner
            .map_grid_components
            .entry(grid)
            .or_insert_with(sekai::MapGridComponent::new);
        grid_component.base.owner = grid;
        grid_component.grid_index = GridId::new(3);
        let map_grid = grid_component.alloc_map_grid(grid, MapId::new(1), 1).clone();
        entities.inner.map_grids.insert(grid, map_grid);
        entities.inner.set_parent(grid, map);
        entities
            .inner
            .map_grids
            .get_mut(&grid)
            .unwrap()
            .set_tile(Vector2i::new(0, 0), Tile::new(1, TileRenderFlag(0), 0));

        let child = entities.create_entity(None);
        entities.initialize_entity(child);
        let _ = entities.inner.apply_transform_state(
            child,
            sekai::TransformComponentState {
                local_position: keisan::Vector2::new(0.5, 0.5),
                rotation: keisan::Angle::ZERO,
                parent_id: grid,
                map_id: MapId::new(1),
                grid_id: GridId::new(3),
                no_local_rotation: false,
                anchored: false,
            },
        );
        entities.inner.reconcile_transform_runtime(child, None);

        assert!(entities.remove_component_by_net_id(grid, EntityManager::MAP_GRID_NET_ID));
        assert!(!entities.inner.map_grid_components.contains_key(&grid));
        assert!(!entities.inner.map_grids.contains_key(&grid));

        assert!(entities.remove_component_by_net_id(child, EntityManager::TRANSFORM_NET_ID));
        assert!(!entities.inner.transforms.contains_key(&child));
    }

    #[test]
    fn server_entity_manager_removes_map_components_with_runtime_cleanup() {
        let mut entities = ServerEntityManager::new();
        let map = entities.create_entity(None);
        entities.initialize_entity(map);
        entities.inner.ensure_map(MapId::new(1), map);
        entities.inner.ensure_broadphase(map);
        entities.inner.ensure_physics_map(map);

        let grid = entities.create_entity(None);
        entities.initialize_entity(grid);
        let grid_component = entities
            .inner
            .map_grid_components
            .entry(grid)
            .or_insert_with(sekai::MapGridComponent::new);
        grid_component.base.owner = grid;
        grid_component.grid_index = GridId::new(5);
        let map_grid = grid_component.alloc_map_grid(grid, MapId::new(1), 1).clone();
        entities.inner.map_grids.insert(grid, map_grid);
        entities.inner.set_parent(grid, map);

        assert!(entities.remove_component_by_net_id(map, EntityManager::MAP_NET_ID));
        assert!(!entities.inner.map_components.contains_key(&map));
        assert!(!entities.inner.has_map_broadphase(MapId::new(1)));
        assert!(!entities.inner.has_map_physics_runtime(MapId::new(1)));
        let grid_transform = entities.inner.transforms.get(&grid).unwrap();
        assert_eq!(grid_transform.parent, EntityUid::INVALID);
        assert_eq!(grid_transform.map_id, MapId::NULLSPACE);
        assert_eq!(entities.inner.map_grids.get(&grid).unwrap().parent_map_id, MapId::NULLSPACE);
    }

    #[test]
    fn server_entity_manager_removes_physics_and_fixtures_with_runtime_cleanup() {
        let mut entities = ServerEntityManager::new();
        let map = entities.create_entity(None);
        entities.initialize_entity(map);
        entities.inner.ensure_map(MapId::new(2), map);
        entities.inner.transforms.get_mut(&map).unwrap().map_id = MapId::new(2);
        entities.inner.ensure_broadphase(map);
        entities.inner.ensure_physics_map(map);

        let first = entities.create_entity(None);
        entities.initialize_entity(first);
        let _ = entities.inner.apply_transform_state(
            first,
            sekai::TransformComponentState {
                local_position: keisan::Vector2::ZERO,
                rotation: keisan::Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(2),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(entities.inner.configure_physics_body(
            first,
            Some(sekai::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        let _ = entities.inner.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "first",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let second = entities.create_entity(None);
        entities.initialize_entity(second);
        let _ = entities.inner.apply_transform_state(
            second,
            sekai::TransformComponentState {
                local_position: keisan::Vector2::new(0.5, 0.0),
                rotation: keisan::Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(2),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(entities.inner.configure_physics_body(
            second,
            Some(sekai::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        let _ = entities.inner.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "second",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        entities.inner.refresh_entities_runtime(&[first, second]);
        assert_eq!(entities.inner.map_contact_count(MapId::new(2)), 1);

        assert!(entities.remove_component_by_net_id(first, EntityManager::PHYSICS_NET_ID));
        assert_eq!(entities.inner.map_contact_count(MapId::new(2)), 0);
        assert_eq!(entities.inner.query_aabb_entities(map, Box2::new(-1.0, -1.0, 1.0, 1.0)), vec![second]);

        assert!(entities.remove_component_by_net_id(second, EntityManager::FIXTURES_NET_ID));
        assert!(entities
            .inner
            .query_aabb_entities(map, Box2::new(-1.0, -1.0, 1.0, 1.0))
            .is_empty());
    }

    #[test]
    fn server_entity_manager_removes_joint_components_symmetrically_with_runtime_cleanup() {
        let mut entities = ServerEntityManager::new();
        let map = entities.create_entity(None);
        entities.initialize_entity(map);
        entities.inner.ensure_map(MapId::new(3), map);
        entities.inner.transforms.get_mut(&map).unwrap().map_id = MapId::new(3);
        entities.inner.ensure_broadphase(map);
        entities.inner.ensure_physics_map(map);

        let first = entities.create_entity(None);
        entities.initialize_entity(first);
        let _ = entities.inner.apply_transform_state(
            first,
            sekai::TransformComponentState {
                local_position: keisan::Vector2::ZERO,
                rotation: keisan::Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(3),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(entities.inner.configure_physics_body(
            first,
            Some(sekai::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        let _ = entities.inner.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "first",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let second = entities.create_entity(None);
        entities.initialize_entity(second);
        let _ = entities.inner.apply_transform_state(
            second,
            sekai::TransformComponentState {
                local_position: keisan::Vector2::new(0.5, 0.0),
                rotation: keisan::Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(3),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(entities.inner.configure_physics_body(
            second,
            Some(sekai::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        let _ = entities.inner.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "second",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let mut joint = Joint::new(first.raw(), second.raw(), JointType::Distance);
        joint.id = "rope".to_string();
        joint.collide_connected = false;
        assert!(entities.inner.add_joint_between(joint));
        entities.inner.refresh_entities_runtime(&[first, second]);
        assert_eq!(entities.inner.map_contact_count(MapId::new(3)), 0);

        assert!(entities.remove_component_by_net_id(first, EntityManager::JOINTS_NET_ID));
        assert!(!entities.inner.joint_components.contains_key(&first));
        assert_eq!(entities.inner.joint_components.get(&second).unwrap().joint_count(), 0);
        assert_eq!(entities.inner.map_contact_count(MapId::new(3)), 1);
    }

    #[test]
    fn server_entity_manager_removing_physics_also_emits_joint_component_deletion() {
        let mut entities = ServerEntityManager::new();
        entities.set_current_tick(GameTick::new(9));
        let first = entities.create_entity(None);
        entities.initialize_entity(first);
        entities.inner.ensure_physics(first);
        let second = entities.create_entity(None);
        entities.initialize_entity(second);
        entities.inner.ensure_physics(second);

        let mut joint = Joint::new(first.raw(), second.raw(), JointType::Distance);
        joint.id = "rope".to_string();
        assert!(entities.inner.add_joint_between(joint));

        assert!(entities.remove_component_by_net_id(first, EntityManager::PHYSICS_NET_ID));
        let deleted = entities.get_deleted_components(first, GameTick::new(8));
        assert!(deleted.contains(&EntityManager::PHYSICS_NET_ID));
        assert!(deleted.contains(&EntityManager::JOINTS_NET_ID));
        assert_eq!(entities.inner.joint_components.get(&second).unwrap().joint_count(), 0);
    }

    #[test]
    fn server_entity_manager_builds_runtime_component_deltas_after_immediate_physics_refresh() {
        let mut entities = ServerEntityManager::new();
        let map = entities.create_entity(None);
        entities.initialize_entity(map);
        entities.inner.ensure_map(MapId::new(4), map);
        entities.inner.transforms.get_mut(&map).unwrap().map_id = MapId::new(4);
        entities.inner.ensure_broadphase(map);
        entities.inner.ensure_physics_map(map);

        let uid = entities.create_entity(None);
        entities.initialize_entity(uid);
        let _ = entities.inner.apply_transform_state(
            uid,
            sekai::TransformComponentState {
                local_position: keisan::Vector2::ZERO,
                rotation: keisan::Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(4),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(entities.inner.configure_physics_body(
            uid,
            Some(sekai::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        let _ = entities.inner.insert_fixture_and_reconcile(
            uid,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );
        entities.inner.refresh_map_physics_runtime(MapId::new(4));

        entities.set_current_tick(GameTick::new(5));
        let physics = crate::PhysicsSystem::new();
        assert!(physics.set_can_collide(&mut entities, uid, false));

        let mut serializer = RobustSerializer::new();
        let state = entities
            .build_entity_state_since(&mut serializer, map, GameTick::new(4))
            .unwrap();
        assert!(state
            .component_changes
            .iter()
            .any(|change| change.net_id == EntityManager::BROADPHASE_NET_ID));
        assert!(state
            .component_changes
            .iter()
            .any(|change| change.net_id == EntityManager::PHYSICS_MAP_NET_ID));
    }

    #[test]
    fn server_entity_manager_serializes_awake_state_in_physics_component() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.create_entity(None);
        entities.initialize_entity(uid);
        assert!(entities.inner.configure_physics_body(
            uid,
            Some(sekai::BodyType::Dynamic),
            Some(false),
            Some(true),
            None,
        ));

        let mut serializer = RobustSerializer::new();
        let state = entities.build_entity_state(&mut serializer, uid).unwrap();
        let physics = state
            .component_changes
            .iter()
            .find(|change| change.net_id == EntityManager::PHYSICS_NET_ID)
            .and_then(|change| change.state.as_ref())
            .map(|component| {
                serializer
                    .deserialize_component_state::<PhysicsComponentState>(component)
                    .unwrap()
            })
            .unwrap();
        assert!(!physics.awake);
    }

    #[test]
    fn server_entity_manager_serializes_collision_wake_component_state() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.create_entity(None);
        entities.initialize_entity(uid);
        entities.inner.ensure_collision_wake(uid).enabled = false;

        let mut serializer = RobustSerializer::new();
        let state = entities.build_entity_state(&mut serializer, uid).unwrap();
        let collision_wake = state
            .component_changes
            .iter()
            .find(|change| change.net_id == EntityManager::COLLISION_WAKE_NET_ID)
            .and_then(|change| change.state.as_ref())
            .map(|component| {
                serializer
                    .deserialize_component_state::<sekai::CollisionWakeComponentState>(component)
                    .unwrap()
            })
            .unwrap();
        assert!(!collision_wake.enabled);
    }

    #[test]
    fn server_entity_manager_serializes_map_paused_state() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.create_entity(None);
        entities.initialize_entity(uid);
        entities.inner.ensure_map(MapId::new(77), uid);
        assert!(entities.inner.set_map_paused(uid, true));

        let mut serializer = RobustSerializer::new();
        let state = entities.build_entity_state(&mut serializer, uid).unwrap();
        let map = state
            .component_changes
            .iter()
            .find(|change| change.net_id == EntityManager::MAP_NET_ID)
            .and_then(|change| change.state.as_ref())
            .map(|component| {
                serializer
                    .deserialize_component_state::<sekai::MapComponentState>(component)
                    .unwrap()
            })
            .unwrap();
        assert!(map.map_paused);
    }

    #[test]
    fn server_entity_manager_serializes_collide_on_anchor_component_state() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.create_entity(None);
        entities.initialize_entity(uid);
        entities.inner.ensure_collide_on_anchor(uid).enable = true;

        let mut serializer = RobustSerializer::new();
        let state = entities.build_entity_state(&mut serializer, uid).unwrap();
        let collide_on_anchor = state
            .component_changes
            .iter()
            .find(|change| change.net_id == EntityManager::COLLIDE_ON_ANCHOR_NET_ID)
            .and_then(|change| change.state.as_ref())
            .map(|component| {
                serializer
                    .deserialize_component_state::<sekai::CollideOnAnchorComponentState>(component)
                    .unwrap()
            })
            .unwrap();
        assert!(collide_on_anchor.enable);
    }

    #[test]
    fn server_entity_manager_build_entity_states_for_sync_respects_visibility_and_dirtying() {
        let mut entities = ServerEntityManager::new();
        entities.set_current_tick(GameTick::new(4));

        let newly_visible = entities.create_entity(Some("new"));
        entities.initialize_entity(newly_visible);

        let dirty = entities.create_entity(Some("dirty"));
        entities.initialize_entity(dirty);

        let clean = entities.create_entity(Some("clean"));
        entities.initialize_entity(clean);

        entities.set_current_tick(GameTick::new(5));
        assert!(entities.inner.set_appearance_data(newly_visible, "mode", 1u8));
        assert!(entities.inner.set_appearance_data(dirty, "mode", 2u8));
        entities.inner.dirty_entity(dirty);

        let mut serializer = RobustSerializer::new();
        let states = entities.build_entity_states_for_sync(
            &mut serializer,
            &[newly_visible, dirty, clean],
            &[newly_visible],
            GameTick::new(4),
        );

        assert_eq!(states.len(), 2);
        assert!(states.iter().any(|state| state.uid == newly_visible));
        assert!(states.iter().any(|state| state.uid == dirty));
        assert!(!states.iter().any(|state| state.uid == clean));
    }
}
