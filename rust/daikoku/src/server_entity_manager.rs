use crate::ActorComponent;
use jikan::GameTick;
use sekai::{
    AppearanceComponentState, EntityManager, MapComponentState, MapGridComponentState,
    MetaDataComponentState, NetworkComponentMessage, PhysicsComponentState, RobustSerializer,
    SerializedComponentChange, SerializedEntityState, TransformComponentState, EntityUid,
    FixturesComponentState, JointComponentState, EntityLookupComponentState,
    BroadphaseComponentState, SharedPhysicsMapComponentState,
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
    const METADATA_NET_ID: u16 = 1;
    const TRANSFORM_NET_ID: u16 = 2;
    const MAP_NET_ID: u16 = 3;
    const MAP_GRID_NET_ID: u16 = 4;
    const PHYSICS_NET_ID: u16 = 5;
    const APPEARANCE_NET_ID: u16 = 6;
    const FIXTURES_NET_ID: u16 = 7;
    const JOINTS_NET_ID: u16 = 8;
    const LOOKUP_NET_ID: u16 = 9;
    const BROADPHASE_NET_ID: u16 = 10;
    const PHYSICS_MAP_NET_ID: u16 = 11;

    pub fn new() -> Self {
        Self {
            inner: EntityManager::new(),
            actors: HashMap::new(),
            component_deletion_history: HashMap::new(),
            received_component_messages: Vec::new(),
            received_system_messages: Vec::new(),
        }
    }

    pub fn initialize(&mut self) {}

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
        let removed = match net_id {
            Self::METADATA_NET_ID => self.inner.metadata.remove(&uid).is_some(),
            Self::TRANSFORM_NET_ID => self.inner.remove_transform_component(uid),
            Self::MAP_NET_ID => self.inner.remove_map_component(uid),
            Self::MAP_GRID_NET_ID => self.inner.remove_map_grid_component(uid),
            Self::PHYSICS_NET_ID => self.inner.physics.remove(&uid).is_some(),
            Self::APPEARANCE_NET_ID => self.inner.appearances.remove(&uid).is_some(),
            Self::FIXTURES_NET_ID => self.inner.fixtures.remove(&uid).is_some(),
            Self::JOINTS_NET_ID => self.inner.joint_components.remove(&uid).is_some(),
            Self::LOOKUP_NET_ID => self.inner.entity_lookups.remove(&uid).is_some(),
            Self::BROADPHASE_NET_ID => self.inner.broadphases.remove(&uid).is_some(),
            Self::PHYSICS_MAP_NET_ID => self.inner.physics_maps.remove(&uid).is_some(),
            _ => false,
        };

        if removed {
            self.note_component_removed(uid, self.inner.current_tick, net_id);
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
        if from_tick == GameTick::ZERO {
            return self.inner.entity_exists(uid);
        }

        self.inner
            .metadata
            .get(&uid)
            .map(|meta| meta.entity_last_modified_tick > from_tick)
            .unwrap_or(false)
    }

    fn component_dirty_since(component: &sekai::Component, from_tick: GameTick) -> bool {
        from_tick == GameTick::ZERO
            || component.creation_tick > from_tick
            || component.last_modified_tick > from_tick
    }

    fn component_created_since(component: &sekai::Component, from_tick: GameTick) -> bool {
        from_tick == GameTick::ZERO || component.creation_tick > from_tick
    }

    fn component_exists_for_net_id(&self, uid: EntityUid, net_id: u16) -> bool {
        match net_id {
            Self::METADATA_NET_ID => self.inner.metadata.contains_key(&uid),
            Self::TRANSFORM_NET_ID => self.inner.transforms.contains_key(&uid),
            Self::MAP_NET_ID => self.inner.map_components.contains_key(&uid),
            Self::MAP_GRID_NET_ID => self.inner.map_grid_components.contains_key(&uid),
            Self::PHYSICS_NET_ID => self.inner.physics.contains_key(&uid),
            Self::APPEARANCE_NET_ID => self.inner.appearances.contains_key(&uid),
            Self::FIXTURES_NET_ID => self.inner.fixtures.contains_key(&uid),
            Self::JOINTS_NET_ID => self.inner.joint_components.contains_key(&uid),
            Self::LOOKUP_NET_ID => self.inner.entity_lookups.contains_key(&uid),
            Self::BROADPHASE_NET_ID => self.inner.broadphases.contains_key(&uid),
            Self::PHYSICS_MAP_NET_ID => self.inner.physics_maps.contains_key(&uid),
            _ => false,
        }
    }

    pub fn build_entity_state(
        &mut self,
        serializer: &mut RobustSerializer,
        uid: EntityUid,
    ) -> Option<SerializedEntityState> {
        self.build_entity_state_since(serializer, uid, GameTick::ZERO)
    }

    pub fn build_entity_state_since(
        &mut self,
        serializer: &mut RobustSerializer,
        uid: EntityUid,
        from_tick: GameTick,
    ) -> Option<SerializedEntityState> {
        if !self.inner.entity_exists(uid) {
            return None;
        }

        let mut changes = Vec::new();

        if let Some(meta) = self.inner.metadata.get(&uid) {
            if from_tick == GameTick::ZERO || meta.entity_last_modified_tick > from_tick {
                changes.push(SerializedComponentChange::new(
                    Self::METADATA_NET_ID,
                    Self::component_created_since(&meta.base, from_tick),
                    false,
                    Some(serializer.serialize_component_state::<MetaDataComponentState>(&meta.get_component_state()).ok()?),
                ));
            }
        }

        if let Some(xform) = self.inner.transforms.get(&uid) {
            if Self::component_dirty_since(&xform.base, from_tick) {
                changes.push(SerializedComponentChange::new(
                    Self::TRANSFORM_NET_ID,
                    Self::component_created_since(&xform.base, from_tick),
                    false,
                    Some(serializer.serialize_component_state::<TransformComponentState>(&xform.get_component_state()).ok()?),
                ));
            }
        }

        if let Some(map) = self.inner.map_components.get(&uid) {
            if Self::component_dirty_since(&map.base, from_tick) {
                changes.push(SerializedComponentChange::new(
                    Self::MAP_NET_ID,
                    Self::component_created_since(&map.base, from_tick),
                    false,
                    Some(serializer.serialize_component_state::<MapComponentState>(&map.get_component_state()).ok()?),
                ));
            }
        }

        if let Some(grid) = self.inner.map_grid_components.get(&uid) {
            if Self::component_dirty_since(&grid.base, from_tick) {
                changes.push(SerializedComponentChange::new(
                    Self::MAP_GRID_NET_ID,
                    Self::component_created_since(&grid.base, from_tick),
                    false,
                    Some(serializer.serialize_component_state::<MapGridComponentState>(&grid.get_component_state()).ok()?),
                ));
            }
        }

        if let Some(physics) = self.inner.physics.get(&uid) {
            if Self::component_dirty_since(&physics.base, from_tick) {
                changes.push(SerializedComponentChange::new(
                    Self::PHYSICS_NET_ID,
                    Self::component_created_since(&physics.base, from_tick),
                    false,
                    Some(serializer.serialize_component_state::<PhysicsComponentState>(&physics.get_component_state()).ok()?),
                ));
            }
        }

        if let Some(appearance) = self.inner.appearances.get(&uid) {
            if Self::component_dirty_since(&appearance.base, from_tick) {
                changes.push(SerializedComponentChange::new(
                    Self::APPEARANCE_NET_ID,
                    Self::component_created_since(&appearance.base, from_tick),
                    false,
                    Some(
                        serializer
                            .serialize_component_state::<AppearanceComponentState>(&appearance.get_component_state())
                            .ok()?,
                    ),
                ));
            }
        }

        if let Some(fixtures) = self.inner.fixtures.get(&uid) {
            if Self::component_dirty_since(&fixtures.base, from_tick) {
                changes.push(SerializedComponentChange::new(
                    Self::FIXTURES_NET_ID,
                    Self::component_created_since(&fixtures.base, from_tick),
                    false,
                    Some(
                        serializer
                            .serialize_component_state::<FixturesComponentState>(&fixtures.get_component_state())
                            .ok()?,
                    ),
                ));
            }
        }

        if let Some(joints) = self.inner.joint_components.get(&uid) {
            if Self::component_dirty_since(&joints.base, from_tick) {
                changes.push(SerializedComponentChange::new(
                    Self::JOINTS_NET_ID,
                    Self::component_created_since(&joints.base, from_tick),
                    false,
                    Some(
                        serializer
                            .serialize_component_state::<JointComponentState>(&joints.get_component_state())
                            .ok()?,
                    ),
                ));
            }
        }

        if let Some(lookup) = self.inner.entity_lookups.get(&uid) {
            if Self::component_dirty_since(&lookup.base, from_tick) {
                changes.push(SerializedComponentChange::new(
                    Self::LOOKUP_NET_ID,
                    Self::component_created_since(&lookup.base, from_tick),
                    false,
                    Some(
                        serializer
                            .serialize_component_state::<EntityLookupComponentState>(&lookup.get_component_state())
                            .ok()?,
                    ),
                ));
            }
        }

        if let Some(broadphase) = self.inner.broadphases.get(&uid) {
            if Self::component_dirty_since(&broadphase.base, from_tick) {
                changes.push(SerializedComponentChange::new(
                    Self::BROADPHASE_NET_ID,
                    Self::component_created_since(&broadphase.base, from_tick),
                    false,
                    Some(
                        serializer
                            .serialize_component_state::<BroadphaseComponentState>(&broadphase.get_component_state())
                            .ok()?,
                    ),
                ));
            }
        }

        if let Some(physics_map) = self.inner.physics_maps.get(&uid) {
            if Self::component_dirty_since(&physics_map.base, from_tick) {
                changes.push(SerializedComponentChange::new(
                    Self::PHYSICS_MAP_NET_ID,
                    Self::component_created_since(&physics_map.base, from_tick),
                    false,
                    Some(
                        serializer
                            .serialize_component_state::<SharedPhysicsMapComponentState>(&physics_map.get_component_state())
                            .ok()?,
                    ),
                ));
            }
        }

        for net_id in self.get_deleted_components(uid, from_tick) {
            if self.component_exists_for_net_id(uid, net_id) {
                continue;
            }
            if changes.iter().any(|change| change.net_id == net_id) {
                continue;
            }
            changes.push(SerializedComponentChange::new(net_id, false, true, None));
        }

        Some(SerializedEntityState { uid, component_changes: changes })
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
    use sekai::{EntityUid, GridId, MapId, RobustSerializer, Tile, TileRenderFlag};
    use butsuri::{AabbShape, Fixture, Joint, JointType, PhysShape};
    use keisan::{Box2, Vector2i};

    #[test]
    fn server_entity_manager_builds_serialized_entity_states() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.create_entity(Some("mob"));
        entities.initialize_entity(uid);
        entities.inner.ensure_fixtures(uid).insert_fixture(Fixture::new(
            "main",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        ));
        entities
            .inner
            .ensure_lookup(uid)
            .add_or_update(uid, Box2::new(-1.0, -1.0, 1.0, 1.0));
        let mut joint = Joint::new(uid.raw(), 999, JointType::Distance);
        joint.id = "rope".to_string();
        entities.inner.ensure_joints(uid).add_joint(joint);
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
        entities.inner.ensure_appearance(uid).set_data("mode", 1u8);
        assert!(entities.remove_component_by_net_id(uid, ServerEntityManager::APPEARANCE_NET_ID));
        let mut serializer = RobustSerializer::new();
        let state = entities
            .build_entity_state_since(&mut serializer, uid, GameTick::new(7))
            .unwrap();
        assert!(state
            .component_changes
            .iter()
            .any(|change| change.net_id == ServerEntityManager::APPEARANCE_NET_ID && change.deleted));
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
        sekai::EntityLookupSystem.update_bounds(&mut entities.inner, child);

        assert!(entities.remove_component_by_net_id(grid, ServerEntityManager::MAP_GRID_NET_ID));
        assert!(!entities.inner.map_grid_components.contains_key(&grid));
        assert!(!entities.inner.map_grids.contains_key(&grid));

        assert!(entities.remove_component_by_net_id(child, ServerEntityManager::TRANSFORM_NET_ID));
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

        assert!(entities.remove_component_by_net_id(map, ServerEntityManager::MAP_NET_ID));
        assert!(!entities.inner.map_components.contains_key(&map));
        assert!(!entities.inner.broadphases.contains_key(&map));
        assert!(!entities.inner.physics_maps.contains_key(&map));
        let grid_transform = entities.inner.transforms.get(&grid).unwrap();
        assert_eq!(grid_transform.parent, EntityUid::INVALID);
        assert_eq!(grid_transform.map_id, MapId::NULLSPACE);
        assert_eq!(entities.inner.map_grids.get(&grid).unwrap().parent_map_id, MapId::NULLSPACE);
    }
}
