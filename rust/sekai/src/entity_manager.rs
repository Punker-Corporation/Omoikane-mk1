use crate::{
    AppearanceComponent, BroadphaseComponent, CollideOnAnchorComponent, CollisionWakeComponent,
    Component, EntityDeletedMessage, EntityInitializedMessage, EntityLookupComponent, EntitySystem,
    EntitySystemManager, EntityTerminatingEvent, EntityUid, FixturesComponent, GameState,
    GameStateMapData, IgnorePauseComponent, JointComponent, MapComponent, MapComponentState,
    MapCoordinates, MapGrid, MapGridComponent, MapGridComponentState, MapId, MetaDataComponent,
    PhysicsComponent, PhysicsQueryHit, RobustSerializer, SerializableComponentState,
    SerializedComponentChange, SerializedEntityState, SharedPhysicsMapComponent, Tile, TileRef,
    TimerComponent, TransformComponent, TransformComponentState, TransformResolver, WorldTransform,
    map_grid::MapGridLike,
};
use butsuri::CollisionRay;
use jikan::GameTick;
use keisan::{Angle, Box2, Vector2, Vector2i};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityStringRepresentation {
    pub uid: EntityUid,
    pub deleted: bool,
    pub name: Option<String>,
    pub prototype: Option<String>,
}

impl EntityStringRepresentation {
    pub fn new(
        uid: EntityUid,
        deleted: bool,
        name: Option<String>,
        prototype: Option<String>,
    ) -> Self {
        Self {
            uid,
            deleted,
            name,
            prototype,
        }
    }
}

pub struct EntityManager {
    pub current_tick: GameTick,
    entity_sys_manager: EntitySystemManager,
    queued_deletions: VecDeque<EntityUid>,
    queued_deletions_set: HashSet<EntityUid>,
    pub entities: HashSet<EntityUid>,
    next_entity_uid: i32,
    pub metadata: HashMap<EntityUid, MetaDataComponent>,
    pub transforms: HashMap<EntityUid, TransformComponent>,
    pub map_components: HashMap<EntityUid, MapComponent>,
    pub appearances: HashMap<EntityUid, AppearanceComponent>,
    pub entity_lookups: HashMap<EntityUid, EntityLookupComponent>,
    pub joint_components: HashMap<EntityUid, JointComponent>,
    pub ignore_pauses: HashMap<EntityUid, IgnorePauseComponent>,
    pub collision_wakes: HashMap<EntityUid, CollisionWakeComponent>,
    pub collide_on_anchors: HashMap<EntityUid, CollideOnAnchorComponent>,
    pub timers: HashMap<EntityUid, TimerComponent>,
    pub physics: HashMap<EntityUid, PhysicsComponent>,
    pub fixtures: HashMap<EntityUid, FixturesComponent>,
    pub broadphases: HashMap<EntityUid, BroadphaseComponent>,
    pub physics_maps: HashMap<EntityUid, SharedPhysicsMapComponent>,
    pub map_grids: HashMap<EntityUid, MapGrid>,
    pub map_grid_components: HashMap<EntityUid, MapGridComponent>,
    pub components: HashMap<EntityUid, Vec<Component>>,
    entity_runtime_events: VecDeque<crate::EntityRuntimeEvent>,
    appearance_dirty_components: HashSet<EntityUid>,
    deferred_grid_moves: VecDeque<crate::MoveEvent>,
    deferred_other_moves: VecDeque<crate::MoveEvent>,
    processing_subscription_side_effects: bool,
}

impl EntityManager {
    pub const METADATA_NET_ID: u16 = 1;
    pub const TRANSFORM_NET_ID: u16 = 2;
    pub const MAP_NET_ID: u16 = 3;
    pub const MAP_GRID_NET_ID: u16 = 4;
    pub const PHYSICS_NET_ID: u16 = 5;
    pub const APPEARANCE_NET_ID: u16 = 6;
    pub const FIXTURES_NET_ID: u16 = 7;
    pub const JOINTS_NET_ID: u16 = 8;
    pub const LOOKUP_NET_ID: u16 = 9;
    pub const BROADPHASE_NET_ID: u16 = 10;
    pub const PHYSICS_MAP_NET_ID: u16 = 11;
    pub const COLLISION_WAKE_NET_ID: u16 = 12;
    pub const COLLIDE_ON_ANCHOR_NET_ID: u16 = 13;
    pub const SERIALIZED_NET_IDS: [u16; 13] = [
        Self::METADATA_NET_ID,
        Self::TRANSFORM_NET_ID,
        Self::MAP_NET_ID,
        Self::MAP_GRID_NET_ID,
        Self::PHYSICS_NET_ID,
        Self::APPEARANCE_NET_ID,
        Self::FIXTURES_NET_ID,
        Self::JOINTS_NET_ID,
        Self::LOOKUP_NET_ID,
        Self::BROADPHASE_NET_ID,
        Self::PHYSICS_MAP_NET_ID,
        Self::COLLISION_WAKE_NET_ID,
        Self::COLLIDE_ON_ANCHOR_NET_ID,
    ];
    const INIT_AND_START_COMPONENT_ORDER: [&'static str; 15] = [
        "TransformComponent",
        "Physics",
        "AppearanceComponent",
        "EntityLookupComponent",
        "JointComponent",
        "IgnorePauseComponent",
        "CollisionWakeComponent",
        "CollideOnAnchorComponent",
        "TimerComponent",
        "FixturesComponent",
        "BroadphaseComponent",
        "SharedPhysicsMapComponent",
        "MapComponent",
        "MapGridComponent",
        "MetaDataComponent",
    ];
    const ENTITY_EVENT_COMPONENT_ORDER: [&'static str; 15] = [
        "TransformComponent",
        "MetaDataComponent",
        "MapComponent",
        "MapGridComponent",
        "AppearanceComponent",
        "EntityLookupComponent",
        "JointComponent",
        "IgnorePauseComponent",
        "CollisionWakeComponent",
        "CollideOnAnchorComponent",
        "TimerComponent",
        "Physics",
        "FixturesComponent",
        "BroadphaseComponent",
        "SharedPhysicsMapComponent",
    ];
    const DELETE_COMPONENT_ORDER: [&'static str; 15] = [
        "MapGridComponent",
        "MapComponent",
        "SharedPhysicsMapComponent",
        "BroadphaseComponent",
        "FixturesComponent",
        "TimerComponent",
        "CollideOnAnchorComponent",
        "CollisionWakeComponent",
        "IgnorePauseComponent",
        "JointComponent",
        "EntityLookupComponent",
        "AppearanceComponent",
        "Physics",
        "TransformComponent",
        "MetaDataComponent",
    ];

    fn shared_physics() -> crate::shared_physics_system::SharedPhysicsSystem {
        crate::shared_physics_system::SharedPhysicsSystem::new()
    }

    pub fn map_transform_state(map_id: MapId) -> TransformComponentState {
        TransformComponentState {
            local_position: Vector2::ZERO,
            rotation: keisan::Angle::ZERO,
            parent_id: EntityUid::INVALID,
            map_id,
            grid_id: crate::GridId::INVALID,
            no_local_rotation: false,
            anchored: false,
        }
    }

    pub fn grid_transform_state(
        map_id: MapId,
        parent_id: EntityUid,
        grid_id: crate::GridId,
        local_position: Vector2,
        rotation: keisan::Angle,
    ) -> TransformComponentState {
        TransformComponentState {
            local_position,
            rotation,
            parent_id,
            map_id,
            grid_id,
            no_local_rotation: false,
            anchored: false,
        }
    }

    pub fn new() -> Self {
        Self {
            current_tick: GameTick::ZERO,
            entity_sys_manager: EntitySystemManager::new(),
            queued_deletions: VecDeque::new(),
            queued_deletions_set: HashSet::new(),
            entities: HashSet::new(),
            next_entity_uid: EntityUid::FIRST_UID.raw(),
            metadata: HashMap::new(),
            transforms: HashMap::new(),
            map_components: HashMap::new(),
            appearances: HashMap::new(),
            entity_lookups: HashMap::new(),
            joint_components: HashMap::new(),
            ignore_pauses: HashMap::new(),
            collision_wakes: HashMap::new(),
            collide_on_anchors: HashMap::new(),
            timers: HashMap::new(),
            physics: HashMap::new(),
            fixtures: HashMap::new(),
            broadphases: HashMap::new(),
            physics_maps: HashMap::new(),
            map_grids: HashMap::new(),
            map_grid_components: HashMap::new(),
            components: HashMap::new(),
            entity_runtime_events: VecDeque::new(),
            appearance_dirty_components: HashSet::new(),
            deferred_grid_moves: VecDeque::new(),
            deferred_other_moves: VecDeque::new(),
            processing_subscription_side_effects: false,
        }
    }

    pub fn add_entity_system(&mut self, system: Box<dyn EntitySystem>) {
        self.entity_sys_manager.add_system(system);
    }

    pub fn initialize_entity_systems(&mut self) {
        self.entity_sys_manager.initialize();
    }

    pub fn shutdown_entity_systems(&mut self) {
        self.entity_sys_manager.shutdown();
    }

    pub fn entity_systems_initialized(&self) -> bool {
        self.entity_sys_manager.initialized
    }

    pub fn entity_system_count(&self) -> usize {
        self.entity_sys_manager.system_count()
    }

    pub fn entity_system_names(&self) -> Vec<String> {
        self.entity_sys_manager.system_names()
    }

    pub fn create_entity_uninitialized(&mut self, prototype_name: Option<&str>) -> EntityUid {
        let uid = self.generate_entity_uid();
        self.alloc_entity(uid, prototype_name);
        uid
    }

    pub fn create_entity_uninitialized_with_transform(
        &mut self,
        prototype_name: Option<&str>,
        state: TransformComponentState,
    ) -> EntityUid {
        let uid = self.create_entity_uninitialized(prototype_name);
        let _ = self.apply_transform_state(uid, state);
        uid
    }

    pub fn create_entity_uninitialized_at_map(
        &mut self,
        prototype_name: Option<&str>,
        coordinates: MapCoordinates,
    ) -> EntityUid {
        self.create_entity_uninitialized_with_transform(
            prototype_name,
            TransformComponentState {
                local_position: coordinates.position,
                rotation: keisan::Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: coordinates.map_id,
                grid_id: crate::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        )
    }

    pub fn create_entity_uninitialized_as_map(
        &mut self,
        prototype_name: Option<&str>,
        map_id: MapId,
    ) -> EntityUid {
        let uid = self.create_entity_uninitialized_with_transform(
            prototype_name,
            Self::map_transform_state(map_id),
        );
        let _ = self.apply_map_component_state(
            uid,
            MapComponentState {
                map_id,
                lighting_enabled: true,
                map_paused: false,
            },
        );
        uid
    }

    pub fn create_entity_uninitialized_as_grid(
        &mut self,
        prototype_name: Option<&str>,
        map_id: MapId,
        parent_id: EntityUid,
        grid_id: crate::GridId,
        chunk_size: u16,
        local_position: Vector2,
        rotation: keisan::Angle,
    ) -> EntityUid {
        let uid = self.create_entity_uninitialized_with_transform(
            prototype_name,
            Self::grid_transform_state(map_id, parent_id, grid_id, local_position, rotation),
        );
        let _ = self.apply_map_grid_component_state(
            uid,
            MapGridComponentState {
                grid_index: grid_id,
                chunk_size,
            },
        );
        uid
    }

    pub fn initialize_entity(&mut self, entity: EntityUid) -> EntityInitializedMessage {
        if let Some(meta) = self.metadata.get_mut(&entity) {
            meta.entity_life_stage = crate::EntityLifeStage::Initializing;
            meta.dirty(self.current_tick);
        }
        self.initialize_existing_components_for_entity(entity);
        if let Some(meta) = self.metadata.get_mut(&entity) {
            meta.entity_life_stage = crate::EntityLifeStage::Initialized;
            meta.dirty(self.current_tick);
        }
        let message = EntityInitializedMessage { entity };
        self.queue_entity_event(message);
        self.start_existing_components_for_entity(entity);
        self.run_map_init_if_current_map_initialized(entity);
        message
    }

    pub fn dirty_entity(&mut self, uid: EntityUid) {
        let Some(metadata) = self.metadata.get_mut(&uid) else {
            panic!("Entity {uid} does not exist, cannot dirty it.");
        };
        if metadata.entity_last_modified_tick != self.current_tick {
            metadata.entity_last_modified_tick = self.current_tick;
        }
    }

    pub fn set_entity_paused(&mut self, uid: EntityUid, paused: bool) -> bool {
        if paused && self.ignore_pauses.contains_key(&uid) {
            return false;
        }
        let Some(meta) = self.metadata.get_mut(&uid) else {
            return false;
        };
        if meta.entity_paused == paused {
            return false;
        }
        meta.entity_paused = paused;
        meta.dirty(self.current_tick);
        let mut event = crate::EntityPausedEvent {
            entity: uid,
            paused,
        };
        self.raise_component_event(uid, "MetaDataComponent", &mut event);
        self.entity_runtime_events.push_back(event.into());
        true
    }

    pub fn set_map_paused(&mut self, uid: EntityUid, paused: bool) -> bool {
        let previous_paused = self
            .map_components
            .get(&uid)
            .map(|map| map.map_paused || map.map_pre_init);
        let current_paused = {
            let Some(map) = self.map_components.get_mut(&uid) else {
                return false;
            };
            if map.map_paused == paused {
                return false;
            }
            map.map_paused = paused;
            map.base.last_modified_tick = self.current_tick;
            map.map_paused || map.map_pre_init
        };
        self.dirty_entity(uid);
        self.sync_map_pause_recursive(uid);
        if previous_paused != Some(current_paused) {
            self.queue_entity_event(crate::MapPausedEvent {
                entity: uid,
                paused: current_paused,
            });
        }
        true
    }

    pub fn run_map_init(&mut self, uid: EntityUid) -> bool {
        let Some(meta) = self.metadata.get(&uid) else {
            return false;
        };
        if !meta.entity_initialized()
            || meta.entity_life_stage == crate::EntityLifeStage::MapInitialized
            || meta.entity_life_stage as i32 >= crate::EntityLifeStage::Terminating as i32
        {
            return false;
        }

        if let Some(meta) = self.metadata.get_mut(&uid) {
            meta.entity_life_stage = crate::EntityLifeStage::MapInitialized;
            meta.dirty(self.current_tick);
        }
        self.queue_entity_event(crate::MapInitEvent { entity: uid });
        true
    }

    fn run_map_init_if_current_map_initialized(&mut self, uid: EntityUid) -> bool {
        let map_id = self.map_id_for(uid);
        if map_id == MapId::NULLSPACE || !self.is_map_initialized(map_id) {
            return false;
        }
        self.run_map_init(uid)
    }

    pub fn set_map_pre_init(&mut self, uid: EntityUid, pre_init: bool) -> bool {
        let previous_paused = self
            .map_components
            .get(&uid)
            .map(|map| map.map_paused || map.map_pre_init);
        let current_paused = {
            let Some(map) = self.map_components.get_mut(&uid) else {
                return false;
            };
            if map.map_pre_init == pre_init {
                return false;
            }
            map.map_pre_init = pre_init;
            map.base.last_modified_tick = self.current_tick;
            map.map_paused || map.map_pre_init
        };
        self.dirty_entity(uid);
        self.sync_map_pause_recursive(uid);
        if previous_paused != Some(current_paused) {
            self.queue_entity_event(crate::MapPausedEvent {
                entity: uid,
                paused: current_paused,
            });
        }
        if !pre_init {
            self.run_map_init_recursive(uid);
        }
        true
    }

    pub fn is_entity_paused(&self, uid: EntityUid) -> bool {
        self.metadata
            .get(&uid)
            .map(|meta| meta.entity_paused)
            .unwrap_or(false)
    }

    pub fn is_map_paused(&self, map_id: MapId) -> bool {
        self.map_entity_for(map_id)
            .and_then(|uid| self.map_components.get(&uid))
            .map(|map| map.map_paused || map.map_pre_init)
            .unwrap_or(false)
    }

    pub fn is_map_initialized(&self, map_id: MapId) -> bool {
        self.map_entity_for(map_id)
            .and_then(|uid| self.map_components.get(&uid))
            .map(|map| !map.map_pre_init)
            .unwrap_or(map_id == MapId::NULLSPACE)
    }

    pub fn delete_entity(
        &mut self,
        uid: EntityUid,
    ) -> Option<(EntityTerminatingEvent, EntityDeletedMessage)> {
        let meta = self.metadata.get(&uid)?;
        if meta.entity_deleted() || meta.entity_life_stage == crate::EntityLifeStage::Terminating {
            return None;
        }

        if let Some(meta) = self.metadata.get_mut(&uid) {
            meta.entity_life_stage = crate::EntityLifeStage::Terminating;
            meta.dirty(self.current_tick);
        }
        let terminate = EntityTerminatingEvent { entity: uid };
        self.queue_entity_event(terminate);

        let children = self.children_of(uid);
        for child in children {
            let _ = self.delete_entity(child);
        }

        let _ = self.remove_joint_component(uid);
        self.clear_transform_runtime_state(uid);

        for component_name in Self::DELETE_COMPONENT_ORDER {
            if component_name == "MetaDataComponent" {
                if let Some(meta) = self.metadata.get_mut(&uid) {
                    meta.entity_life_stage = crate::EntityLifeStage::Deleted;
                    meta.dirty(self.current_tick);
                }
            }
            self.remove_component_storage_with_lifecycle(uid, component_name);
        }
        self.map_grids.remove(&uid);
        self.map_grid_components.remove(&uid);
        self.components.remove(&uid);
        self.entities.remove(&uid);
        let deleted = EntityDeletedMessage { entity: uid };
        self.queue_entity_event(deleted);
        Some((terminate, deleted))
    }

    pub fn queue_delete_entity(&mut self, uid: EntityUid) {
        if self.queued_deletions_set.insert(uid) {
            self.queued_deletions.push_back(uid);
        }
    }

    pub fn flush_queued_deletions(&mut self) {
        while let Some(uid) = self.queued_deletions.pop_front() {
            let _ = self.delete_entity(uid);
        }
        self.queued_deletions_set.clear();
    }

    pub fn entity_exists(&self, uid: EntityUid) -> bool {
        self.metadata.contains_key(&uid)
    }

    pub fn map_entity_for(&self, map_id: MapId) -> Option<EntityUid> {
        self.map_components
            .iter()
            .find_map(|(uid, component)| (component.world_map == map_id).then_some(*uid))
    }

    pub fn grid_entity_for(&self, grid_id: crate::GridId) -> Option<EntityUid> {
        self.map_grid_components
            .iter()
            .find_map(|(uid, component)| (component.grid_index == grid_id).then_some(*uid))
    }

    pub fn grid_exists(&self, grid_id: crate::GridId) -> bool {
        self.grid_entity_for(grid_id).is_some()
    }

    fn touch_lookup_component_runtime(&mut self, owner: EntityUid) {
        if let Some(lookup) = self.entity_lookups.get_mut(&owner) {
            lookup.base.last_modified_tick = self.current_tick;
        }
        if self.entity_exists(owner) {
            self.dirty_entity(owner);
        }
    }

    fn remove_from_snap_grids(&mut self, entity: EntityUid) {
        let grid_owners = self.map_grids.keys().copied().collect::<Vec<_>>();
        for owner in grid_owners {
            let (chunk_indices, chunk_size) = self
                .map_grids
                .get(&owner)
                .map(|grid| (grid.chunk_indices(), grid.chunk_size))
                .unwrap_or_default();
            if let Some(grid) = self.map_grids.get_mut(&owner) {
                for chunk_index in chunk_indices {
                    let cells = (0..chunk_size)
                        .flat_map(|x| (0..chunk_size).map(move |y| (x, y)))
                        .collect::<Vec<_>>();
                    for (x, y) in cells {
                        grid.remove_from_snap_grid_cell(
                            chunk_index * chunk_size as i32 + Vector2i::new(x as i32, y as i32),
                            entity,
                        );
                    }
                }
            }
        }
    }

    fn lookup_anchor_registration(
        &self,
        entity: EntityUid,
        xform: &TransformComponent,
    ) -> Option<(EntityUid, Vector2i)> {
        if !xform.anchored {
            return None;
        }

        let grid_uid = if self.map_grids.contains_key(&xform.parent) {
            xform.parent
        } else if xform.grid_id.is_valid() {
            self.grid_entity_for(xform.grid_id)?
        } else {
            return None;
        };

        let world = self.world_transform(entity)?;
        let grid = self.map_grids.get(&grid_uid)?;
        Some((grid_uid, grid.world_to_tile(world.world_position)))
    }

    fn lookup_local_bounds(&self, grid_indices: Vector2i, tile_size: u16) -> Box2 {
        Box2::new(
            grid_indices.x as f32 * tile_size as f32,
            grid_indices.y as f32 * tile_size as f32,
            (grid_indices.x + 1) as f32 * tile_size as f32,
            (grid_indices.y + 1) as f32 * tile_size as f32,
        )
    }

    fn lookup_owner_for_transform(
        &self,
        entity: EntityUid,
        xform: &TransformComponent,
    ) -> Option<EntityUid> {
        if self.map_components.contains_key(&entity) || self.map_grids.contains_key(&entity) {
            return None;
        }

        if xform.parent.is_valid() {
            return Some(xform.parent);
        }

        let mover = self.mover_coordinates_for_transform(xform);
        Some(mover.entity_id)
    }

    pub fn refresh_entity_lookup(&mut self, uid: EntityUid) -> bool {
        let Some(xform) = self.transforms.get(&uid).cloned() else {
            return false;
        };
        self.remove_lookup_from_entity_tree(uid, false);
        if xform.anchored {
            let Some((grid_uid, tile)) = self.lookup_anchor_registration(uid, &xform) else {
                return false;
            };
            return self
                .map_grids
                .get_mut(&grid_uid)
                .map(|grid| grid.add_to_snap_grid_cell(tile, uid))
                .unwrap_or(false);
        }
        let Some(lookup_owner) = self.lookup_owner_for_transform(uid, &xform) else {
            return false;
        };
        let Some(aabb) = self.entity_world_aabb(uid) else {
            return false;
        };
        let changed = {
            let lookup = self.ensure_lookup(lookup_owner);
            if lookup.entities.get(&uid).copied() == Some(aabb) {
                false
            } else {
                lookup.add_or_update(uid, aabb);
                true
            }
        };
        if changed {
            self.touch_lookup_component_runtime(lookup_owner);
        }
        true
    }

    pub fn refresh_entity_lookup_subtree(&mut self, uid: EntityUid) {
        let mut stack = vec![uid];
        while let Some(next_uid) = stack.pop() {
            let _ = self.refresh_entity_lookup(next_uid);
            stack.extend(self.children_of(next_uid));
        }
    }

    pub fn remove_lookup_from_entity_tree(&mut self, entity: EntityUid, recursive: bool) {
        self.remove_from_snap_grids(entity);

        let owners = self.entity_lookups.keys().copied().collect::<Vec<_>>();
        let mut touched = Vec::new();
        for owner in owners {
            let removed = self
                .entity_lookups
                .get_mut(&owner)
                .map(|lookup| lookup.entities.remove(&entity).is_some())
                .unwrap_or(false);
            if removed {
                touched.push(owner);
            }
        }
        for owner in touched {
            self.touch_lookup_component_runtime(owner);
        }

        if recursive {
            for child in self.children_of(entity) {
                self.remove_lookup_from_entity_tree(child, true);
            }
        }
    }

    pub fn refresh_entity_runtime(&mut self, uid: EntityUid) -> bool {
        let updated_lookup = self.refresh_entity_lookup(uid);
        let map_id = self
            .transforms
            .get(&uid)
            .map(|transform| transform.map_id)
            .unwrap_or(MapId::NULLSPACE);
        if map_id != MapId::NULLSPACE {
            self.refresh_map_physics_runtime_many([map_id]);
        }
        updated_lookup || map_id != MapId::NULLSPACE
    }

    pub fn reconcile_transform_runtime(&mut self, uid: EntityUid, previous_map: Option<MapId>) {
        self.sync_entity_metadata_map_id(uid);
        self.sync_entity_pause_from_current_map(uid);
        let _ = self.sync_map_grid_runtime(uid);
        self.refresh_entity_lookup_subtree(uid);
        let _ = self.run_map_init_if_current_map_initialized(uid);

        let current_map = self
            .transforms
            .get(&uid)
            .map(|transform| transform.map_id)
            .unwrap_or(MapId::NULLSPACE);
        let map_ids = previous_map
            .into_iter()
            .chain(std::iter::once(current_map))
            .filter(|map_id| *map_id != MapId::NULLSPACE)
            .collect::<HashSet<_>>();
        self.refresh_map_physics_runtime_many(map_ids);
    }

    fn refresh_transform_map_runtime(&mut self, uid: EntityUid, previous_map: MapId) {
        self.reconcile_transform_runtime(uid, Some(previous_map));
    }

    fn apply_transform_move_event(&mut self, event: &mut crate::MoveEvent) {
        self.raise_component_event(event.sender, "TransformComponent", event);
    }

    pub fn apply_transform_move_event_and_reconcile(
        &mut self,
        uid: EntityUid,
        previous_map: MapId,
        event: &mut crate::MoveEvent,
    ) {
        self.apply_transform_move_event(event);
        self.refresh_transform_map_runtime(uid, previous_map);
    }

    pub fn defer_move_event(&mut self, move_event: crate::MoveEvent) {
        if self.map_grids.contains_key(&move_event.sender) {
            self.deferred_grid_moves.push_back(move_event);
        } else {
            self.deferred_other_moves.push_back(move_event);
        }
    }

    pub fn process_deferred_move_events(&mut self) -> Vec<crate::MoveEvent> {
        let mut processed = Vec::new();
        let mut grid_moves = std::mem::take(&mut self.deferred_grid_moves);
        let mut other_moves = std::mem::take(&mut self.deferred_other_moves);
        Self::process_deferred_move_queue(&mut grid_moves, self, &mut processed);
        Self::process_deferred_move_queue(&mut other_moves, self, &mut processed);
        processed
    }

    fn process_deferred_move_queue(
        queue: &mut VecDeque<crate::MoveEvent>,
        manager: &EntityManager,
        processed: &mut Vec<crate::MoveEvent>,
    ) {
        while let Some(event) = queue.pop_front() {
            if manager.deleted(event.sender) || !event.new_position.is_valid(manager) {
                continue;
            }
            processed.push(event);
        }
    }

    fn touch_transform_runtime(&mut self, uid: EntityUid) -> bool {
        let Some(transform) = self.transforms.get_mut(&uid) else {
            return false;
        };
        transform.base.last_modified_tick = self.current_tick;
        self.dirty_entity(uid);
        true
    }

    pub fn set_local_transform_deferred(
        &mut self,
        uid: EntityUid,
        local_position: Vector2,
        local_rotation: keisan::Angle,
    ) -> Option<crate::MoveEvent> {
        let event = self.set_local_transform(uid, local_position, local_rotation)?;
        let _ = self.touch_transform_runtime(uid);
        Some(event)
    }

    pub fn offset_local_transform_deferred(
        &mut self,
        uid: EntityUid,
        delta: Vector2,
        angular_delta: keisan::Angle,
    ) -> Option<crate::MoveEvent> {
        let event = self.offset_local_transform(uid, delta, angular_delta)?;
        let _ = self.touch_transform_runtime(uid);
        Some(event)
    }

    pub fn apply_transform_state_with_move_event(
        &mut self,
        uid: EntityUid,
        state: TransformComponentState,
    ) -> Option<crate::MoveEvent> {
        let old_position = self
            .transforms
            .get(&uid)
            .map(|transform| transform.coordinates())?;
        if !self.apply_transform_state(uid, state) {
            return None;
        }
        let _ = self.touch_transform_runtime(uid);
        let new_position = self
            .transforms
            .get(&uid)
            .map(|transform| transform.coordinates())?;
        Some(crate::MoveEvent {
            sender: uid,
            old_position,
            new_position,
        })
    }

    pub fn set_anchored_with_move_event(
        &mut self,
        uid: EntityUid,
        anchored: bool,
    ) -> Option<crate::MoveEvent> {
        let old_position = self
            .transforms
            .get(&uid)
            .filter(|transform| transform.anchored != anchored)
            .map(|transform| transform.coordinates())?;
        if !self.set_anchored(uid, anchored) {
            return None;
        }
        let _ = self.touch_transform_runtime(uid);
        let new_position = self
            .transforms
            .get(&uid)
            .map(|transform| transform.coordinates())?;
        Some(crate::MoveEvent {
            sender: uid,
            old_position,
            new_position,
        })
    }

    pub fn set_local_transform_immediate(
        &mut self,
        uid: EntityUid,
        local_position: Vector2,
        local_rotation: keisan::Angle,
    ) -> bool {
        let previous_map = self
            .transforms
            .get(&uid)
            .map(|transform| transform.map_id)
            .unwrap_or(MapId::NULLSPACE);
        if let Some(mut event) = self.set_local_transform(uid, local_position, local_rotation) {
            self.apply_transform_move_event_and_reconcile(uid, previous_map, &mut event);
            return true;
        }
        false
    }

    pub fn offset_local_transform_immediate(
        &mut self,
        uid: EntityUid,
        delta: Vector2,
        angular_delta: keisan::Angle,
    ) -> bool {
        let previous_map = self
            .transforms
            .get(&uid)
            .map(|transform| transform.map_id)
            .unwrap_or(MapId::NULLSPACE);
        if let Some(mut event) = self.offset_local_transform(uid, delta, angular_delta) {
            self.apply_transform_move_event_and_reconcile(uid, previous_map, &mut event);
            return true;
        }
        false
    }

    pub fn mutate_transform_and_reconcile<F>(&mut self, uid: EntityUid, mutate: F) -> bool
    where
        F: FnOnce(&mut TransformComponent),
    {
        let Some(previous_map) = self.transforms.get(&uid).map(|transform| transform.map_id) else {
            return false;
        };

        if let Some(transform) = self.transforms.get_mut(&uid) {
            mutate(transform);
            transform.rebuild_for_manager();
        } else {
            return false;
        }

        self.reconcile_transform_runtime(uid, Some(previous_map));
        true
    }

    pub fn mutate_physics_and_reconcile<F>(&mut self, uid: EntityUid, mutate: F) -> bool
    where
        F: FnOnce(&mut crate::PhysicsComponent),
    {
        let physics = self.ensure_physics(uid);
        mutate(physics);
        self.reconcile_entity_physics_runtime(uid);
        true
    }

    pub fn configure_physics_body(
        &mut self,
        uid: EntityUid,
        body_type: Option<crate::BodyType>,
        awake: Option<bool>,
        can_collide: Option<bool>,
        predict: Option<bool>,
    ) -> bool {
        self.mutate_physics_and_reconcile(uid, |body| {
            if let Some(body_type) = body_type {
                body.set_body_type(body_type);
            }
            if let Some(awake) = awake {
                body.awake = awake;
            }
            if let Some(can_collide) = can_collide {
                body.can_collide = can_collide;
            }
            if let Some(predict) = predict {
                body.predict = predict;
            }
        })
    }

    pub fn refresh_entities_runtime(&mut self, entities: &[EntityUid]) -> usize {
        let mut updated = 0;
        let mut maps = HashSet::new();
        for &uid in entities {
            if self.refresh_entity_lookup(uid) {
                updated += 1;
            }
            if let Some(map_id) = self.transforms.get(&uid).map(|transform| transform.map_id) {
                if map_id != MapId::NULLSPACE {
                    maps.insert(map_id);
                }
            }
        }
        self.refresh_map_physics_runtime_many(maps);
        updated
    }

    pub fn grid_world_position(&self, grid_id: crate::GridId) -> Option<Vector2> {
        let uid = self.grid_entity_for(grid_id)?;
        self.map_grids.get(&uid).map(|grid| grid.world_position)
    }

    pub fn grid_world_bounds(&self, grid_id: crate::GridId) -> Option<Box2> {
        let uid = self.grid_entity_for(grid_id)?;
        self.map_grids.get(&uid).map(|grid| grid.world_bounds())
    }

    pub fn try_find_grid_at(&self, map_id: MapId, world_pos: Vector2) -> Option<crate::GridId> {
        self.map_grids
            .values()
            .find(|grid| {
                grid.parent_map_id == map_id
                    && grid.collides_with_grid(grid.world_to_tile(world_pos))
            })
            .map(|grid| grid.index)
    }

    pub fn find_grids_intersecting(
        &self,
        map_id: MapId,
        world_aabb: Box2,
        approx: bool,
    ) -> Vec<crate::GridId> {
        let mut grids = Vec::new();

        for grid in self.map_grids.values() {
            if grid.parent_map_id != map_id {
                continue;
            }

            if !grid.world_bounds().intersects(world_aabb) {
                continue;
            }

            if approx {
                grids.push(grid.index);
                continue;
            }

            let chunk_hit = grid
                .get_map_chunks_intersecting(world_aabb)
                .next()
                .is_some();
            if chunk_hit
                || (grid.chunk_count() == 0 && world_aabb.contains(grid.world_position, true))
            {
                grids.push(grid.index);
            }
        }

        grids
    }

    pub fn tile_ref(&self, grid_id: crate::GridId, tile_indices: Vector2i) -> Option<TileRef> {
        let uid = self.grid_entity_for(grid_id)?;
        self.map_grids
            .get(&uid)
            .map(|grid| grid.get_tile_ref(tile_indices))
    }

    pub fn tile_world_bounds(
        &self,
        grid_id: crate::GridId,
        tile_indices: Vector2i,
    ) -> Option<Box2> {
        let tile_ref = self.tile_ref(grid_id, tile_indices)?;
        let grid = self
            .map_grids
            .values()
            .find(|grid| grid.index == tile_ref.grid_index)?;
        let center = grid.grid_tile_to_world_pos(tile_ref.grid_indices);
        let size = Vector2::new(grid.tile_size as f32, grid.tile_size as f32);
        Some(Box2::centered_around(center, size))
    }

    pub fn entities_at_tile(
        &self,
        grid_id: crate::GridId,
        tile_indices: Vector2i,
    ) -> Vec<EntityUid> {
        let Some(grid_uid) = self.grid_entity_for(grid_id) else {
            return Vec::new();
        };
        let Some(grid) = self.map_grids.get(&grid_uid) else {
            return Vec::new();
        };

        let lookup = self.entity_lookups.get(&grid.grid_entity_id);
        let local = self.lookup_local_bounds(tile_indices, grid.tile_size);
        let world = Box2::from_corners(
            grid.local_to_world(local.bottom_left()),
            grid.local_to_world(local.top_right()),
        );
        let mut results = Vec::new();

        if let Some(lookup) = lookup {
            results.extend(
                lookup
                    .entities
                    .iter()
                    .filter_map(|(uid, bounds)| bounds.intersects(world).then_some(*uid)),
            );
        }

        results.extend(grid.get_anchored_entities(tile_indices));
        results.sort();
        results.dedup();
        results
    }

    pub fn entities_at_tiles<I>(&self, grid_id: crate::GridId, tile_indices: I) -> Vec<EntityUid>
    where
        I: IntoIterator<Item = Vector2i>,
    {
        let mut results = Vec::new();
        for tile in tile_indices {
            results.extend(self.entities_at_tile(grid_id, tile));
        }
        results.sort();
        results.dedup();
        results
    }

    pub fn entities_in_grid_aabb(
        &self,
        grid_id: crate::GridId,
        world_aabb: Box2,
        include_anchored: bool,
    ) -> Vec<EntityUid> {
        let Some(grid_uid) = self.grid_entity_for(grid_id) else {
            return Vec::new();
        };
        self.entities_in_lookup_aabb(grid_uid, world_aabb, include_anchored)
    }

    pub fn entities_in_lookup_aabb(
        &self,
        lookup_owner: EntityUid,
        world_aabb: Box2,
        include_anchored: bool,
    ) -> Vec<EntityUid> {
        let mut results = self
            .entity_lookups
            .get(&lookup_owner)
            .map(|lookup| {
                lookup
                    .entities
                    .iter()
                    .filter_map(|(uid, bounds)| bounds.intersects(world_aabb).then_some(*uid))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if include_anchored {
            if let Some(grid) = self.map_grids.get(&lookup_owner) {
                let tile_hits = grid
                    .get_map_chunks_intersecting(world_aabb)
                    .flat_map(|chunk_index| {
                        let Some(chunk) = grid.try_get_chunk(chunk_index) else {
                            return Vec::new().into_iter();
                        };
                        let mut anchored = Vec::new();
                        for x in 0..chunk.chunk_size() {
                            for y in 0..chunk.chunk_size() {
                                let grid_tile = chunk
                                    .chunk_tile_to_grid_tile(Vector2i::new(x as i32, y as i32));
                                let bounds = self.lookup_local_bounds(grid_tile, grid.tile_size);
                                let world = Box2::from_corners(
                                    grid.local_to_world(bounds.bottom_left()),
                                    grid.local_to_world(bounds.top_right()),
                                );
                                if !world.intersects(world_aabb) {
                                    continue;
                                }
                                anchored.extend(chunk.get_snap_grid_cell(x, y).iter().copied());
                            }
                        }
                        anchored.into_iter()
                    })
                    .collect::<Vec<_>>();
                results.extend(tile_hits);
            }
        }

        results.sort();
        results.dedup();
        results
    }

    pub fn entity_world_aabb(&self, entity: EntityUid) -> Option<Box2> {
        if let Some(body_bounds) = self
            .physics
            .contains_key(&entity)
            .then(|| self.physics_world_aabb(entity))
            .flatten()
        {
            return Some(body_bounds);
        }

        let xform = self.transforms.get(&entity)?;
        let world = self.world_transform(entity)?;
        let point = world.world_position;
        let radius = if xform.anchored { 0.0 } else { 0.5 };
        Some(Box2::new(
            point.x - radius,
            point.y - radius,
            point.x + radius,
            point.y + radius,
        ))
    }

    pub fn world_position(&self, entity: EntityUid) -> Option<Vector2> {
        self.world_transform(entity)
            .map(|world| world.world_position)
    }

    pub fn local_position(&self, entity: EntityUid) -> Option<Vector2> {
        self.transforms
            .get(&entity)
            .map(|transform| transform.local_position)
    }

    pub fn local_rotation(&self, entity: EntityUid) -> Option<keisan::Angle> {
        self.transforms
            .get(&entity)
            .map(|transform| transform.local_rotation)
    }

    pub fn world_rotation(&self, entity: EntityUid) -> Option<keisan::Angle> {
        self.world_transform(entity)
            .map(|world| world.world_rotation)
    }

    pub fn world_matrix(&self, entity: EntityUid) -> Option<keisan::Matrix3> {
        self.world_transform(entity).map(|world| world.world_matrix)
    }

    pub fn inv_world_matrix(&self, entity: EntityUid) -> Option<keisan::Matrix3> {
        self.world_transform(entity)
            .map(|world| world.inv_world_matrix)
    }

    pub fn mover_coordinates_for_transform(
        &self,
        xform: &crate::TransformComponent,
    ) -> crate::EntityCoordinates {
        if let Some(grid) = xform
            .grid_id
            .is_valid()
            .then(|| self.grid_entity_for(xform.grid_id))
            .flatten()
            .and_then(|uid| self.map_grids.get(&uid))
        {
            if grid.grid_entity_id == xform.parent {
                return xform.coordinates();
            }

            let world = xform.map_position(self);
            let grid_pos = grid.inv_world_matrix() * world.position;
            return crate::EntityCoordinates::new(grid.grid_entity_id, grid_pos);
        }

        let map_uid = self.map_entity_for(xform.map_id).unwrap_or(xform.parent);

        if xform.parent == map_uid {
            return xform.coordinates();
        }

        crate::EntityCoordinates::new(map_uid, xform.map_position(self).position)
    }

    pub fn mover_coordinates(&self, entity: EntityUid) -> Option<crate::EntityCoordinates> {
        let xform = self.transforms.get(&entity)?;
        Some(self.mover_coordinates_for_transform(xform))
    }

    pub fn query_aabb_entities(
        &self,
        broadphase_owner: EntityUid,
        world_aabb: Box2,
    ) -> Vec<EntityUid> {
        Self::shared_physics().query_aabb_entities(self, broadphase_owner, world_aabb)
    }

    pub fn physics_world_aabb(&self, entity: EntityUid) -> Option<Box2> {
        Self::shared_physics().get_world_aabb(self, entity)
    }

    #[cfg(test)]
    fn map_linear_velocity(&self, uid: EntityUid) -> Vector2 {
        self.map_velocities(uid).0
    }

    #[cfg(test)]
    fn map_angular_velocity(&self, uid: EntityUid) -> f32 {
        self.map_velocities(uid).1
    }

    fn map_velocities(&self, uid: EntityUid) -> (Vector2, f32) {
        Self::shared_physics().get_map_velocities(self, uid)
    }

    pub fn step_physics_body(
        &mut self,
        uid: EntityUid,
        frame_time: f32,
    ) -> Option<crate::PhysicsStepState> {
        Self::shared_physics().step_body(self, uid, frame_time)
    }

    pub fn refresh_broadphase_runtime(
        &mut self,
        broadphase_owner: EntityUid,
        bodies: &[EntityUid],
    ) -> usize {
        Self::shared_physics().sync_broadphase(self, broadphase_owner, bodies)
    }

    pub fn refresh_contact_runtime(&mut self, map_owner: EntityUid, bodies: &[EntityUid]) -> usize {
        Self::shared_physics().sync_contacts(self, map_owner, bodies)
    }

    pub fn intersect_ray_on(
        &self,
        broadphase_owner: EntityUid,
        ray: CollisionRay,
        max_length: f32,
        return_on_first_hit: bool,
    ) -> Vec<PhysicsQueryHit> {
        Self::shared_physics().intersect_ray(
            self,
            broadphase_owner,
            ray,
            max_length,
            return_on_first_hit,
        )
    }

    pub fn entities_in_map_aabb(&self, map_id: MapId, world_aabb: Box2) -> Vec<EntityUid> {
        let Some(map_owner) = self.map_entity_for(map_id) else {
            return Vec::new();
        };
        self.query_aabb_entities(map_owner, world_aabb)
    }

    pub fn has_map_broadphase(&self, map_id: MapId) -> bool {
        self.map_entity_for(map_id)
            .is_some_and(|owner| self.broadphases.contains_key(&owner))
    }

    #[cfg(test)]
    fn has_broadphase_owner(&self, owner: EntityUid) -> bool {
        self.broadphases.contains_key(&owner)
    }

    pub fn has_map_physics_runtime(&self, map_id: MapId) -> bool {
        self.map_entity_for(map_id)
            .is_some_and(|owner| self.physics_maps.contains_key(&owner))
    }

    pub fn has_physics_runtime_owner(&self, owner: EntityUid) -> bool {
        self.physics_maps.contains_key(&owner)
    }

    pub fn ensure_map_physics_runtime(&mut self, map_id: MapId) -> Option<EntityUid> {
        let owner = self.map_entity_for(map_id)?;
        self.ensure_broadphase(owner);
        self.ensure_physics_map(owner);
        Some(owner)
    }

    pub fn map_contact_count(&self, map_id: MapId) -> usize {
        self.map_entity_for(map_id)
            .and_then(|owner| self.physics_maps.get(&owner))
            .map(|map| map.contact_count())
            .unwrap_or(0)
    }

    #[cfg(test)]
    pub(crate) fn owner_contact_count(&self, owner: EntityUid) -> usize {
        self.physics_maps
            .get(&owner)
            .map(|map| map.contact_count())
            .unwrap_or(0)
    }

    pub fn map_contacts_snapshot(&self, map_id: MapId) -> Vec<butsuri::Contact> {
        self.map_entity_for(map_id)
            .and_then(|owner| self.physics_maps.get(&owner))
            .map(|map| map.contacts().to_vec())
            .unwrap_or_default()
    }

    #[cfg(test)]
    pub(crate) fn owner_contacts_snapshot(&self, owner: EntityUid) -> Vec<butsuri::Contact> {
        self.physics_maps
            .get(&owner)
            .map(|map| map.contacts().to_vec())
            .unwrap_or_default()
    }

    pub fn map_has_touching_contact(&self, map_id: MapId) -> bool {
        self.map_entity_for(map_id)
            .and_then(|owner| self.physics_maps.get(&owner))
            .is_some_and(|map| map.contacts().iter().any(|contact| contact.is_touching))
    }

    pub fn drain_owner_contact_events(
        &mut self,
        owner: EntityUid,
    ) -> Vec<crate::PhysicsContactEvent> {
        self.physics_maps
            .get_mut(&owner)
            .map(|map| map.drain_contact_events())
            .unwrap_or_default()
    }

    pub fn map_contains_body(&self, map_id: MapId, uid: EntityUid) -> bool {
        self.map_entity_for(map_id)
            .and_then(|owner| self.physics_maps.get(&owner))
            .is_some_and(|map| map.bodies.contains(&uid))
    }

    pub fn owner_contains_body(&self, owner: EntityUid, uid: EntityUid) -> bool {
        self.physics_maps
            .get(&owner)
            .is_some_and(|map| map.bodies.contains(&uid))
    }

    pub fn map_contains_awake_body(&self, map_id: MapId, uid: EntityUid) -> bool {
        self.map_entity_for(map_id)
            .and_then(|owner| self.physics_maps.get(&owner))
            .is_some_and(|map| map.awake_bodies.contains(&uid))
    }

    pub fn owner_contains_awake_body(&self, owner: EntityUid, uid: EntityUid) -> bool {
        self.physics_maps
            .get(&owner)
            .is_some_and(|map| map.awake_bodies.contains(&uid))
    }

    pub fn map_gravity(&self, map_id: MapId) -> Option<Vector2> {
        self.map_entity_for(map_id)
            .and_then(|owner| self.physics_maps.get(&owner))
            .map(|map| map.gravity)
    }

    pub fn map_auto_clear_forces(&self, map_id: MapId) -> Option<bool> {
        self.map_entity_for(map_id)
            .and_then(|owner| self.physics_maps.get(&owner))
            .map(|map| map.auto_clear_forces)
    }

    pub fn owner_auto_clear_forces(&self, owner: EntityUid) -> Option<bool> {
        self.physics_maps
            .get(&owner)
            .map(|map| map.auto_clear_forces)
    }

    pub fn map_physics_runtime_last_modified(&self, map_id: MapId) -> Option<jikan::GameTick> {
        self.map_entity_for(map_id)
            .and_then(|owner| self.physics_maps.get(&owner))
            .map(|map| map.base.last_modified_tick)
    }

    pub fn map_broadphase_last_modified(&self, map_id: MapId) -> Option<jikan::GameTick> {
        self.map_entity_for(map_id)
            .and_then(|owner| self.broadphases.get(&owner))
            .map(|broadphase| broadphase.base.last_modified_tick)
    }

    pub fn set_map_gravity(&mut self, map_id: MapId, gravity: Vector2) -> bool {
        let Some(owner) = self.map_entity_for(map_id) else {
            return false;
        };
        let tick = self.current_tick;
        let physics_map = self.ensure_physics_map(owner);
        if physics_map.gravity == gravity {
            return false;
        }
        physics_map.gravity = gravity;
        physics_map.base.last_modified_tick = tick;
        if self.entity_exists(owner) {
            self.dirty_entity(owner);
        }
        true
    }

    pub fn set_map_auto_clear_forces(&mut self, map_id: MapId, auto_clear_forces: bool) -> bool {
        let Some(owner) = self.map_entity_for(map_id) else {
            return false;
        };
        let tick = self.current_tick;
        let physics_map = self.ensure_physics_map(owner);
        if physics_map.auto_clear_forces == auto_clear_forces {
            return false;
        }
        physics_map.auto_clear_forces = auto_clear_forces;
        physics_map.base.last_modified_tick = tick;
        if self.entity_exists(owner) {
            self.dirty_entity(owner);
        }
        true
    }

    pub fn entity_map_velocities(&self, uid: EntityUid) -> Option<(Vector2, f32)> {
        self.physics
            .contains_key(&uid)
            .then(|| self.map_velocities(uid))
    }

    pub fn entity_map_linear_velocity(&self, uid: EntityUid) -> Option<Vector2> {
        self.entity_map_velocities(uid).map(|(linear, _)| linear)
    }

    pub fn entity_map_angular_velocity(&self, uid: EntityUid) -> Option<f32> {
        self.entity_map_velocities(uid).map(|(_, angular)| angular)
    }

    pub fn intersect_ray(
        &self,
        map_id: MapId,
        ray: CollisionRay,
        max_length: f32,
        return_on_first_hit: bool,
    ) -> Vec<PhysicsQueryHit> {
        let Some(map_owner) = self.map_entity_for(map_id) else {
            return Vec::new();
        };
        self.intersect_ray_on(map_owner, ray, max_length, return_on_first_hit)
    }

    pub fn drain_map_contact_events(&mut self, map_id: MapId) -> Vec<crate::PhysicsContactEvent> {
        let Some(map_owner) = self.map_entity_for(map_id) else {
            return Vec::new();
        };
        self.physics_maps
            .get_mut(&map_owner)
            .map(|map| map.drain_contact_events())
            .unwrap_or_default()
    }

    pub fn drain_map_runtime_events(&mut self, map_id: MapId) -> Vec<crate::PhysicsRuntimeEvent> {
        let Some(map_owner) = self.map_entity_for(map_id) else {
            return Vec::new();
        };
        self.physics_maps
            .get_mut(&map_owner)
            .map(|map| map.drain_runtime_events())
            .unwrap_or_default()
    }

    pub fn map_id_for(&self, uid: EntityUid) -> MapId {
        self.map_components
            .get(&uid)
            .map(|component| component.world_map)
            .or_else(|| self.transforms.get(&uid).map(|transform| transform.map_id))
            .unwrap_or(MapId::NULLSPACE)
    }

    pub fn grid_id_for(&self, uid: EntityUid) -> crate::GridId {
        self.map_grid_components
            .get(&uid)
            .map(|component| component.grid_index)
            .or_else(|| self.transforms.get(&uid).map(|transform| transform.grid_id))
            .unwrap_or(crate::GridId::INVALID)
    }

    pub fn is_map_entity(&self, uid: EntityUid) -> bool {
        self.map_components.contains_key(&uid)
    }

    pub fn is_grid_entity(&self, uid: EntityUid) -> bool {
        self.map_grid_components.contains_key(&uid)
    }

    pub fn is_grid_paused(&self, grid_id: crate::GridId) -> bool {
        self.grid_entity_for(grid_id)
            .map(|uid| self.map_id_for(uid))
            .map(|map_id| self.is_map_paused(map_id))
            .unwrap_or(true)
    }

    pub fn entity_uids(&self, include_paused: bool) -> Vec<EntityUid> {
        self.entities
            .iter()
            .copied()
            .filter(|uid| self.include_entity_in_query(*uid, include_paused))
            .collect()
    }

    pub fn entities_in_map_radius(
        &self,
        map_id: MapId,
        center: Vector2,
        radius: f32,
        include_paused: bool,
        include_map_entities: bool,
    ) -> Vec<EntityUid> {
        let radius_sq = radius * radius;
        self.entity_uids(include_paused)
            .into_iter()
            .filter(|uid| {
                if include_map_entities
                    && self.is_map_entity(*uid)
                    && self.map_id_for(*uid) == map_id
                {
                    return true;
                }

                self.world_transform(*uid)
                    .map(|world| {
                        world.map_id == map_id
                            && (world.world_position - center).length_squared() <= radius_sq
                    })
                    .unwrap_or(false)
            })
            .collect()
    }

    pub fn deleted(&self, uid: EntityUid) -> bool {
        self.metadata
            .get(&uid)
            .map(|m| m.entity_deleted())
            .unwrap_or(true)
    }

    pub fn children_of(&self, uid: EntityUid) -> Vec<EntityUid> {
        self.transforms
            .get(&uid)
            .map(|transform| transform.children.iter().copied().collect())
            .unwrap_or_default()
    }

    fn include_entity_in_query(&self, uid: EntityUid, include_paused: bool) -> bool {
        let Some(meta) = self.metadata.get(&uid) else {
            return false;
        };
        if meta.entity_deleted() {
            return false;
        }
        include_paused || !meta.entity_paused
    }

    pub fn entity_query<'a, T>(
        &'a self,
        storage: &'a HashMap<EntityUid, T>,
        include_paused: bool,
    ) -> Vec<(EntityUid, &'a T)> {
        storage
            .iter()
            .filter_map(|(uid, component)| {
                self.include_entity_in_query(*uid, include_paused)
                    .then_some((*uid, component))
            })
            .collect()
    }

    pub fn entity_query2<'a, TComp1, TComp2>(
        &'a self,
        storage1: &'a HashMap<EntityUid, TComp1>,
        storage2: &'a HashMap<EntityUid, TComp2>,
        include_paused: bool,
    ) -> Vec<(EntityUid, &'a TComp1, &'a TComp2)> {
        storage1
            .iter()
            .filter_map(|(uid, comp1)| {
                let comp2 = storage2.get(uid)?;
                self.include_entity_in_query(*uid, include_paused)
                    .then_some((*uid, comp1, comp2))
            })
            .collect()
    }

    pub fn entity_query3<'a, TComp1, TComp2, TComp3>(
        &'a self,
        storage1: &'a HashMap<EntityUid, TComp1>,
        storage2: &'a HashMap<EntityUid, TComp2>,
        storage3: &'a HashMap<EntityUid, TComp3>,
        include_paused: bool,
    ) -> Vec<(EntityUid, &'a TComp1, &'a TComp2, &'a TComp3)> {
        storage1
            .iter()
            .filter_map(|(uid, comp1)| {
                let comp2 = storage2.get(uid)?;
                let comp3 = storage3.get(uid)?;
                self.include_entity_in_query(*uid, include_paused)
                    .then_some((*uid, comp1, comp2, comp3))
            })
            .collect()
    }

    pub fn entity_query4<'a, TComp1, TComp2, TComp3, TComp4>(
        &'a self,
        storage1: &'a HashMap<EntityUid, TComp1>,
        storage2: &'a HashMap<EntityUid, TComp2>,
        storage3: &'a HashMap<EntityUid, TComp3>,
        storage4: &'a HashMap<EntityUid, TComp4>,
        include_paused: bool,
    ) -> Vec<(EntityUid, &'a TComp1, &'a TComp2, &'a TComp3, &'a TComp4)> {
        storage1
            .iter()
            .filter_map(|(uid, comp1)| {
                let comp2 = storage2.get(uid)?;
                let comp3 = storage3.get(uid)?;
                let comp4 = storage4.get(uid)?;
                self.include_entity_in_query(*uid, include_paused)
                    .then_some((*uid, comp1, comp2, comp3, comp4))
            })
            .collect()
    }

    pub fn get_all_components<'a, T>(
        &'a self,
        storage: &'a HashMap<EntityUid, T>,
        include_paused: bool,
    ) -> Vec<&'a T> {
        storage
            .iter()
            .filter_map(|(uid, component)| {
                self.include_entity_in_query(*uid, include_paused)
                    .then_some(component)
            })
            .collect()
    }

    pub fn entity_component_names(&self, uid: EntityUid) -> Vec<String> {
        let mut names = Vec::new();
        for component_name in Self::ENTITY_EVENT_COMPONENT_ORDER {
            if let Some(base) = self.component_base(uid, component_name) {
                names.push(base.name.clone());
            }
        }
        names
    }

    pub fn physics_bodies_in_map(&self, map_id: MapId, include_paused: bool) -> Vec<EntityUid> {
        self.entity_query3(
            &self.physics,
            &self.transforms,
            &self.fixtures,
            include_paused,
        )
        .into_iter()
        .filter_map(|(uid, body, transform, fixtures)| {
            (body.can_collide && transform.map_id == map_id && !fixtures.fixtures.is_empty())
                .then_some(uid)
        })
        .collect()
    }

    pub fn physics_body_entities(&self, include_paused: bool) -> Vec<EntityUid> {
        self.entity_query(&self.physics, include_paused)
            .into_iter()
            .map(|(uid, _)| uid)
            .collect()
    }

    pub fn component_exists_for_net_id(&self, uid: EntityUid, net_id: u16) -> bool {
        match net_id {
            Self::METADATA_NET_ID => self.metadata.contains_key(&uid),
            Self::TRANSFORM_NET_ID => self.transforms.contains_key(&uid),
            Self::MAP_NET_ID => self.map_components.contains_key(&uid),
            Self::MAP_GRID_NET_ID => self.map_grid_components.contains_key(&uid),
            Self::PHYSICS_NET_ID => self.physics.contains_key(&uid),
            Self::APPEARANCE_NET_ID => self.appearances.contains_key(&uid),
            Self::FIXTURES_NET_ID => self.fixtures.contains_key(&uid),
            Self::JOINTS_NET_ID => self.joint_components.contains_key(&uid),
            Self::LOOKUP_NET_ID => self.entity_lookups.contains_key(&uid),
            Self::BROADPHASE_NET_ID => self.broadphases.contains_key(&uid),
            Self::PHYSICS_MAP_NET_ID => self.physics_maps.contains_key(&uid),
            Self::COLLISION_WAKE_NET_ID => self.collision_wakes.contains_key(&uid),
            Self::COLLIDE_ON_ANCHOR_NET_ID => self.collide_on_anchors.contains_key(&uid),
            _ => false,
        }
    }

    pub fn remove_component_by_net_id(&mut self, uid: EntityUid, net_id: u16) -> bool {
        match net_id {
            Self::METADATA_NET_ID => self.remove_metadata_component(uid),
            Self::TRANSFORM_NET_ID => self.remove_transform_component(uid),
            Self::MAP_NET_ID => self.remove_map_component(uid),
            Self::MAP_GRID_NET_ID => self.remove_map_grid_component(uid),
            Self::PHYSICS_NET_ID => self.remove_physics_component(uid),
            Self::APPEARANCE_NET_ID => self.remove_appearance_component(uid),
            Self::FIXTURES_NET_ID => self.remove_fixtures_component(uid),
            Self::JOINTS_NET_ID => self.remove_joint_component(uid),
            Self::LOOKUP_NET_ID => self.remove_lookup_component(uid),
            Self::BROADPHASE_NET_ID => self.remove_broadphase_component(uid),
            Self::PHYSICS_MAP_NET_ID => self.remove_physics_map_component(uid),
            Self::COLLISION_WAKE_NET_ID => self.remove_collision_wake_component(uid),
            Self::COLLIDE_ON_ANCHOR_NET_ID => self.remove_collide_on_anchor_component(uid),
            _ => false,
        }
    }

    pub fn component_dirty_since(component: &Component, from_tick: GameTick) -> bool {
        from_tick == GameTick::ZERO
            || component.creation_tick > from_tick
            || component.last_modified_tick > from_tick
    }

    pub fn component_created_since(component: &Component, from_tick: GameTick) -> bool {
        from_tick == GameTick::ZERO || component.creation_tick > from_tick
    }

    pub fn deleted_component_change(net_id: u16) -> SerializedComponentChange {
        SerializedComponentChange::new(net_id, false, true, None)
    }

    pub fn create_entity_from_serialized_state(
        &mut self,
        serializer: &mut RobustSerializer,
        state: &SerializedEntityState,
    ) -> Option<EntityUid> {
        if self.entity_exists(state.uid) {
            return Some(state.uid);
        }

        let prototype = state
            .component_changes
            .iter()
            .find(|change| change.net_id == Self::METADATA_NET_ID)
            .and_then(|change| change.state.as_ref())
            .and_then(|component| {
                serializer
                    .deserialize_component_state::<crate::MetaDataComponentState>(component)
                    .ok()
            })
            .and_then(|meta| meta.prototype_id);
        let initial_transform = state
            .component_changes
            .iter()
            .find(|change| change.net_id == Self::TRANSFORM_NET_ID && !change.deleted)
            .and_then(|change| change.state.as_ref())
            .and_then(|component| {
                serializer
                    .deserialize_component_state::<TransformComponentState>(component)
                    .ok()
            });

        if let Some(transform) = initial_transform {
            self.alloc_entity_external_with_transform(state.uid, prototype.as_deref(), transform);
        } else {
            self.alloc_entity_external(state.uid, prototype.as_deref());
        }

        Some(state.uid)
    }

    pub fn build_serialized_component_change_by_net_id(
        &mut self,
        serializer: &mut RobustSerializer,
        uid: EntityUid,
        net_id: u16,
        from_tick: GameTick,
    ) -> Option<SerializedComponentChange> {
        match net_id {
            Self::METADATA_NET_ID => {
                let meta = self.metadata.get(&uid)?;
                if from_tick != GameTick::ZERO && meta.entity_last_modified_tick <= from_tick {
                    return None;
                }
                Some(SerializedComponentChange::new(
                    Self::METADATA_NET_ID,
                    Self::component_created_since(&meta.base, from_tick),
                    false,
                    Some(
                        serializer
                            .serialize_component_state::<crate::MetaDataComponentState>(
                                &meta.get_component_state(),
                            )
                            .ok()?,
                    ),
                ))
            }
            Self::TRANSFORM_NET_ID => {
                let xform = self.transforms.get(&uid)?;
                if !Self::component_dirty_since(&xform.base, from_tick) {
                    return None;
                }
                Some(SerializedComponentChange::new(
                    Self::TRANSFORM_NET_ID,
                    Self::component_created_since(&xform.base, from_tick),
                    false,
                    Some(
                        serializer
                            .serialize_component_state::<TransformComponentState>(
                                &xform.get_component_state(),
                            )
                            .ok()?,
                    ),
                ))
            }
            Self::MAP_NET_ID => {
                let map = self.map_components.get(&uid)?;
                if !Self::component_dirty_since(&map.base, from_tick) {
                    return None;
                }
                Some(SerializedComponentChange::new(
                    Self::MAP_NET_ID,
                    Self::component_created_since(&map.base, from_tick),
                    false,
                    Some(
                        serializer
                            .serialize_component_state::<MapComponentState>(
                                &map.get_component_state(),
                            )
                            .ok()?,
                    ),
                ))
            }
            Self::MAP_GRID_NET_ID => {
                let grid = self.map_grid_components.get(&uid)?;
                if !Self::component_dirty_since(&grid.base, from_tick) {
                    return None;
                }
                Some(SerializedComponentChange::new(
                    Self::MAP_GRID_NET_ID,
                    Self::component_created_since(&grid.base, from_tick),
                    false,
                    Some(
                        serializer
                            .serialize_component_state::<MapGridComponentState>(
                                &grid.get_component_state(),
                            )
                            .ok()?,
                    ),
                ))
            }
            Self::PHYSICS_NET_ID => {
                let physics = self.physics.get(&uid)?;
                if !Self::component_dirty_since(&physics.base, from_tick) {
                    return None;
                }
                Some(SerializedComponentChange::new(
                    Self::PHYSICS_NET_ID,
                    Self::component_created_since(&physics.base, from_tick),
                    false,
                    Some(
                        serializer
                            .serialize_component_state::<crate::PhysicsComponentState>(
                                &physics.get_component_state(),
                            )
                            .ok()?,
                    ),
                ))
            }
            Self::APPEARANCE_NET_ID => {
                let appearance = self.appearances.get(&uid)?;
                if !Self::component_dirty_since(&appearance.base, from_tick) {
                    return None;
                }
                Some(SerializedComponentChange::new(
                    Self::APPEARANCE_NET_ID,
                    Self::component_created_since(&appearance.base, from_tick),
                    false,
                    Some(
                        serializer
                            .serialize_component_state::<crate::AppearanceComponentState>(
                                &appearance.get_component_state(),
                            )
                            .ok()?,
                    ),
                ))
            }
            Self::FIXTURES_NET_ID => {
                let fixtures = self.fixtures.get(&uid)?;
                if !Self::component_dirty_since(&fixtures.base, from_tick) {
                    return None;
                }
                Some(SerializedComponentChange::new(
                    Self::FIXTURES_NET_ID,
                    Self::component_created_since(&fixtures.base, from_tick),
                    false,
                    Some(
                        serializer
                            .serialize_component_state::<crate::FixturesComponentState>(
                                &fixtures.get_component_state(),
                            )
                            .ok()?,
                    ),
                ))
            }
            Self::JOINTS_NET_ID => {
                let joints = self.joint_components.get(&uid)?;
                if !Self::component_dirty_since(&joints.base, from_tick) {
                    return None;
                }
                Some(SerializedComponentChange::new(
                    Self::JOINTS_NET_ID,
                    Self::component_created_since(&joints.base, from_tick),
                    false,
                    Some(
                        serializer
                            .serialize_component_state::<crate::JointComponentState>(
                                &joints.get_component_state(),
                            )
                            .ok()?,
                    ),
                ))
            }
            Self::LOOKUP_NET_ID => {
                let lookup = self.entity_lookups.get(&uid)?;
                if !Self::component_dirty_since(&lookup.base, from_tick) {
                    return None;
                }
                Some(SerializedComponentChange::new(
                    Self::LOOKUP_NET_ID,
                    Self::component_created_since(&lookup.base, from_tick),
                    false,
                    Some(
                        serializer
                            .serialize_component_state::<crate::EntityLookupComponentState>(
                                &lookup.get_component_state(),
                            )
                            .ok()?,
                    ),
                ))
            }
            Self::BROADPHASE_NET_ID => {
                let broadphase = self.broadphases.get(&uid)?;
                if !Self::component_dirty_since(&broadphase.base, from_tick) {
                    return None;
                }
                Some(SerializedComponentChange::new(
                    Self::BROADPHASE_NET_ID,
                    Self::component_created_since(&broadphase.base, from_tick),
                    false,
                    Some(
                        serializer
                            .serialize_component_state::<crate::BroadphaseComponentState>(
                                &broadphase.get_component_state(),
                            )
                            .ok()?,
                    ),
                ))
            }
            Self::PHYSICS_MAP_NET_ID => {
                let physics_map = self.physics_maps.get(&uid)?;
                if !Self::component_dirty_since(&physics_map.base, from_tick) {
                    return None;
                }
                Some(SerializedComponentChange::new(
                    Self::PHYSICS_MAP_NET_ID,
                    Self::component_created_since(&physics_map.base, from_tick),
                    false,
                    Some(
                        serializer
                            .serialize_component_state::<crate::SharedPhysicsMapComponentState>(
                                &physics_map.get_component_state(),
                            )
                            .ok()?,
                    ),
                ))
            }
            Self::COLLISION_WAKE_NET_ID => {
                let collision_wake = self.collision_wakes.get(&uid)?;
                if !Self::component_dirty_since(&collision_wake.base, from_tick) {
                    return None;
                }
                Some(SerializedComponentChange::new(
                    Self::COLLISION_WAKE_NET_ID,
                    Self::component_created_since(&collision_wake.base, from_tick),
                    false,
                    Some(
                        serializer
                            .serialize_component_state::<crate::CollisionWakeComponentState>(
                                &collision_wake.get_component_state(),
                            )
                            .ok()?,
                    ),
                ))
            }
            Self::COLLIDE_ON_ANCHOR_NET_ID => {
                let collide_on_anchor = self.collide_on_anchors.get(&uid)?;
                if !Self::component_dirty_since(&collide_on_anchor.base, from_tick) {
                    return None;
                }
                Some(SerializedComponentChange::new(
                    Self::COLLIDE_ON_ANCHOR_NET_ID,
                    Self::component_created_since(&collide_on_anchor.base, from_tick),
                    false,
                    Some(
                        serializer
                            .serialize_component_state::<crate::CollideOnAnchorComponentState>(
                                &collide_on_anchor.get_component_state(),
                            )
                            .ok()?,
                    ),
                ))
            }
            _ => None,
        }
    }

    pub fn build_serialized_component_changes_since(
        &mut self,
        serializer: &mut RobustSerializer,
        uid: EntityUid,
        from_tick: GameTick,
    ) -> Vec<SerializedComponentChange> {
        let mut changes = Vec::new();
        for net_id in Self::SERIALIZED_NET_IDS {
            if let Some(change) =
                self.build_serialized_component_change_by_net_id(serializer, uid, net_id, from_tick)
            {
                changes.push(change);
            }
        }
        changes
    }

    pub fn append_deleted_component_changes(
        &self,
        uid: EntityUid,
        changes: &mut Vec<SerializedComponentChange>,
        deleted_net_ids: &[u16],
    ) {
        for net_id in deleted_net_ids {
            if self.component_exists_for_net_id(uid, *net_id) {
                continue;
            }
            if changes.iter().any(|change| change.net_id == *net_id) {
                continue;
            }
            changes.push(Self::deleted_component_change(*net_id));
        }
    }

    pub fn build_serialized_entity_state(
        &mut self,
        serializer: &mut RobustSerializer,
        uid: EntityUid,
    ) -> Option<SerializedEntityState> {
        self.build_serialized_entity_state_since(serializer, uid, GameTick::ZERO, &[])
    }

    pub fn entity_dirty_since(&self, uid: EntityUid, from_tick: GameTick) -> bool {
        if from_tick == GameTick::ZERO {
            return self.entity_exists(uid);
        }

        self.metadata
            .get(&uid)
            .map(|meta| meta.entity_last_modified_tick > from_tick)
            .unwrap_or(false)
    }

    pub fn build_serialized_entity_state_since(
        &mut self,
        serializer: &mut RobustSerializer,
        uid: EntityUid,
        from_tick: GameTick,
        deleted_net_ids: &[u16],
    ) -> Option<SerializedEntityState> {
        if !self.entity_exists(uid) {
            return None;
        }

        let mut changes = self.build_serialized_component_changes_since(serializer, uid, from_tick);
        self.append_deleted_component_changes(uid, &mut changes, deleted_net_ids);

        Some(SerializedEntityState {
            uid,
            component_changes: changes,
        })
    }

    pub fn build_serialized_entity_state_with_deleted_callback<F>(
        &mut self,
        serializer: &mut RobustSerializer,
        uid: EntityUid,
        from_tick: GameTick,
        mut deleted_net_ids: F,
    ) -> Option<SerializedEntityState>
    where
        F: FnMut(EntityUid, GameTick) -> Vec<u16>,
    {
        let deleted_components = deleted_net_ids(uid, from_tick);
        self.build_serialized_entity_state_since(serializer, uid, from_tick, &deleted_components)
    }

    pub fn build_serialized_entity_states_for_sync<F>(
        &mut self,
        serializer: &mut RobustSerializer,
        ids: &[EntityUid],
        newly_visible: &[EntityUid],
        from_tick: GameTick,
        mut deleted_net_ids: F,
    ) -> Vec<SerializedEntityState>
    where
        F: FnMut(EntityUid, GameTick) -> Vec<u16>,
    {
        let newly_visible = newly_visible
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        let mut states = Vec::new();

        for uid in ids.iter().copied() {
            if newly_visible.contains(&uid) {
                if let Some(state) = self.build_serialized_entity_state(serializer, uid) {
                    if !state.component_changes.is_empty() {
                        states.push(state);
                    }
                }
                continue;
            }

            if !self.entity_dirty_since(uid, from_tick) {
                continue;
            }

            if let Some(state) = self.build_serialized_entity_state_with_deleted_callback(
                serializer,
                uid,
                from_tick,
                &mut deleted_net_ids,
            ) {
                if !state.component_changes.is_empty() {
                    states.push(state);
                }
            }
        }

        states
    }

    pub fn ensure_appearance(&mut self, uid: EntityUid) -> &mut AppearanceComponent {
        let created = !self.appearances.contains_key(&uid);
        let start_running = self.entity_initialized_for_component_lifecycle(uid);
        self.appearances.entry(uid).or_insert_with(|| {
            let mut component = AppearanceComponent::new();
            Self::initialize_new_component_base(
                &mut component.base,
                uid,
                self.current_tick,
                start_running,
            );
            component
        });
        if created {
            self.queue_new_component_lifecycle(uid, "AppearanceComponent", start_running);
        }
        self.appearances.get_mut(&uid).unwrap()
    }

    pub fn set_appearance_data<T>(
        &mut self,
        uid: EntityUid,
        key: impl Into<String>,
        value: T,
    ) -> bool
    where
        T: crate::IntoAppearanceValue,
    {
        let changed = self.ensure_appearance(uid).set_data(key, value);
        if changed {
            self.mark_appearance_dirty(uid);
        }
        changed
    }

    pub fn ensure_metadata(&mut self, uid: EntityUid) -> &mut MetaDataComponent {
        let created = !self.metadata.contains_key(&uid);
        let start_running = self.entity_initialized_for_component_lifecycle(uid);
        self.metadata.entry(uid).or_insert_with(|| {
            let mut component = MetaDataComponent::new();
            Self::initialize_new_component_base(
                &mut component.base,
                uid,
                self.current_tick,
                start_running,
            );
            component
        });
        if created {
            self.queue_new_component_lifecycle(uid, "MetaDataComponent", start_running);
        }
        self.metadata.get_mut(&uid).unwrap()
    }

    pub fn ensure_transform(&mut self, uid: EntityUid) -> &mut TransformComponent {
        let created = !self.transforms.contains_key(&uid);
        let start_running = self.entity_initialized_for_component_lifecycle(uid);
        self.transforms.entry(uid).or_insert_with(|| {
            let mut component = TransformComponent::new();
            Self::initialize_new_component_base(
                &mut component.base,
                uid,
                self.current_tick,
                start_running,
            );
            component
        });
        if created {
            self.queue_new_component_lifecycle(uid, "TransformComponent", start_running);
        }
        self.transforms.get_mut(&uid).unwrap()
    }

    pub fn ensure_lookup(&mut self, uid: EntityUid) -> &mut EntityLookupComponent {
        let created = !self.entity_lookups.contains_key(&uid);
        let start_running = self.entity_initialized_for_component_lifecycle(uid);
        self.entity_lookups.entry(uid).or_insert_with(|| {
            let mut component = EntityLookupComponent::new();
            Self::initialize_new_component_base(
                &mut component.base,
                uid,
                self.current_tick,
                start_running,
            );
            component
        });
        if created {
            self.queue_new_component_lifecycle(uid, "EntityLookupComponent", start_running);
        }
        self.entity_lookups.get_mut(&uid).unwrap()
    }

    pub fn ensure_joints(&mut self, uid: EntityUid) -> &mut JointComponent {
        let created = !self.joint_components.contains_key(&uid);
        let start_running = self.entity_initialized_for_component_lifecycle(uid);
        self.joint_components.entry(uid).or_insert_with(|| {
            let mut component = JointComponent::new();
            Self::initialize_new_component_base(
                &mut component.base,
                uid,
                self.current_tick,
                start_running,
            );
            component
        });
        if created {
            self.queue_new_component_lifecycle(uid, "JointComponent", start_running);
        }
        self.joint_components.get_mut(&uid).unwrap()
    }

    fn touch_joint_component_runtime(&mut self, uid: EntityUid) {
        if let Some(component) = self.joint_components.get_mut(&uid) {
            component.base.last_modified_tick = self.current_tick;
        }
        if self.entity_exists(uid) {
            self.dirty_entity(uid);
        }
    }

    fn set_physics_awake_from_runtime(&mut self, uid: EntityUid, awake: bool) -> bool {
        let Some(body) = self.physics.get_mut(&uid) else {
            return false;
        };
        let previous = body.awake;
        body.set_awake(awake);
        if body.awake == previous {
            return false;
        }
        body.base.last_modified_tick = self.current_tick;
        if self.entity_exists(uid) {
            self.dirty_entity(uid);
        }
        self.queue_entity_physics_runtime(
            uid,
            if awake {
                crate::PhysicsRuntimeEvent::Wake(crate::PhysicsWakeMessage { body: uid })
            } else {
                crate::PhysicsRuntimeEvent::Sleep(crate::PhysicsSleepMessage { body: uid })
            },
        );
        self.reconcile_entity_physics_runtime(uid);
        true
    }

    pub fn add_joint_between(&mut self, joint: butsuri::Joint) -> bool {
        let body_a = EntityUid::new(joint.body_a_uid);
        let body_b = EntityUid::new(joint.body_b_uid);
        if body_a == body_b {
            return false;
        }

        let id = joint.id.clone();
        let inserted_a = self.ensure_joints(body_a).add_joint(joint.clone());
        let inserted_b = self.ensure_joints(body_b).add_joint(joint);
        if inserted_a && inserted_b {
            self.touch_joint_component_runtime(body_a);
            self.touch_joint_component_runtime(body_b);
            self.queue_entity_physics_runtime(
                body_a,
                crate::PhysicsRuntimeEvent::JointAdded(crate::JointAddedEvent {
                    body_a,
                    body_b,
                    joint_id: id,
                }),
            );
            let _ = self.set_physics_awake_from_runtime(body_a, true);
            let _ = self.set_physics_awake_from_runtime(body_b, true);
            return true;
        }

        if inserted_a {
            let _ = self
                .joint_components
                .get_mut(&body_a)
                .and_then(|component| component.remove_joint(&id));
        }
        if inserted_b {
            let _ = self
                .joint_components
                .get_mut(&body_b)
                .and_then(|component| component.remove_joint(&id));
        }
        false
    }

    pub fn remove_joint_between(&mut self, body_a: EntityUid, body_b: EntityUid, id: &str) -> bool {
        let removed_a = self
            .joint_components
            .get_mut(&body_a)
            .and_then(|component| component.remove_joint(id))
            .is_some();
        let removed_b = self
            .joint_components
            .get_mut(&body_b)
            .and_then(|component| component.remove_joint(id))
            .is_some();
        if removed_a {
            self.touch_joint_component_runtime(body_a);
        }
        if removed_b {
            self.touch_joint_component_runtime(body_b);
        }
        if removed_a || removed_b {
            self.queue_entity_physics_runtime(
                body_a,
                crate::PhysicsRuntimeEvent::JointRemoved(crate::JointRemovedEvent {
                    body_a,
                    body_b,
                    joint_id: id.to_string(),
                }),
            );
        }
        removed_a || removed_b
    }

    pub fn ensure_ignore_pause(&mut self, uid: EntityUid) -> &mut IgnorePauseComponent {
        let created = !self.ignore_pauses.contains_key(&uid);
        let start_running = self.entity_initialized_for_component_lifecycle(uid);
        self.ignore_pauses.entry(uid).or_insert_with(|| {
            let mut component = IgnorePauseComponent::new();
            Self::initialize_new_component_base(
                &mut component.base,
                uid,
                self.current_tick,
                start_running,
            );
            component
        });
        if created {
            self.queue_new_component_lifecycle(uid, "IgnorePauseComponent", start_running);
            let _ = self.set_entity_paused(uid, false);
        }
        self.ignore_pauses.get_mut(&uid).unwrap()
    }

    pub fn ensure_collision_wake(&mut self, uid: EntityUid) -> &mut CollisionWakeComponent {
        let created = !self.collision_wakes.contains_key(&uid);
        let start_running = self.entity_initialized_for_component_lifecycle(uid);
        self.collision_wakes.entry(uid).or_insert_with(|| {
            let mut component = CollisionWakeComponent::new();
            Self::initialize_new_component_base(
                &mut component.base,
                uid,
                self.current_tick,
                start_running,
            );
            component
        });
        if created {
            self.queue_new_component_lifecycle(uid, "CollisionWakeComponent", start_running);
        }
        self.collision_wakes.get_mut(&uid).unwrap()
    }

    pub fn set_collision_wake_enabled(&mut self, uid: EntityUid, enabled: bool) -> bool {
        let tick = self.current_tick;
        let Some(component) = self.collision_wakes.get_mut(&uid) else {
            return false;
        };
        if component.enabled == enabled {
            return false;
        }

        component.enabled = enabled;
        component.base.last_modified_tick = tick;
        self.dirty_entity(uid);
        self.refresh_collision_wake(uid)
    }

    pub fn refresh_collision_wake(&mut self, uid: EntityUid) -> bool {
        let Some(component) = self.collision_wakes.get(&uid) else {
            return false;
        };

        let desired = if !component.enabled {
            true
        } else {
            self.physics.get(&uid).is_some_and(|body| body.awake)
                || self
                    .joint_components
                    .get(&uid)
                    .is_some_and(|joints| joints.joint_count() > 0)
                || self
                    .transforms
                    .get(&uid)
                    .is_some_and(|transform| transform.grid_id == crate::GridId::INVALID)
        };

        let Some(body) = self.physics.get_mut(&uid) else {
            return false;
        };
        if body.can_collide == desired {
            return false;
        }

        body.can_collide = desired;
        body.base.last_modified_tick = self.current_tick;
        self.dirty_entity(uid);
        let _ = self.queue_entity_runtime_event(
            uid,
            crate::PhysicsRuntimeEvent::CollisionChange(crate::CollisionChangeMessage {
                owner: uid,
                can_collide: desired,
            }),
        );
        true
    }

    pub fn ensure_collide_on_anchor(&mut self, uid: EntityUid) -> &mut CollideOnAnchorComponent {
        let created = !self.collide_on_anchors.contains_key(&uid);
        let start_running = self.entity_initialized_for_component_lifecycle(uid);
        self.collide_on_anchors.entry(uid).or_insert_with(|| {
            let mut component = CollideOnAnchorComponent::new();
            Self::initialize_new_component_base(
                &mut component.base,
                uid,
                self.current_tick,
                start_running,
            );
            component
        });
        if created {
            self.queue_new_component_lifecycle(uid, "CollideOnAnchorComponent", start_running);
        }
        self.collide_on_anchors.get_mut(&uid).unwrap()
    }

    pub fn set_collide_on_anchor_enabled(&mut self, uid: EntityUid, enable: bool) -> bool {
        let tick = self.current_tick;
        let Some(component) = self.collide_on_anchors.get_mut(&uid) else {
            return false;
        };
        if component.enable == enable {
            return false;
        }

        component.enable = enable;
        component.base.last_modified_tick = tick;
        self.dirty_entity(uid);
        self.refresh_collide_on_anchor(uid)
    }

    pub fn refresh_collide_on_anchor(&mut self, uid: EntityUid) -> bool {
        let Some(component) = self.collide_on_anchors.get(&uid) else {
            return false;
        };
        let anchored = self
            .transforms
            .get(&uid)
            .is_some_and(|transform| transform.anchored);
        let desired = if anchored {
            component.enable
        } else {
            !component.enable
        };

        let Some(body) = self.physics.get_mut(&uid) else {
            return false;
        };
        if body.can_collide == desired {
            return false;
        }

        body.can_collide = desired;
        body.base.last_modified_tick = self.current_tick;
        self.dirty_entity(uid);
        let _ = self.queue_entity_runtime_event(
            uid,
            crate::PhysicsRuntimeEvent::CollisionChange(crate::CollisionChangeMessage {
                owner: uid,
                can_collide: desired,
            }),
        );
        true
    }

    pub fn ensure_map(&mut self, map_id: MapId, uid: EntityUid) -> &mut MapComponent {
        let created = !self.map_components.contains_key(&uid);
        let start_running = self.entity_initialized_for_component_lifecycle(uid);
        self.map_components.entry(uid).or_insert_with(|| {
            let mut component = MapComponent::new();
            Self::initialize_new_component_base(
                &mut component.base,
                uid,
                self.current_tick,
                start_running,
            );
            component.world_map = map_id;
            component
        });
        if let Some(meta) = self.metadata.get_mut(&uid) {
            meta.map_id = map_id;
        }
        if created {
            self.queue_new_component_lifecycle(uid, "MapComponent", start_running);
        }
        self.map_components.get_mut(&uid).unwrap()
    }

    pub fn ensure_map_grid(&mut self, uid: EntityUid) -> &mut MapGridComponent {
        let created = !self.map_grid_components.contains_key(&uid);
        let start_running = self.entity_initialized_for_component_lifecycle(uid);
        self.map_grid_components.entry(uid).or_insert_with(|| {
            let mut component = MapGridComponent::new();
            Self::initialize_new_component_base(
                &mut component.base,
                uid,
                self.current_tick,
                start_running,
            );
            component
        });
        if created {
            self.queue_new_component_lifecycle(uid, "MapGridComponent", start_running);
        }
        self.map_grid_components.get_mut(&uid).unwrap()
    }

    pub fn ensure_timer(&mut self, uid: EntityUid) -> &mut TimerComponent {
        let created = !self.timers.contains_key(&uid);
        let start_running = self.entity_initialized_for_component_lifecycle(uid);
        self.timers.entry(uid).or_insert_with(|| {
            let mut component = TimerComponent::new();
            Self::initialize_new_component_base(
                &mut component.base,
                uid,
                self.current_tick,
                start_running,
            );
            component
        });
        if created {
            self.queue_new_component_lifecycle(uid, "TimerComponent", start_running);
        }
        self.timers.get_mut(&uid).unwrap()
    }

    pub fn spawn_timer(
        &mut self,
        uid: EntityUid,
        milliseconds: i32,
        on_fired: impl FnMut() + Send + Sync + 'static,
    ) -> crate::TimerHandle {
        self.ensure_timer(uid).spawn(milliseconds, on_fired)
    }

    pub fn spawn_repeating_timer(
        &mut self,
        uid: EntityUid,
        milliseconds: i32,
        on_fired: impl FnMut() + Send + Sync + 'static,
    ) -> crate::TimerHandle {
        self.ensure_timer(uid)
            .spawn_repeating(milliseconds, on_fired)
    }

    pub fn update_timer_runtime(&mut self, frame_time: f32) {
        let timers: Vec<_> = self.timers.keys().copied().collect();

        for uid in &timers {
            if self.is_entity_paused(*uid) {
                continue;
            }
            if let Some(timer) = self.timers.get_mut(uid) {
                timer.update(frame_time);
            }
        }

        let removable = timers
            .into_iter()
            .filter(|uid| {
                self.timers
                    .get(uid)
                    .is_some_and(|timer| timer.remove_on_empty && timer.timer_count() == 0)
            })
            .collect::<Vec<_>>();

        for uid in removable {
            if !self.deleted(uid) {
                let _ = self.remove_timer_component(uid);
            }
        }
    }

    pub fn ensure_physics(&mut self, uid: EntityUid) -> &mut PhysicsComponent {
        let created = !self.physics.contains_key(&uid);
        let start_running = self.entity_initialized_for_component_lifecycle(uid);
        if created {
            let component = self.physics.entry(uid).or_insert_with(|| {
                let mut component = PhysicsComponent::new();
                Self::initialize_new_component_base(
                    &mut component.base,
                    uid,
                    self.current_tick,
                    start_running,
                );
                component
            });
            let _ = component;
            self.queue_new_component_lifecycle(uid, "Physics", start_running);
        }
        self.physics.entry(uid).or_insert_with(|| {
            let mut component = PhysicsComponent::new();
            Self::initialize_new_component_base(
                &mut component.base,
                uid,
                self.current_tick,
                start_running,
            );
            component
        })
    }

    pub fn ensure_fixtures(&mut self, uid: EntityUid) -> &mut FixturesComponent {
        let created = !self.fixtures.contains_key(&uid);
        let start_running = self.entity_initialized_for_component_lifecycle(uid);
        self.fixtures.entry(uid).or_insert_with(|| {
            let mut component = FixturesComponent::new();
            Self::initialize_new_component_base(
                &mut component.base,
                uid,
                self.current_tick,
                start_running,
            );
            component
        });
        if created {
            self.queue_new_component_lifecycle(uid, "FixturesComponent", start_running);
        }
        self.fixtures.get_mut(&uid).unwrap()
    }

    pub fn insert_fixture_and_reconcile(
        &mut self,
        uid: EntityUid,
        fixture: butsuri::Fixture,
    ) -> Option<butsuri::Fixture> {
        if !self.entity_exists(uid) {
            return None;
        }

        let current_tick = self.current_tick;
        let previous = {
            let fixtures = self.ensure_fixtures(uid);
            let previous = fixtures.insert_fixture(fixture);
            fixtures.base.last_modified_tick = current_tick;
            previous
        };
        self.dirty_entity(uid);
        self.reconcile_entity_physics_runtime(uid);
        previous
    }

    pub fn ensure_broadphase(&mut self, uid: EntityUid) -> &mut BroadphaseComponent {
        let created = !self.broadphases.contains_key(&uid);
        let start_running = self.entity_initialized_for_component_lifecycle(uid);
        self.broadphases.entry(uid).or_insert_with(|| {
            let mut component = BroadphaseComponent::new();
            Self::initialize_new_component_base(
                &mut component.base,
                uid,
                self.current_tick,
                start_running,
            );
            component
        });
        if created {
            self.queue_new_component_lifecycle(uid, "BroadphaseComponent", start_running);
        }
        self.broadphases.get_mut(&uid).unwrap()
    }

    pub fn ensure_physics_map(&mut self, uid: EntityUid) -> &mut SharedPhysicsMapComponent {
        let created = !self.physics_maps.contains_key(&uid);
        let start_running = self.entity_initialized_for_component_lifecycle(uid);
        self.physics_maps.entry(uid).or_insert_with(|| {
            let mut component = SharedPhysicsMapComponent::new();
            Self::initialize_new_component_base(
                &mut component.base,
                uid,
                self.current_tick,
                start_running,
            );
            component
        });
        if created {
            self.queue_new_component_lifecycle(uid, "SharedPhysicsMapComponent", start_running);
        }
        self.physics_maps.get_mut(&uid).unwrap()
    }

    pub fn to_pretty_string(&self, uid: EntityUid) -> EntityStringRepresentation {
        let Some(metadata) = self.metadata.get(&uid) else {
            return EntityStringRepresentation::new(uid, true, None, None);
        };
        EntityStringRepresentation::new(
            uid,
            metadata.entity_deleted(),
            metadata.entity_name.clone(),
            metadata.prototype_id.clone(),
        )
    }

    pub fn remove_transform_component(&mut self, uid: EntityUid) -> bool {
        if !self.transforms.contains_key(&uid) {
            return false;
        }

        self.clear_transform_runtime_state(uid);

        let removed = self.transforms.remove(&uid);
        if let Some(mut component) = removed {
            self.queue_component_shutdown_and_remove(uid, &mut component.base);
            return true;
        }
        false
    }

    pub fn remove_metadata_component(&mut self, uid: EntityUid) -> bool {
        self.remove_component_storage_with_lifecycle(uid, "MetaDataComponent")
    }

    pub fn remove_map_grid_component(&mut self, uid: EntityUid) -> bool {
        let removed_component = self.map_grid_components.remove(&uid);
        let removed_component_exists = removed_component.is_some();
        let removed_grid = self.remove_map_grid_runtime_support(uid);
        if let Some(mut component) = removed_component {
            self.queue_component_shutdown_and_remove(uid, &mut component.base);
        }
        removed_component_exists || removed_grid
    }

    pub fn apply_map_component_state(&mut self, uid: EntityUid, state: MapComponentState) -> bool {
        self.ensure_transform(uid);
        let previous_paused = self
            .map_components
            .get(&uid)
            .map(|map| map.map_paused || map.map_pre_init);
        let map = self.ensure_map(state.map_id, uid);
        map.handle_map_state(state);
        self.sync_entity_metadata_map_id(uid);
        self.sync_map_pause_recursive(uid);
        let current_paused = self.is_map_paused(state.map_id);
        if previous_paused != Some(current_paused) {
            self.queue_entity_event(crate::MapPausedEvent {
                entity: uid,
                paused: current_paused,
            });
        }
        let _ = self.run_map_init_if_current_map_initialized(uid);
        true
    }

    pub fn apply_map_grid_component_state(
        &mut self,
        uid: EntityUid,
        state: MapGridComponentState,
    ) -> bool {
        self.ensure_transform(uid);
        let grid = self.ensure_map_grid(uid);
        grid.handle_map_grid_state(state);
        self.sync_map_grid_runtime(uid)
    }

    pub fn apply_metadata_component_state(
        &mut self,
        uid: EntityUid,
        state: crate::MetaDataComponentState,
        tick: GameTick,
    ) -> bool {
        let metadata = self.ensure_metadata(uid);
        metadata.handle_component_state(state, tick);
        true
    }

    pub fn apply_transform_component_state(
        &mut self,
        uid: EntityUid,
        state: TransformComponentState,
    ) -> bool {
        self.ensure_transform(uid);
        self.apply_transform_state(uid, state)
    }

    pub fn apply_physics_component_state(
        &mut self,
        uid: EntityUid,
        state: crate::PhysicsComponentState,
    ) -> bool {
        let previous = self
            .physics
            .get(&uid)
            .map(|body| (body.awake, body.can_collide));
        let physics = self.ensure_physics(uid);
        physics.handle_component_state(state);
        let current = self
            .physics
            .get(&uid)
            .map(|body| (body.awake, body.can_collide));
        if let (
            Some((previous_awake, previous_can_collide)),
            Some((current_awake, current_can_collide)),
        ) = (previous, current)
        {
            if previous_awake != current_awake {
                self.queue_entity_physics_runtime(
                    uid,
                    if current_awake {
                        crate::PhysicsRuntimeEvent::Wake(crate::PhysicsWakeMessage { body: uid })
                    } else {
                        crate::PhysicsRuntimeEvent::Sleep(crate::PhysicsSleepMessage { body: uid })
                    },
                );
            }
            if previous_can_collide != current_can_collide {
                self.queue_entity_physics_runtime(
                    uid,
                    crate::PhysicsRuntimeEvent::CollisionChange(crate::CollisionChangeMessage {
                        owner: uid,
                        can_collide: current_can_collide,
                    }),
                );
            }
        }
        self.reconcile_entity_physics_runtime(uid);
        true
    }

    pub fn apply_appearance_component_state(
        &mut self,
        uid: EntityUid,
        state: crate::AppearanceComponentState,
    ) -> bool {
        let appearance = self.ensure_appearance(uid);
        appearance.handle_component_state(state);
        self.mark_appearance_dirty(uid)
    }

    pub fn apply_fixtures_component_state(
        &mut self,
        uid: EntityUid,
        state: crate::FixturesComponentState,
    ) -> bool {
        let current_tick = self.current_tick;
        let fixtures = self.ensure_fixtures(uid);
        fixtures.handle_component_state(state);
        fixtures.base.last_modified_tick = current_tick;
        self.dirty_entity(uid);
        self.reconcile_entity_physics_runtime(uid);
        true
    }

    pub fn apply_joint_component_state(
        &mut self,
        uid: EntityUid,
        state: crate::JointComponentState,
    ) -> bool {
        let previous_joints = self
            .joint_components
            .get(&uid)
            .map(|component| component.joints.clone())
            .unwrap_or_default();
        let joints = self.ensure_joints(uid);
        joints.handle_component_state(state);
        let current_joints = self
            .joint_components
            .get(&uid)
            .map(|component| component.joints.clone())
            .unwrap_or_default();

        for (joint_id, joint) in &current_joints {
            if previous_joints.contains_key(joint_id) {
                continue;
            }
            let body_a = EntityUid::new(joint.body_a_uid);
            let body_b = EntityUid::new(joint.body_b_uid);
            self.queue_entity_physics_runtime(
                uid,
                crate::PhysicsRuntimeEvent::JointAdded(crate::JointAddedEvent {
                    body_a,
                    body_b,
                    joint_id: joint_id.clone(),
                }),
            );
        }

        for (joint_id, joint) in &previous_joints {
            if current_joints.contains_key(joint_id) {
                continue;
            }
            let body_a = EntityUid::new(joint.body_a_uid);
            let body_b = EntityUid::new(joint.body_b_uid);
            self.queue_entity_physics_runtime(
                uid,
                crate::PhysicsRuntimeEvent::JointRemoved(crate::JointRemovedEvent {
                    body_a,
                    body_b,
                    joint_id: joint_id.clone(),
                }),
            );
        }
        self.reconcile_entity_physics_runtime(uid);
        true
    }

    pub fn apply_collision_wake_component_state(
        &mut self,
        uid: EntityUid,
        state: crate::CollisionWakeComponentState,
    ) -> bool {
        let collision_wake = self.ensure_collision_wake(uid);
        collision_wake.handle_component_state(state);
        self.reconcile_entity_physics_runtime(uid);
        true
    }

    pub fn apply_collide_on_anchor_component_state(
        &mut self,
        uid: EntityUid,
        state: crate::CollideOnAnchorComponentState,
    ) -> bool {
        let collide_on_anchor = self.ensure_collide_on_anchor(uid);
        collide_on_anchor.handle_component_state(state);
        self.reconcile_entity_physics_runtime(uid);
        true
    }

    pub fn apply_lookup_component_state(
        &mut self,
        uid: EntityUid,
        state: crate::EntityLookupComponentState,
    ) -> bool {
        let lookup = self.ensure_lookup(uid);
        lookup.handle_component_state(state);
        true
    }

    pub fn apply_broadphase_component_state(
        &mut self,
        uid: EntityUid,
        state: crate::BroadphaseComponentState,
    ) -> bool {
        let broadphase = self.ensure_broadphase(uid);
        broadphase.handle_component_state(state);
        true
    }

    pub fn apply_physics_map_component_state(
        &mut self,
        uid: EntityUid,
        state: crate::SharedPhysicsMapComponentState,
    ) -> bool {
        let physics_map = self.ensure_physics_map(uid);
        physics_map.handle_component_state(state);
        true
    }

    pub fn apply_serialized_component_by_net_id(
        &mut self,
        serializer: &mut RobustSerializer,
        uid: EntityUid,
        net_id: u16,
        component_state: &SerializableComponentState,
    ) -> bool {
        match net_id {
            Self::METADATA_NET_ID => serializer
                .deserialize_component_state::<crate::MetaDataComponentState>(component_state)
                .ok()
                .is_some_and(|state| {
                    self.apply_metadata_component_state(uid, state, self.current_tick)
                }),
            Self::TRANSFORM_NET_ID => serializer
                .deserialize_component_state::<TransformComponentState>(component_state)
                .ok()
                .is_some_and(|state| self.apply_transform_component_state(uid, state)),
            Self::MAP_NET_ID => serializer
                .deserialize_component_state::<MapComponentState>(component_state)
                .ok()
                .is_some_and(|state| self.apply_map_component_state(uid, state)),
            Self::MAP_GRID_NET_ID => serializer
                .deserialize_component_state::<MapGridComponentState>(component_state)
                .ok()
                .is_some_and(|state| self.apply_map_grid_component_state(uid, state)),
            Self::PHYSICS_NET_ID => serializer
                .deserialize_component_state::<crate::PhysicsComponentState>(component_state)
                .ok()
                .is_some_and(|state| self.apply_physics_component_state(uid, state)),
            Self::APPEARANCE_NET_ID => serializer
                .deserialize_component_state::<crate::AppearanceComponentState>(component_state)
                .ok()
                .is_some_and(|state| self.apply_appearance_component_state(uid, state)),
            Self::FIXTURES_NET_ID => serializer
                .deserialize_component_state::<crate::FixturesComponentState>(component_state)
                .ok()
                .is_some_and(|state| self.apply_fixtures_component_state(uid, state)),
            Self::JOINTS_NET_ID => serializer
                .deserialize_component_state::<crate::JointComponentState>(component_state)
                .ok()
                .is_some_and(|state| self.apply_joint_component_state(uid, state)),
            Self::LOOKUP_NET_ID => serializer
                .deserialize_component_state::<crate::EntityLookupComponentState>(component_state)
                .ok()
                .is_some_and(|state| self.apply_lookup_component_state(uid, state)),
            Self::BROADPHASE_NET_ID => serializer
                .deserialize_component_state::<crate::BroadphaseComponentState>(component_state)
                .ok()
                .is_some_and(|state| self.apply_broadphase_component_state(uid, state)),
            Self::PHYSICS_MAP_NET_ID => serializer
                .deserialize_component_state::<crate::SharedPhysicsMapComponentState>(
                    component_state,
                )
                .ok()
                .is_some_and(|state| self.apply_physics_map_component_state(uid, state)),
            Self::COLLISION_WAKE_NET_ID => serializer
                .deserialize_component_state::<crate::CollisionWakeComponentState>(component_state)
                .ok()
                .is_some_and(|state| self.apply_collision_wake_component_state(uid, state)),
            Self::COLLIDE_ON_ANCHOR_NET_ID => serializer
                .deserialize_component_state::<crate::CollideOnAnchorComponentState>(
                    component_state,
                )
                .ok()
                .is_some_and(|state| self.apply_collide_on_anchor_component_state(uid, state)),
            _ => false,
        }
    }

    fn apply_serialized_entity_state_delta_with<F>(
        &mut self,
        serializer: &mut RobustSerializer,
        state: &SerializedEntityState,
        mut apply_transform: F,
    ) -> Option<(EntityUid, bool)>
    where
        F: FnMut(&mut Self, EntityUid, TransformComponentState),
    {
        let created = !self.entity_exists(state.uid);
        let _ = self.create_entity_from_serialized_state(serializer, state);

        for change in &state.component_changes {
            if change.deleted {
                let _ = self.remove_component_by_net_id(state.uid, change.net_id);
                continue;
            }

            let Some(component_state) = change.state.as_ref() else {
                continue;
            };

            if change.net_id == Self::TRANSFORM_NET_ID {
                if let Ok(transform_state) = serializer
                    .deserialize_component_state::<TransformComponentState>(component_state)
                {
                    apply_transform(self, state.uid, transform_state);
                }
                continue;
            }

            let _ = self.apply_serialized_component_by_net_id(
                serializer,
                state.uid,
                change.net_id,
                component_state,
            );
        }

        Some((state.uid, created))
    }

    pub fn apply_serialized_entity_state_with<F>(
        &mut self,
        serializer: &mut RobustSerializer,
        state: &SerializedEntityState,
        mut apply_transform: F,
    ) -> Option<EntityUid>
    where
        F: FnMut(&mut Self, EntityUid, TransformComponentState),
    {
        let (uid, created) = self.apply_serialized_entity_state_delta_with(
            serializer,
            state,
            |manager, uid, transform_state| {
                apply_transform(manager, uid, transform_state);
            },
        )?;

        if created {
            let _ = self.initialize_entity(uid);
        }

        Some(uid)
    }

    pub fn apply_serialized_entity_state(
        &mut self,
        serializer: &mut RobustSerializer,
        state: &SerializedEntityState,
    ) -> Option<EntityUid> {
        self.apply_serialized_entity_state_with(
            serializer,
            state,
            |manager, uid, transform_state| {
                let _ = manager.apply_transform_component_state(uid, transform_state);
            },
        )
    }

    pub fn apply_serialized_entity_states_with<F>(
        &mut self,
        serializer: &mut RobustSerializer,
        states: &[SerializedEntityState],
        mut apply_transform: F,
    ) -> Vec<EntityUid>
    where
        F: FnMut(&mut Self, EntityUid, TransformComponentState),
    {
        let mut created_entities = Vec::new();
        for state in states {
            if let Some((uid, created)) = self.apply_serialized_entity_state_delta_with(
                serializer,
                state,
                |manager, uid, transform_state| {
                    apply_transform(manager, uid, transform_state);
                },
            ) {
                if created {
                    created_entities.push(uid);
                }
            }
        }
        created_entities
    }

    fn initialize_new_entities(&mut self, created_entities: &[EntityUid]) {
        let mut initialized = HashSet::new();
        for uid in created_entities {
            if initialized.insert(*uid) && self.entity_exists(*uid) {
                let _ = self.initialize_entity(*uid);
            }
        }
    }

    pub fn apply_entity_deletions(&mut self, deletions: &[EntityUid]) {
        for uid in deletions {
            self.queue_delete_entity(*uid);
            self.flush_queued_deletions();
        }
    }

    pub fn apply_game_state_delta_with<F>(
        &mut self,
        serializer: &mut RobustSerializer,
        state: &GameState,
        mut apply_transform: F,
    ) -> Vec<EntityUid>
    where
        F: FnMut(&mut Self, EntityUid, TransformComponentState),
    {
        let created_entities = self.apply_serialized_entity_states_with(
            serializer,
            &state.entity_states,
            |manager, uid, transform_state| {
                apply_transform(manager, uid, transform_state);
            },
        );

        if let Some(map_data) = &state.map_data {
            self.apply_game_state_map_data(map_data);
        }

        self.apply_entity_deletions(&state.entity_deletions);

        self.initialize_new_entities(&created_entities);

        self.rebuild_runtime_state();
        created_entities
    }

    pub fn materialize_map_entity(&mut self, uid: EntityUid, map_id: MapId) -> bool {
        self.ensure_transform(uid);
        let transform_applied = self.apply_transform_state(uid, Self::map_transform_state(map_id));
        let map_applied = self.apply_map_component_state(
            uid,
            MapComponentState {
                map_id,
                lighting_enabled: true,
                map_paused: false,
            },
        );
        transform_applied || map_applied
    }

    pub fn materialize_grid_entity(
        &mut self,
        uid: EntityUid,
        map_id: MapId,
        parent_id: EntityUid,
        grid_id: crate::GridId,
        chunk_size: u16,
        local_position: Vector2,
        rotation: keisan::Angle,
    ) -> bool {
        self.ensure_transform(uid);
        let grid_applied = self.apply_map_grid_component_state(
            uid,
            MapGridComponentState {
                grid_index: grid_id,
                chunk_size,
            },
        );
        let transform_applied = self.apply_transform_state(
            uid,
            Self::grid_transform_state(map_id, parent_id, grid_id, local_position, rotation),
        );
        grid_applied || transform_applied
    }

    pub fn ensure_grid_entity_shell(&mut self, grid_id: crate::GridId) -> EntityUid {
        if let Some(uid) = self.grid_entity_for(grid_id) {
            return uid;
        }

        let uid = self.create_entity_uninitialized(None);
        let _ = self.materialize_grid_entity(
            uid,
            MapId::NULLSPACE,
            EntityUid::INVALID,
            grid_id,
            16,
            Vector2::ZERO,
            Angle::ZERO,
        );
        uid
    }

    pub fn apply_grid_chunk_tile_data(
        &mut self,
        grid_id: crate::GridId,
        chunk_index: Vector2i,
        tile_data: &[Tile],
    ) -> bool {
        let Some(uid) = self.grid_entity_for(grid_id) else {
            return false;
        };

        let Some(grid_component) = self.map_grid_components.get(&uid) else {
            return false;
        };
        let chunk_size = grid_component.chunk_size.max(1) as i32;
        let Some(grid) = self.map_grids.get_mut(&uid) else {
            return false;
        };

        let mut changed = Vec::with_capacity(tile_data.len());
        for (offset, tile) in tile_data.iter().copied().enumerate() {
            let offset = offset as i32;
            let local_x = offset % chunk_size;
            let local_y = offset / chunk_size;
            let indices = Vector2i::new(
                chunk_index.x * chunk_size + local_x,
                chunk_index.y * chunk_size + local_y,
            );
            changed.push((indices, tile));
        }

        grid.set_tiles(&changed);
        true
    }

    pub fn apply_game_state_map_data(&mut self, map_data: &GameStateMapData) {
        for grid_id in &map_data.deleted_grids {
            if let Some(uid) = self.grid_entity_for(*grid_id) {
                self.queue_delete_entity(uid);
                self.flush_queued_deletions();
            }
        }

        for (grid_id, datum) in &map_data.grid_data {
            let map_entity = if let Some(uid) = self.map_entity_for(datum.coordinates.map_id) {
                uid
            } else {
                let uid = self.create_entity_uninitialized_as_map(None, datum.coordinates.map_id);
                let _ = self.initialize_entity(uid);
                uid
            };
            let grid_created = self.grid_entity_for(*grid_id).is_none();
            let uid = self.ensure_grid_entity_shell(*grid_id);
            let chunk_size = self
                .map_grid_components
                .get(&uid)
                .map(|component| component.chunk_size)
                .unwrap_or(16);
            let _ = self.materialize_grid_entity(
                uid,
                datum.coordinates.map_id,
                map_entity,
                *grid_id,
                chunk_size,
                datum.coordinates.position,
                datum.angle,
            );
            if grid_created {
                let _ = self.initialize_entity(uid);
            }

            for chunk in &datum.chunk_data {
                if let Some(tile_data) = &chunk.tile_data {
                    let _ = self.apply_grid_chunk_tile_data(*grid_id, chunk.index, tile_data);
                } else if let Some(grid) = self.map_grids.get_mut(&uid) {
                    grid.remove_chunk(chunk.index);
                }
            }
        }
    }

    pub fn remove_map_component(&mut self, uid: EntityUid) -> bool {
        let Some(map_id) = self
            .map_components
            .get(&uid)
            .map(|component| component.world_map)
        else {
            return false;
        };

        self.clear_map_component_runtime_state(uid);
        let removed_component = self.map_components.remove(&uid);
        let removed_component_exists = removed_component.is_some();
        if let Some(mut component) = removed_component {
            self.queue_component_shutdown_and_remove(uid, &mut component.base);
        }
        let removed = removed_component_exists || map_id == MapId::NULLSPACE;
        removed
    }

    pub fn remove_joint_component(&mut self, uid: EntityUid) -> bool {
        let Some(mut component) = self.joint_components.remove(&uid) else {
            return false;
        };

        let mut affected_maps = HashSet::new();
        if let Some(map_id) = self.transforms.get(&uid).map(|transform| transform.map_id) {
            affected_maps.insert(map_id);
        }

        for joint in component.joints.into_values() {
            let body_a = EntityUid::new(joint.body_a_uid);
            let body_b = EntityUid::new(joint.body_b_uid);
            let other = if body_a == uid { body_b } else { body_a };
            self.queue_entity_physics_runtime(
                uid,
                crate::PhysicsRuntimeEvent::JointRemoved(crate::JointRemovedEvent {
                    body_a,
                    body_b,
                    joint_id: joint.id.clone(),
                }),
            );

            if let Some(other_component) = self.joint_components.get_mut(&other) {
                let _ = other_component.remove_joint(&joint.id);
                other_component.base.last_modified_tick = self.current_tick;
            }
            if self.entity_exists(other) {
                self.dirty_entity(other);
            }

            if let Some(map_id) = self
                .transforms
                .get(&other)
                .map(|transform| transform.map_id)
            {
                affected_maps.insert(map_id);
            }
        }

        self.refresh_map_physics_runtime_many(affected_maps);

        self.queue_component_shutdown_and_remove(uid, &mut component.base);
        true
    }

    pub fn remove_physics_component(&mut self, uid: EntityUid) -> bool {
        let previous = self
            .physics
            .get(&uid)
            .map(|body| (body.awake, body.can_collide));
        let Some(mut component) = self.physics.remove(&uid) else {
            return false;
        };

        if let Some((awake, can_collide)) = previous {
            if awake {
                self.queue_entity_physics_runtime(
                    uid,
                    crate::PhysicsRuntimeEvent::Sleep(crate::PhysicsSleepMessage { body: uid }),
                );
            }
            if can_collide {
                self.queue_entity_physics_runtime(
                    uid,
                    crate::PhysicsRuntimeEvent::CollisionChange(crate::CollisionChangeMessage {
                        owner: uid,
                        can_collide: false,
                    }),
                );
            }
        }
        let _ = self.remove_joint_component(uid);
        self.reconcile_entity_physics_runtime(uid);
        self.queue_component_shutdown_and_remove(uid, &mut component.base);
        true
    }

    pub fn remove_fixtures_component(&mut self, uid: EntityUid) -> bool {
        if !self.fixtures.contains_key(&uid) {
            return false;
        }
        let removed = self.remove_component_storage_with_lifecycle(uid, "FixturesComponent");
        if removed {
            self.reconcile_entity_physics_runtime(uid);
        }
        removed
    }

    pub fn remove_appearance_component(&mut self, uid: EntityUid) -> bool {
        self.remove_component_storage_with_lifecycle(uid, "AppearanceComponent")
    }

    pub fn remove_lookup_component(&mut self, uid: EntityUid) -> bool {
        if !self.entity_lookups.contains_key(&uid) {
            return false;
        }
        self.clear_lookup_runtime_support(uid);
        self.remove_component_storage_with_lifecycle(uid, "EntityLookupComponent")
    }

    pub fn remove_broadphase_component(&mut self, uid: EntityUid) -> bool {
        self.remove_component_storage_with_lifecycle(uid, "BroadphaseComponent")
    }

    pub fn remove_physics_map_component(&mut self, uid: EntityUid) -> bool {
        self.remove_component_storage_with_lifecycle(uid, "SharedPhysicsMapComponent")
    }

    pub fn remove_collision_wake_component(&mut self, uid: EntityUid) -> bool {
        if !self.collision_wakes.contains_key(&uid) {
            return false;
        }
        let removed = self.remove_component_storage_with_lifecycle(uid, "CollisionWakeComponent");
        if !removed {
            return false;
        }
        self.clear_collision_wake_runtime_support(uid);
        true
    }

    pub fn remove_ignore_pause_component(&mut self, uid: EntityUid) -> bool {
        let removed = self.remove_component_storage_with_lifecycle(uid, "IgnorePauseComponent");
        if !removed {
            return false;
        }
        self.sync_entity_pause_from_current_map(uid);
        true
    }

    pub fn remove_collide_on_anchor_component(&mut self, uid: EntityUid) -> bool {
        if !self.collide_on_anchors.contains_key(&uid) {
            return false;
        }
        self.reconcile_entity_physics_runtime(uid);
        self.remove_component_storage_with_lifecycle(uid, "CollideOnAnchorComponent")
    }

    pub fn remove_timer_component(&mut self, uid: EntityUid) -> bool {
        self.remove_component_storage_with_lifecycle(uid, "TimerComponent")
    }

    pub fn set_parent(&mut self, uid: EntityUid, parent: EntityUid) -> bool {
        let Some((old_parent, previous_map)) = self
            .transforms
            .get(&uid)
            .map(|transform| (transform.parent, transform.map_id))
        else {
            return false;
        };

        if old_parent == parent {
            return true;
        }

        let previous_map_velocities =
            self.capture_preserved_map_velocities_for_parent_change(uid, old_parent, parent);

        self.rewire_transform_parent(uid, old_parent, parent);

        if let Some(transform) = self.transforms.get_mut(&uid) {
            transform.parent = parent;
            transform.rebuild_for_manager();
        }

        self.restore_preserved_map_velocities(uid, previous_map_velocities);
        self.reconcile_transform_runtime(uid, Some(previous_map));
        self.queue_parent_changed_if_needed(uid, old_parent, parent);
        true
    }

    pub fn apply_transform_state(
        &mut self,
        uid: EntityUid,
        state: TransformComponentState,
    ) -> bool {
        let Some((old_parent, old_anchored, previous_map)) = self
            .transforms
            .get(&uid)
            .map(|transform| (transform.parent, transform.anchored, transform.map_id))
        else {
            return false;
        };

        let previous_map_velocities = self.capture_preserved_map_velocities_for_parent_change(
            uid,
            old_parent,
            state.parent_id,
        );

        self.rewire_transform_parent(uid, old_parent, state.parent_id);

        let Some(transform) = self.transforms.get_mut(&uid) else {
            return false;
        };
        transform.handle_transform_state(state);
        let _ = transform;
        self.restore_preserved_map_velocities(uid, previous_map_velocities);
        self.reconcile_transform_runtime(uid, Some(previous_map));
        self.queue_parent_changed_if_needed(uid, old_parent, state.parent_id);
        self.queue_anchor_changed_if_needed(uid, old_anchored, state.anchored);
        true
    }

    pub fn set_spatial_metadata(
        &mut self,
        uid: EntityUid,
        map_id: MapId,
        grid_id: crate::GridId,
    ) -> bool {
        let Some(transform) = self.transforms.get_mut(&uid) else {
            return false;
        };
        if transform.map_id == map_id && transform.grid_id == grid_id {
            return false;
        }
        let previous_map = transform.map_id;
        transform.map_id = map_id;
        transform.grid_id = grid_id;
        let _ = transform;
        self.reconcile_transform_runtime(uid, Some(previous_map));
        true
    }

    pub fn set_local_transform(
        &mut self,
        uid: EntityUid,
        local_position: Vector2,
        local_rotation: keisan::Angle,
    ) -> Option<crate::MoveEvent> {
        let (old_coordinates, new_coordinates) = {
            let transform = self.transforms.get_mut(&uid)?;
            let applied_rotation = if transform.no_local_rotation {
                transform.local_rotation
            } else {
                local_rotation
            };
            if transform.local_position == local_position
                && transform.local_rotation == applied_rotation
            {
                return None;
            }

            let old_coordinates = transform.coordinates();
            transform.local_position = local_position;
            transform.local_rotation = applied_rotation;
            transform.rebuild_for_manager();
            let new_coordinates = transform.coordinates();
            (old_coordinates, new_coordinates)
        };
        let _ = self.sync_map_grid_runtime(uid);
        Some(crate::MoveEvent {
            sender: uid,
            old_position: old_coordinates,
            new_position: new_coordinates,
        })
    }

    pub fn offset_local_transform(
        &mut self,
        uid: EntityUid,
        delta: Vector2,
        angular_delta: keisan::Angle,
    ) -> Option<crate::MoveEvent> {
        let (position, rotation) = self.transforms.get(&uid).map(|transform| {
            (
                transform.local_position + delta,
                transform.local_rotation + angular_delta,
            )
        })?;
        self.set_local_transform(uid, position, rotation)
    }

    pub fn set_anchored(&mut self, uid: EntityUid, anchored: bool) -> bool {
        let Some(transform) = self.transforms.get_mut(&uid) else {
            return false;
        };
        if transform.anchored == anchored {
            return false;
        }
        let event = transform.set_anchored(anchored);
        let _ = transform;
        self.reconcile_entity_physics_runtime(uid);
        let mut event = event;
        self.raise_transform_runtime_event(uid, &mut event);
        true
    }

    fn capture_map_velocities(&self, uid: EntityUid) -> Option<(Vector2, f32)> {
        self.physics
            .contains_key(&uid)
            .then(|| self.map_velocities(uid))
    }

    fn capture_preserved_map_velocities_for_parent_change(
        &self,
        uid: EntityUid,
        old_parent: EntityUid,
        new_parent: EntityUid,
    ) -> Option<(Vector2, f32)> {
        (old_parent != new_parent)
            .then(|| self.capture_map_velocities(uid))
            .flatten()
    }

    fn restore_preserved_map_velocities(
        &mut self,
        uid: EntityUid,
        previous: Option<(Vector2, f32)>,
    ) {
        let Some(previous) = previous else {
            return;
        };

        let current = self.map_velocities(uid);
        let Some(body) = self.physics.get_mut(&uid) else {
            return;
        };

        body.set_linear_velocity(body.linear_velocity + previous.0 - current.0);
        body.set_angular_velocity(body.angular_velocity + previous.1 - current.1);
    }

    fn rewire_transform_parent(
        &mut self,
        uid: EntityUid,
        old_parent: EntityUid,
        new_parent: EntityUid,
    ) {
        if old_parent == new_parent {
            return;
        }

        if old_parent.is_valid() {
            if let Some(transform) = self.transforms.get_mut(&old_parent) {
                transform.children.remove(&uid);
            }
        }

        if new_parent.is_valid() {
            if let Some(transform) = self.transforms.get_mut(&new_parent) {
                transform.children.insert(uid);
            }
        }
    }

    fn queue_parent_changed_if_needed(
        &mut self,
        uid: EntityUid,
        old_parent: EntityUid,
        new_parent: EntityUid,
    ) {
        if old_parent == new_parent {
            return;
        }

        let mut event = crate::EntParentChangedMessage {
            entity: uid,
            old_parent: old_parent.is_valid().then_some(old_parent),
        };
        self.raise_transform_runtime_event(uid, &mut event);
    }

    fn queue_anchor_changed_if_needed(
        &mut self,
        uid: EntityUid,
        old_anchored: bool,
        new_anchored: bool,
    ) {
        if old_anchored == new_anchored {
            return;
        }

        let mut event = crate::AnchorStateChangedEvent {
            entity: uid,
            anchored: new_anchored,
        };
        self.raise_transform_runtime_event(uid, &mut event);
    }

    fn sync_entity_metadata_map_id(&mut self, uid: EntityUid) {
        let map_id = self
            .map_components
            .get(&uid)
            .map(|component| component.world_map)
            .or_else(|| self.transforms.get(&uid).map(|transform| transform.map_id))
            .unwrap_or(MapId::NULLSPACE);
        if let Some(meta) = self.metadata.get_mut(&uid) {
            meta.map_id = map_id;
        }
    }

    fn sync_entity_pause_from_current_map(&mut self, uid: EntityUid) {
        let map_id = self
            .map_components
            .get(&uid)
            .map(|component| component.world_map)
            .or_else(|| self.transforms.get(&uid).map(|transform| transform.map_id))
            .unwrap_or(MapId::NULLSPACE);
        let paused = self.is_map_paused(map_id);
        let _ = self.set_entity_paused(uid, paused);
    }

    fn sync_map_pause_recursive(&mut self, uid: EntityUid) {
        self.sync_entity_pause_from_current_map(uid);
        let children = self
            .transforms
            .get(&uid)
            .map(|transform| transform.children.iter().copied().collect::<Vec<_>>())
            .unwrap_or_default();
        for child in children {
            self.sync_map_pause_recursive(child);
        }
    }

    fn run_map_init_recursive(&mut self, uid: EntityUid) {
        let _ = self.run_map_init(uid);
        let children = self
            .transforms
            .get(&uid)
            .map(|transform| transform.children.iter().copied().collect::<Vec<_>>())
            .unwrap_or_default();
        for child in children {
            self.run_map_init_recursive(child);
        }
    }

    fn propagate_transform_spatial_metadata(
        &mut self,
        uid: EntityUid,
        inherited_map: MapId,
        inherited_grid: crate::GridId,
    ) {
        let (parent_valid, current_map, current_grid) = self
            .transforms
            .get(&uid)
            .map(|transform| {
                (
                    transform.parent.is_valid(),
                    transform.map_id,
                    transform.grid_id,
                )
            })
            .unwrap_or((false, MapId::NULLSPACE, crate::GridId::INVALID));
        let mut map_id = if current_map != MapId::NULLSPACE {
            current_map
        } else {
            inherited_map
        };
        let mut grid_id = if current_grid.is_valid() {
            current_grid
        } else {
            inherited_grid
        };

        if let Some(map) = self.map_components.get(&uid) {
            map_id = map.world_map;
            grid_id = crate::GridId::INVALID;
        } else if let Some(grid) = self.map_grid_components.get(&uid) {
            if map_id == MapId::NULLSPACE {
                map_id = inherited_map;
            }
            grid_id = grid.grid_index;
        } else if parent_valid {
            map_id = inherited_map;
            grid_id = inherited_grid;
        }

        let _ = self.set_spatial_metadata(uid, map_id, grid_id);
        self.sync_entity_pause_from_current_map(uid);

        let children = self.children_of(uid);
        for child in children {
            self.propagate_transform_spatial_metadata(child, map_id, grid_id);
        }
    }

    fn detach_child_transform(
        &mut self,
        uid: EntityUid,
        map_id: MapId,
        grid_id: crate::GridId,
        world_position: Vector2,
        world_rotation: keisan::Angle,
    ) {
        let Some((anchored, no_local_rotation)) = self
            .transforms
            .get(&uid)
            .map(|transform| (transform.anchored, transform.no_local_rotation))
        else {
            return;
        };
        let _ = self.apply_transform_state(
            uid,
            TransformComponentState {
                local_position: world_position,
                rotation: world_rotation,
                parent_id: EntityUid::INVALID,
                map_id,
                grid_id,
                no_local_rotation,
                anchored,
            },
        );
    }

    fn collect_child_world_states(
        &self,
        uid: EntityUid,
    ) -> Vec<(EntityUid, MapId, crate::GridId, Vector2, keisan::Angle)> {
        self.transforms
            .get(&uid)
            .map(|transform| {
                transform
                    .children
                    .iter()
                    .copied()
                    .filter_map(|child| {
                        self.world_transform(child).map(|world| {
                            (
                                child,
                                world.map_id,
                                world.grid_id,
                                world.world_position,
                                world.world_rotation,
                            )
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    fn detach_children_preserving_world_state(&mut self, uid: EntityUid) {
        for (child, map_id, grid_id, world_position, world_rotation) in
            self.collect_child_world_states(uid)
        {
            self.detach_child_transform(child, map_id, grid_id, world_position, world_rotation);
        }
    }

    fn remove_from_parent_transform_tree(&mut self, uid: EntityUid) {
        let parent = self.transforms.get(&uid).map(|transform| transform.parent);
        if let Some(parent) = parent.filter(|parent| parent.is_valid()) {
            if let Some(parent_transform) = self.transforms.get_mut(&parent) {
                parent_transform.children.remove(&uid);
            }
        }
    }

    fn clear_transform_runtime_state(&mut self, uid: EntityUid) {
        self.remove_lookup_from_entity_tree(uid, false);
        self.remove_from_parent_transform_tree(uid);
        self.detach_children_preserving_world_state(uid);
    }

    fn remove_map_runtime_support(&mut self, uid: EntityUid) {
        let _ = self.remove_broadphase_component(uid);
        let _ = self.remove_physics_map_component(uid);
    }

    fn clear_lookup_runtime_support(&mut self, uid: EntityUid) {
        self.remove_lookup_from_entity_tree(uid, false);
    }

    fn clear_collision_wake_runtime_support(&mut self, uid: EntityUid) {
        if let Some(body) = self.physics.get_mut(&uid) {
            body.can_collide = true;
            body.base.last_modified_tick = self.current_tick;
            self.dirty_entity(uid);
        }
    }

    fn remove_map_grid_runtime_support(&mut self, uid: EntityUid) -> bool {
        let removed_grid = self.map_grids.remove(&uid).is_some();
        if removed_grid {
            self.remove_lookup_from_entity_tree(uid, true);
            if let Some(transform) = self.transforms.get_mut(&uid) {
                transform.grid_id = crate::GridId::INVALID;
            }
        }
        removed_grid
    }

    fn detach_direct_children_from_map(&mut self, uid: EntityUid) {
        let children = self
            .transforms
            .iter()
            .filter_map(|(child, transform)| (transform.parent == uid).then_some(*child))
            .collect::<Vec<_>>();
        for child in children {
            let world = self.world_transform(child);
            if let Some(world) = world {
                self.detach_child_transform(
                    child,
                    MapId::NULLSPACE,
                    crate::GridId::INVALID,
                    world.world_position,
                    world.world_rotation,
                );
            } else if let Some(transform) = self.transforms.get_mut(&child) {
                transform.parent = EntityUid::INVALID;
                transform.map_id = MapId::NULLSPACE;
                transform.grid_id = crate::GridId::INVALID;
                transform.rebuild_for_manager();
            }
            if let Some(grid) = self.map_grids.get_mut(&child) {
                grid.parent_map_id = MapId::NULLSPACE;
            }
        }
    }

    fn clear_map_component_runtime_state(&mut self, uid: EntityUid) {
        self.detach_direct_children_from_map(uid);
        if let Some(transform) = self.transforms.get_mut(&uid) {
            transform.map_id = MapId::NULLSPACE;
        }
        self.remove_map_runtime_support(uid);
        if let Some(meta) = self.metadata.get_mut(&uid) {
            meta.map_id = MapId::NULLSPACE;
        }
    }

    pub fn sync_map_grid_runtime(&mut self, uid: EntityUid) -> bool {
        let Some(component) = self.map_grid_components.get(&uid) else {
            self.map_grids.remove(&uid);
            return false;
        };

        let grid_index = component.grid_index;
        let chunk_size = component.chunk_size;
        let tile_size = component
            .grid
            .as_ref()
            .map(|grid| grid.tile_size)
            .unwrap_or(1);
        let (parent_map_id, world_position, world_rotation) = self
            .transforms
            .get(&uid)
            .map(|transform| {
                let (position, rotation, _) = transform.get_world_position_rotation_matrix(self);
                (transform.map_id, position, rotation)
            })
            .unwrap_or((MapId::NULLSPACE, Vector2::ZERO, keisan::Angle::ZERO));

        let runtime = self.map_grids.entry(uid).or_insert_with(|| {
            let mut grid = MapGrid::new(parent_map_id, uid, grid_index, chunk_size);
            grid.tile_size = tile_size;
            grid.world_position = world_position;
            grid.world_rotation = world_rotation;
            grid
        });
        runtime.parent_map_id = parent_map_id;
        runtime.grid_entity_id = uid;
        runtime.index = grid_index;
        runtime.tile_size = tile_size;
        runtime.world_position = world_position;
        runtime.world_rotation = world_rotation;
        true
    }

    pub fn refresh_entity_physics_runtime(&mut self, uid: EntityUid) {
        self.refresh_entity_physics_runtime_many(std::iter::once(uid));
    }

    pub fn refresh_entity_physics_runtime_many<I>(&mut self, entities: I)
    where
        I: IntoIterator<Item = EntityUid>,
    {
        let unique_entities = entities.into_iter().collect::<HashSet<_>>();
        let mut affected_maps = HashSet::new();

        for uid in unique_entities {
            let _ = self.refresh_collision_wake(uid);
            let _ = self.refresh_collide_on_anchor(uid);
            if self.transforms.contains_key(&uid) {
                self.refresh_entity_lookup_subtree(uid);
            }

            if let Some(map_id) = self.transforms.get(&uid).map(|transform| transform.map_id) {
                affected_maps.insert(map_id);
            } else {
                affected_maps.extend(
                    self.map_components
                        .values()
                        .map(|component| component.world_map),
                );
            }
        }

        self.refresh_map_physics_runtime_many(affected_maps);
    }

    fn queue_entity_physics_runtime(&mut self, uid: EntityUid, event: crate::PhysicsRuntimeEvent) {
        let _ = self.queue_entity_runtime_event(uid, event);
    }

    fn reconcile_entity_physics_runtime(&mut self, uid: EntityUid) {
        self.refresh_entity_physics_runtime(uid);
    }

    fn physics_runtime_reconcile_targets(event: &crate::PhysicsRuntimeEvent) -> Vec<EntityUid> {
        match event {
            crate::PhysicsRuntimeEvent::Wake(crate::PhysicsWakeMessage { body })
            | crate::PhysicsRuntimeEvent::Sleep(crate::PhysicsSleepMessage { body }) => vec![*body],
            crate::PhysicsRuntimeEvent::JointAdded(crate::JointAddedEvent {
                body_a,
                body_b,
                ..
            })
            | crate::PhysicsRuntimeEvent::JointRemoved(crate::JointRemovedEvent {
                body_a,
                body_b,
                ..
            }) => {
                vec![*body_a, *body_b]
            }
            crate::PhysicsRuntimeEvent::CollisionChange(_) => Vec::new(),
        }
    }

    pub fn refresh_map_physics_runtime_many<I>(&mut self, map_ids: I)
    where
        I: IntoIterator<Item = MapId>,
    {
        let unique_maps = map_ids
            .into_iter()
            .filter(|map_id| *map_id != MapId::NULLSPACE)
            .collect::<HashSet<_>>();
        for map_id in unique_maps {
            self.refresh_map_physics_runtime(map_id);
        }
    }

    pub fn refresh_transform_spatial_metadata(&mut self) {
        let roots = self
            .transforms
            .iter()
            .filter_map(|(uid, transform)| (!transform.parent.is_valid()).then_some(*uid))
            .collect::<Vec<_>>();
        for root in roots {
            self.propagate_transform_spatial_metadata(
                root,
                MapId::NULLSPACE,
                crate::GridId::INVALID,
            );
        }
    }

    pub fn refresh_all_map_grid_runtimes(&mut self) {
        let grid_uids = self.map_grid_components.keys().copied().collect::<Vec<_>>();
        for uid in grid_uids {
            let _ = self.sync_map_grid_runtime(uid);
        }
    }

    pub fn rebuild_lookup_runtime(&mut self) {
        for lookup in self.entity_lookups.values_mut() {
            lookup.clear();
        }

        let roots = self
            .transforms
            .iter()
            .filter_map(|(uid, transform)| (!transform.parent.is_valid()).then_some(*uid))
            .collect::<Vec<_>>();
        for root in roots {
            self.refresh_entity_lookup_subtree(root);
        }
    }

    pub fn rebuild_map_physics_runtime(&mut self) {
        let collision_wake_entities = self.collision_wakes.keys().copied().collect::<Vec<_>>();
        for uid in collision_wake_entities {
            let _ = self.refresh_collision_wake(uid);
        }
        let collide_on_anchor_entities =
            self.collide_on_anchors.keys().copied().collect::<Vec<_>>();
        for uid in collide_on_anchor_entities {
            let _ = self.refresh_collide_on_anchor(uid);
        }
        let map_ids = self
            .map_components
            .values()
            .map(|component| component.world_map)
            .collect::<HashSet<_>>();
        self.refresh_map_physics_runtime_many(map_ids);
    }

    pub fn rebuild_runtime_state(&mut self) {
        self.refresh_transform_spatial_metadata();
        self.refresh_all_map_grid_runtimes();
        self.rebuild_lookup_runtime();
        self.rebuild_map_physics_runtime();
    }

    pub fn queue_map_runtime_event(
        &mut self,
        map_id: MapId,
        event: crate::PhysicsRuntimeEvent,
    ) -> bool {
        if map_id == MapId::NULLSPACE {
            return false;
        }
        let Some(owner) = self.map_entity_for(map_id) else {
            return false;
        };
        let component_names = self.entity_component_names(owner);
        self.raise_concrete_physics_runtime_event(owner, &component_names, &event);
        self.ensure_physics_map(owner).queue_runtime_event(event);
        true
    }

    pub fn queue_entity_runtime_event(
        &mut self,
        uid: EntityUid,
        event: crate::PhysicsRuntimeEvent,
    ) -> bool {
        let component_names = self.entity_component_names(uid);
        self.raise_concrete_physics_runtime_event(uid, &component_names, &event);
        let reconcile_targets = Self::physics_runtime_reconcile_targets(&event);
        let map_id = self
            .transforms
            .get(&uid)
            .map(|transform| transform.map_id)
            .unwrap_or(MapId::NULLSPACE);
        let queued = self.queue_map_runtime_event(map_id, event);
        if !reconcile_targets.is_empty() {
            self.refresh_entity_physics_runtime_many(reconcile_targets);
        }
        self.process_subscription_side_effects();
        queued
    }

    pub fn queue_entity_event<E>(&mut self, event: E)
    where
        E: Into<crate::EntityRuntimeEvent>,
    {
        let mut event = event.into();
        if let Some(uid) = Self::entity_runtime_event_target(&event) {
            let component_names = self.entity_component_names(uid);
            self.raise_concrete_runtime_event(uid, &component_names, &mut event);
        } else {
            self.entity_sys_manager
                .raise_event(crate::EventSource::Local, &mut event);
        }
        self.process_direct_runtime_event_side_effects(&event);
        self.entity_runtime_events.push_back(event);
        self.process_subscription_side_effects();
    }

    pub fn drain_entity_runtime_events(&mut self) -> Vec<crate::EntityRuntimeEvent> {
        self.entity_runtime_events.drain(..).collect()
    }

    pub fn mark_appearance_dirty(&mut self, uid: EntityUid) -> bool {
        if !self.appearances.contains_key(&uid) {
            return false;
        }
        let tick = self.current_tick;
        if let Some(component) = self.appearances.get_mut(&uid) {
            component.base.last_modified_tick = tick;
        }
        self.dirty_entity(uid);
        self.appearance_dirty_components.insert(uid);
        true
    }

    pub fn get_appearance_state(&self, uid: EntityUid) -> Option<crate::AppearanceComponentState> {
        let component = self.appearances.get(&uid)?;
        Some(component.get_component_state())
    }

    pub fn clear_appearance_dirty(&mut self, uid: EntityUid) -> bool {
        let Some(component) = self.appearances.get_mut(&uid) else {
            return false;
        };
        component.clear_dirty();
        self.appearance_dirty_components.remove(&uid);
        true
    }

    pub fn drain_appearance_dirty(&mut self) -> Vec<EntityUid> {
        self.appearance_dirty_components.drain().collect()
    }

    pub fn raise_component_event<T>(&mut self, uid: EntityUid, component_name: &str, event: &mut T)
    where
        T: std::any::Any + Send + 'static,
    {
        self.entity_sys_manager
            .raise_component_event(uid, component_name, event);
        self.process_direct_component_event_side_effects(uid, component_name, event);
        self.process_subscription_side_effects();
    }

    fn raise_transform_runtime_event<T>(&mut self, uid: EntityUid, event: &mut T)
    where
        T: std::any::Any + Send + Clone + Into<crate::EntityRuntimeEvent> + 'static,
    {
        self.raise_component_event(uid, "TransformComponent", event);
        self.entity_runtime_events.push_back(event.clone().into());
    }

    fn process_direct_component_event_side_effects<T>(
        &mut self,
        uid: EntityUid,
        component_name: &str,
        event: &mut T,
    ) where
        T: std::any::Any + Send + 'static,
    {
        let event = event as &mut dyn std::any::Any;
        match component_name {
            "EntityLookupComponent" if event.is::<crate::ComponentAdd>() => {
                if self.transforms.contains_key(&uid) {
                    self.refresh_entity_lookup_subtree(uid);
                }
            }
            "TransformComponent" => {
                if let Some(move_event) = event.downcast_ref::<crate::MoveEvent>() {
                    if self.transforms.contains_key(&move_event.sender) {
                        self.refresh_entity_lookup_subtree(move_event.sender);
                    }
                } else if let Some(rotate_event) = event.downcast_ref::<crate::RotateEvent>() {
                    if self.transforms.contains_key(&rotate_event.sender) {
                        self.refresh_entity_lookup_subtree(rotate_event.sender);
                    }
                }
            }
            "MetaDataComponent" => {
                if let Some(paused_event) = event.downcast_ref::<crate::EntityPausedEvent>() {
                    if self.appearances.contains_key(&paused_event.entity) {
                        let _ = self.mark_appearance_dirty(paused_event.entity);
                    }
                }
            }
            _ => {}
        }
    }

    fn process_direct_runtime_event_side_effects(&mut self, event: &crate::EntityRuntimeEvent) {
        match event {
            crate::EntityRuntimeEvent::EntityPaused(ev) => {
                if self.appearances.contains_key(&ev.entity) {
                    let _ = self.mark_appearance_dirty(ev.entity);
                }
            }
            crate::EntityRuntimeEvent::EntityDeleted(ev) => {
                self.appearance_dirty_components.remove(&ev.entity);
            }
            crate::EntityRuntimeEvent::MapInit(ev) => {
                if self.appearances.contains_key(&ev.entity) {
                    let _ = self.mark_appearance_dirty(ev.entity);
                }
            }
            crate::EntityRuntimeEvent::ComponentLifecycle(ev)
                if ev.component == "AppearanceComponent" =>
            {
                match ev.stage {
                    crate::ComponentLifeStage::Added
                    | crate::ComponentLifeStage::Initialized
                    | crate::ComponentLifeStage::Running => {
                        if self.appearances.contains_key(&ev.owner) {
                            let _ = self.mark_appearance_dirty(ev.owner);
                        }
                    }
                    crate::ComponentLifeStage::Stopped | crate::ComponentLifeStage::Deleted => {
                        self.appearance_dirty_components.remove(&ev.owner);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn process_subscription_side_effects(&mut self) {
        if self.processing_subscription_side_effects {
            return;
        }

        self.processing_subscription_side_effects = true;
        self.processing_subscription_side_effects = false;
    }

    fn entity_runtime_event_target(event: &crate::EntityRuntimeEvent) -> Option<EntityUid> {
        match event {
            crate::EntityRuntimeEvent::EntityInitialized(ev) => Some(ev.entity),
            crate::EntityRuntimeEvent::EntityDeleted(ev) => Some(ev.entity),
            crate::EntityRuntimeEvent::EntityPaused(ev) => Some(ev.entity),
            crate::EntityRuntimeEvent::MapPaused(ev) => Some(ev.entity),
            crate::EntityRuntimeEvent::EntityTerminating(ev) => Some(ev.entity),
            crate::EntityRuntimeEvent::MapInit(ev) => Some(ev.entity),
            crate::EntityRuntimeEvent::ComponentLifecycle(ev) => Some(ev.owner),
            crate::EntityRuntimeEvent::ParentChanged(ev) => Some(ev.entity),
            crate::EntityRuntimeEvent::AnchorStateChanged(ev) => Some(ev.entity),
            crate::EntityRuntimeEvent::PhysicsInitialized(ev) => Some(ev.uid),
        }
    }

    fn raise_concrete_runtime_event(
        &mut self,
        uid: EntityUid,
        component_names: &[String],
        event: &mut crate::EntityRuntimeEvent,
    ) {
        match event {
            crate::EntityRuntimeEvent::EntityInitialized(ev) => {
                self.entity_sys_manager
                    .raise_local_event(uid, component_names, ev, true);
            }
            crate::EntityRuntimeEvent::EntityDeleted(ev) => {
                self.entity_sys_manager
                    .raise_local_event(uid, component_names, ev, true);
            }
            crate::EntityRuntimeEvent::EntityPaused(ev) => {
                self.entity_sys_manager
                    .raise_local_event(uid, component_names, ev, true);
            }
            crate::EntityRuntimeEvent::MapPaused(ev) => {
                self.entity_sys_manager
                    .raise_local_event(uid, component_names, ev, true);
            }
            crate::EntityRuntimeEvent::EntityTerminating(ev) => {
                self.entity_sys_manager
                    .raise_local_event(uid, component_names, ev, true);
            }
            crate::EntityRuntimeEvent::MapInit(ev) => {
                self.entity_sys_manager
                    .raise_local_event(uid, component_names, ev, true);
            }
            crate::EntityRuntimeEvent::ComponentLifecycle(ev) => {
                self.entity_sys_manager
                    .raise_local_event(uid, component_names, ev, true);
            }
            crate::EntityRuntimeEvent::ParentChanged(ev) => {
                self.entity_sys_manager
                    .raise_local_event(uid, component_names, ev, true);
            }
            crate::EntityRuntimeEvent::AnchorStateChanged(ev) => {
                self.entity_sys_manager
                    .raise_local_event(uid, component_names, ev, true);
            }
            crate::EntityRuntimeEvent::PhysicsInitialized(ev) => {
                self.entity_sys_manager
                    .raise_local_event(uid, component_names, ev, true);
            }
        }
    }

    fn raise_concrete_physics_runtime_event(
        &mut self,
        uid: EntityUid,
        component_names: &[String],
        event: &crate::PhysicsRuntimeEvent,
    ) {
        match event {
            crate::PhysicsRuntimeEvent::Wake(ev) => {
                let mut ev = *ev;
                self.entity_sys_manager
                    .raise_local_event(uid, component_names, &mut ev, true);
            }
            crate::PhysicsRuntimeEvent::Sleep(ev) => {
                let mut ev = *ev;
                self.entity_sys_manager
                    .raise_local_event(uid, component_names, &mut ev, true);
            }
            crate::PhysicsRuntimeEvent::CollisionChange(ev) => {
                let mut ev = *ev;
                self.entity_sys_manager
                    .raise_local_event(uid, component_names, &mut ev, true);
            }
            crate::PhysicsRuntimeEvent::JointAdded(ev) => {
                let mut ev = ev.clone();
                self.entity_sys_manager
                    .raise_local_event(uid, component_names, &mut ev, true);
            }
            crate::PhysicsRuntimeEvent::JointRemoved(ev) => {
                let mut ev = ev.clone();
                self.entity_sys_manager
                    .raise_local_event(uid, component_names, &mut ev, true);
            }
        }
    }

    fn entity_component_lifecycle_state(&self, uid: EntityUid) -> (bool, bool) {
        self.metadata
            .get(&uid)
            .map(|meta| {
                (
                    meta.entity_initializing() || meta.entity_initialized(),
                    meta.entity_initialized(),
                )
            })
            .unwrap_or((false, false))
    }

    fn lifecycle_component_name(component_name: &str) -> &str {
        match component_name {
            "Appearance" => "AppearanceComponent",
            "CollisionWake" => "CollisionWakeComponent",
            "CollideOnAnchor" => "CollideOnAnchorComponent",
            "Fixtures" => "FixturesComponent",
            other => other,
        }
    }

    fn prepare_new_component_base(base: &mut Component, owner: EntityUid, tick: GameTick) {
        base.owner = owner;
        base.creation_tick = tick;
        base.last_modified_tick = tick;
    }

    fn initialize_new_component_base(
        base: &mut Component,
        owner: EntityUid,
        tick: GameTick,
        _start_running: bool,
    ) {
        Self::prepare_new_component_base(base, owner, tick);
    }

    fn entity_initialized_for_component_lifecycle(&self, uid: EntityUid) -> bool {
        self.metadata
            .get(&uid)
            .map(|meta| meta.entity_initialized())
            .unwrap_or(false)
    }

    fn queue_new_component_lifecycle(
        &mut self,
        owner: EntityUid,
        component_name: &str,
        start_running: bool,
    ) {
        let (initialize, running) = self.entity_component_lifecycle_state(owner);
        let Some(event_name) = self.component_base_name(owner, component_name) else {
            return;
        };

        if let Some(base) = self.component_base_mut(owner, component_name) {
            base.life_stage = crate::ComponentLifeStage::Adding;
        }
        self.raise_component_event(owner, &event_name, &mut crate::ComponentAdd);
        if let Some(base) = self.component_base_mut(owner, component_name) {
            base.on_add();
        }
        self.queue_component_stage(owner, component_name, crate::ComponentLifeStage::Added);

        if initialize {
            if let Some(base) = self.component_base_mut(owner, component_name) {
                base.life_stage = crate::ComponentLifeStage::Initializing;
            }
            self.raise_component_event(owner, &event_name, &mut crate::ComponentInit);
            if let Some(base) = self.component_base_mut(owner, component_name) {
                base.initialize();
            }
            self.queue_component_stage(
                owner,
                component_name,
                crate::ComponentLifeStage::Initialized,
            );
            self.process_initialized_component_runtime(owner, &event_name);
        }

        if running && start_running {
            if let Some(base) = self.component_base_mut(owner, component_name) {
                base.life_stage = crate::ComponentLifeStage::Starting;
            }
            self.raise_component_event(owner, &event_name, &mut crate::ComponentStartup);
            if let Some(base) = self.component_base_mut(owner, component_name) {
                base.startup();
            }
            self.queue_component_stage(owner, component_name, crate::ComponentLifeStage::Running);
            self.process_started_component_runtime(owner, &event_name);
        }
    }

    fn queue_component_stage(
        &mut self,
        owner: EntityUid,
        component: &str,
        stage: crate::ComponentLifeStage,
    ) {
        self.queue_entity_event(crate::ComponentLifecycleEvent {
            owner,
            component: component.to_string(),
            stage,
        });
    }

    fn process_initialized_component_runtime(&mut self, owner: EntityUid, event_name: &str) {
        match event_name {
            "Physics" => {
                self.queue_entity_event(crate::PhysicsInitializedEvent { uid: owner });
                self.refresh_entity_physics_runtime(owner);
            }
            "SharedPhysicsMapComponent" => {
                if let Some(map_id) = self
                    .map_components
                    .get(&owner)
                    .map(|component| component.world_map)
                {
                    self.refresh_map_physics_runtime_many(std::iter::once(map_id));
                }
            }
            _ => {}
        }
    }

    fn process_started_component_runtime(&mut self, owner: EntityUid, event_name: &str) {
        match event_name {
            "CollisionWake" | "CollideOnAnchor" => self.reconcile_entity_physics_runtime(owner),
            _ => {}
        }
    }

    fn queue_component_shutdown_and_remove(&mut self, owner: EntityUid, base: &mut Component) {
        let name = base.name.clone();
        let component_name = Self::lifecycle_component_name(&name).to_string();
        if base.running() {
            base.life_stage = crate::ComponentLifeStage::Stopping;
            self.raise_component_event(owner, &name, &mut crate::ComponentShutdown);
            base.shutdown();
            self.queue_component_stage(owner, &component_name, crate::ComponentLifeStage::Stopped);
        }
        if base.life_stage != crate::ComponentLifeStage::PreAdd && !base.deleted() {
            base.life_stage = crate::ComponentLifeStage::Removing;
            self.raise_component_event(owner, &name, &mut crate::ComponentRemove);
            base.on_remove();
            self.queue_component_stage(owner, &component_name, crate::ComponentLifeStage::Deleted);
        }
    }

    fn initialize_existing_components_for_entity(&mut self, uid: EntityUid) {
        for component_name in Self::INIT_AND_START_COMPONENT_ORDER {
            self.initialize_component_for_entity(uid, component_name);
        }
    }

    fn initialize_component_for_entity(&mut self, uid: EntityUid, component_name: &str) {
        let Some(event_name) = self.component_base_name(uid, component_name) else {
            return;
        };
        let should_initialize = self
            .component_base(uid, component_name)
            .map(|base| !base.initialized())
            .unwrap_or(false);
        if should_initialize {
            if let Some(base) = self.component_base_mut(uid, component_name) {
                base.life_stage = crate::ComponentLifeStage::Initializing;
            }
            self.raise_component_event(uid, &event_name, &mut crate::ComponentInit);
            if let Some(base) = self.component_base_mut(uid, component_name) {
                base.initialize();
            }
            self.queue_component_stage(uid, component_name, crate::ComponentLifeStage::Initialized);
            self.process_initialized_component_runtime(uid, &event_name);
        }
    }

    fn start_existing_components_for_entity(&mut self, uid: EntityUid) {
        for component_name in Self::INIT_AND_START_COMPONENT_ORDER {
            self.start_component_for_entity(uid, component_name);
        }
    }

    fn remove_component_storage_with_lifecycle(
        &mut self,
        uid: EntityUid,
        component_name: &str,
    ) -> bool {
        let removed = match component_name {
            "AppearanceComponent" => self
                .appearances
                .remove(&uid)
                .map(|component| component.base),
            "TransformComponent" => self.transforms.remove(&uid).map(|component| component.base),
            "EntityLookupComponent" => self
                .entity_lookups
                .remove(&uid)
                .map(|component| component.base),
            "JointComponent" => self
                .joint_components
                .remove(&uid)
                .map(|component| component.base),
            "IgnorePauseComponent" => self
                .ignore_pauses
                .remove(&uid)
                .map(|component| component.base),
            "CollisionWakeComponent" => self
                .collision_wakes
                .remove(&uid)
                .map(|component| component.base),
            "CollideOnAnchorComponent" => self
                .collide_on_anchors
                .remove(&uid)
                .map(|component| component.base),
            "TimerComponent" => self.timers.remove(&uid).map(|component| component.base),
            "Physics" => self.physics.remove(&uid).map(|component| component.base),
            "FixturesComponent" => self.fixtures.remove(&uid).map(|component| component.base),
            "BroadphaseComponent" => self
                .broadphases
                .remove(&uid)
                .map(|component| component.base),
            "SharedPhysicsMapComponent" => self
                .physics_maps
                .remove(&uid)
                .map(|component| component.base),
            "MapComponent" => self
                .map_components
                .remove(&uid)
                .map(|component| component.base),
            "MapGridComponent" => self
                .map_grid_components
                .remove(&uid)
                .map(|component| component.base),
            "MetaDataComponent" => self.metadata.remove(&uid).map(|component| component.base),
            _ => None,
        };

        if let Some(mut base) = removed {
            self.queue_component_shutdown_and_remove(uid, &mut base);
            return true;
        }

        false
    }

    fn start_component_for_entity(&mut self, uid: EntityUid, component_name: &str) {
        let Some(event_name) = self.component_base_name(uid, component_name) else {
            return;
        };
        let should_start = self
            .component_base(uid, component_name)
            .map(|base| base.life_stage == crate::ComponentLifeStage::Initialized)
            .unwrap_or(false);
        if should_start {
            if let Some(base) = self.component_base_mut(uid, component_name) {
                base.life_stage = crate::ComponentLifeStage::Starting;
            }
            self.raise_component_event(uid, &event_name, &mut crate::ComponentStartup);
            if let Some(base) = self.component_base_mut(uid, component_name) {
                base.startup();
            }
            self.queue_component_stage(uid, component_name, crate::ComponentLifeStage::Running);
            self.process_started_component_runtime(uid, &event_name);
        }
    }

    fn component_base(&self, uid: EntityUid, component_name: &str) -> Option<&Component> {
        match component_name {
            "AppearanceComponent" | "Appearance" => self.appearances.get(&uid).map(|c| &c.base),
            "TransformComponent" => self.transforms.get(&uid).map(|c| &c.base),
            "EntityLookupComponent" => self.entity_lookups.get(&uid).map(|c| &c.base),
            "JointComponent" => self.joint_components.get(&uid).map(|c| &c.base),
            "IgnorePauseComponent" => self.ignore_pauses.get(&uid).map(|c| &c.base),
            "CollisionWakeComponent" | "CollisionWake" => {
                self.collision_wakes.get(&uid).map(|c| &c.base)
            }
            "CollideOnAnchorComponent" | "CollideOnAnchor" => {
                self.collide_on_anchors.get(&uid).map(|c| &c.base)
            }
            "TimerComponent" => self.timers.get(&uid).map(|c| &c.base),
            "Physics" => self.physics.get(&uid).map(|c| &c.base),
            "FixturesComponent" | "Fixtures" => self.fixtures.get(&uid).map(|c| &c.base),
            "BroadphaseComponent" => self.broadphases.get(&uid).map(|c| &c.base),
            "SharedPhysicsMapComponent" => self.physics_maps.get(&uid).map(|c| &c.base),
            "MapComponent" => self.map_components.get(&uid).map(|c| &c.base),
            "MapGridComponent" => self.map_grid_components.get(&uid).map(|c| &c.base),
            "MetaDataComponent" => self.metadata.get(&uid).map(|c| &c.base),
            _ => None,
        }
    }

    fn component_base_name(&self, uid: EntityUid, component_name: &str) -> Option<String> {
        self.component_base(uid, component_name)
            .map(|base| base.name.clone())
    }

    fn component_base_mut(
        &mut self,
        uid: EntityUid,
        component_name: &str,
    ) -> Option<&mut Component> {
        match component_name {
            "AppearanceComponent" | "Appearance" => {
                self.appearances.get_mut(&uid).map(|c| &mut c.base)
            }
            "TransformComponent" => self.transforms.get_mut(&uid).map(|c| &mut c.base),
            "EntityLookupComponent" => self.entity_lookups.get_mut(&uid).map(|c| &mut c.base),
            "JointComponent" => self.joint_components.get_mut(&uid).map(|c| &mut c.base),
            "IgnorePauseComponent" => self.ignore_pauses.get_mut(&uid).map(|c| &mut c.base),
            "CollisionWakeComponent" | "CollisionWake" => {
                self.collision_wakes.get_mut(&uid).map(|c| &mut c.base)
            }
            "CollideOnAnchorComponent" | "CollideOnAnchor" => {
                self.collide_on_anchors.get_mut(&uid).map(|c| &mut c.base)
            }
            "TimerComponent" => self.timers.get_mut(&uid).map(|c| &mut c.base),
            "Physics" => self.physics.get_mut(&uid).map(|c| &mut c.base),
            "FixturesComponent" | "Fixtures" => self.fixtures.get_mut(&uid).map(|c| &mut c.base),
            "BroadphaseComponent" => self.broadphases.get_mut(&uid).map(|c| &mut c.base),
            "SharedPhysicsMapComponent" => self.physics_maps.get_mut(&uid).map(|c| &mut c.base),
            "MapComponent" => self.map_components.get_mut(&uid).map(|c| &mut c.base),
            "MapGridComponent" => self.map_grid_components.get_mut(&uid).map(|c| &mut c.base),
            "MetaDataComponent" => self.metadata.get_mut(&uid).map(|c| &mut c.base),
            _ => None,
        }
    }

    pub fn refresh_map_physics_runtime(&mut self, map_id: MapId) {
        if map_id == MapId::NULLSPACE {
            return;
        }

        let Some(owner) = self.map_entity_for(map_id) else {
            return;
        };

        let bodies = self.physics_bodies_in_map(map_id, true);

        let broadphase_created = !self.broadphases.contains_key(&owner);
        let physics_map_created = !self.physics_maps.contains_key(&owner);
        let sync_broadphase = self.broadphases.contains_key(&owner) || broadphase_created;
        let sync_physics_map = self.physics_maps.contains_key(&owner) || physics_map_created;
        if !sync_broadphase && !sync_physics_map {
            return;
        }

        let previous_entries = self
            .broadphases
            .get(&owner)
            .map(|component| component.tree.snapshot())
            .unwrap_or_default();
        let previous_bodies = self
            .physics_maps
            .get(&owner)
            .map(|component| component.bodies.clone())
            .unwrap_or_default();
        let previous_awake = self
            .physics_maps
            .get(&owner)
            .map(|component| component.awake_bodies.clone())
            .unwrap_or_default();

        if sync_broadphase {
            if broadphase_created {
                self.ensure_broadphase(owner);
            }
            let _ = self.refresh_broadphase_runtime(owner, &bodies);
            let changed = self
                .broadphases
                .get(&owner)
                .map(|component| component.tree.snapshot())
                .unwrap_or_default()
                != previous_entries;
            if changed {
                if let Some(component) = self.broadphases.get_mut(&owner) {
                    component.base.last_modified_tick = self.current_tick;
                }
                if self.entity_exists(owner) {
                    self.dirty_entity(owner);
                }
            }
        }

        if sync_physics_map {
            if physics_map_created {
                self.ensure_physics_map(owner);
            }
            let body_set = bodies.iter().copied().collect::<HashSet<_>>();
            let awake_set = bodies
                .iter()
                .copied()
                .filter(|uid| self.physics.get(uid).is_some_and(|body| body.awake))
                .collect::<HashSet<_>>();
            if let Some(physics_map) = self.physics_maps.get_mut(&owner) {
                physics_map.bodies = body_set;
                physics_map.awake_bodies = awake_set;
            }
            let _ = self.refresh_contact_runtime(owner, &bodies);
            let changed = self.physics_maps.get(&owner).is_some_and(|component| {
                component.bodies != previous_bodies || component.awake_bodies != previous_awake
            });
            if changed {
                if let Some(component) = self.physics_maps.get_mut(&owner) {
                    component.base.last_modified_tick = self.current_tick;
                }
                if self.entity_exists(owner) {
                    self.dirty_entity(owner);
                }
            }
        }
    }

    fn alloc_entity(&mut self, uid: EntityUid, prototype_name: Option<&str>) {
        self.entities.insert(uid);
        let mut meta = MetaDataComponent::new();
        Self::initialize_new_component_base(&mut meta.base, uid, self.current_tick, false);
        meta.prototype_id = prototype_name.map(str::to_string);
        self.metadata.insert(uid, meta);
        self.queue_new_component_lifecycle(uid, "MetaDataComponent", false);

        let mut xform = TransformComponent::new();
        Self::initialize_new_component_base(&mut xform.base, uid, self.current_tick, false);
        xform.map_id = MapId::NULLSPACE;
        self.transforms.insert(uid, xform);
        self.queue_new_component_lifecycle(uid, "TransformComponent", false);
        self.components.insert(uid, Vec::new());
    }

    pub fn alloc_entity_external(&mut self, uid: EntityUid, prototype_name: Option<&str>) {
        self.alloc_entity(uid, prototype_name);
    }

    pub fn alloc_entity_external_with_transform(
        &mut self,
        uid: EntityUid,
        prototype_name: Option<&str>,
        state: TransformComponentState,
    ) {
        self.alloc_entity(uid, prototype_name);
        let _ = self.apply_transform_state(uid, state);
    }

    fn generate_entity_uid(&mut self) -> EntityUid {
        let uid = EntityUid::new(self.next_entity_uid);
        self.next_entity_uid += 1;
        uid
    }
}

impl crate::EntityCoordinateResolver for EntityManager {
    fn entity_exists(&self, entity: EntityUid) -> bool {
        self.entity_exists(entity)
    }

    fn transform_state(&self, entity: EntityUid) -> Option<crate::TransformState> {
        let world = self.world_transform(entity)?;
        Some(crate::TransformState {
            map_id: world.map_id,
            grid_id: world.grid_id,
            world_position: world.world_position,
        })
    }
}

impl TransformResolver for EntityManager {
    fn world_transform(&self, entity: EntityUid) -> Option<WorldTransform> {
        let xform = self.transforms.get(&entity)?;

        if xform.parent.is_valid() {
            let parent = self.world_transform(xform.parent)?;
            let world_matrix = xform.local_matrix * parent.world_matrix;
            let world_position = Vector2::new(world_matrix.r0c2, world_matrix.r1c2);
            let world_rotation = xform.local_rotation + parent.world_rotation;
            return Some(WorldTransform {
                map_id: xform.map_id,
                grid_id: xform.grid_id,
                world_position,
                world_rotation,
                world_matrix,
                inv_world_matrix: world_matrix.inverted(),
            });
        }

        Some(WorldTransform {
            map_id: xform.map_id,
            grid_id: xform.grid_id,
            world_position: xform.local_position,
            world_rotation: xform.local_rotation,
            world_matrix: xform.local_matrix,
            inv_world_matrix: xform.inv_local_matrix,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::EntityManager;
    use crate::transform_component::TransformResolver;
    use crate::{
        AnchorStateChangedEvent, ChunkDatum, ComponentLifeStage, ComponentLifecycleEvent,
        EntParentChangedMessage, EntityDeletedMessage, EntityInitializedMessage, EntityPausedEvent,
        EntityRuntimeEvent, EntitySystem, EntitySystemInfo, EntitySystemSubscriptions,
        EntityTerminatingEvent, GameState, GameStateMapData, GridDatum, MapComponentState,
        MapCoordinates, MapGridComponentState, MetaDataComponentState, PhysicsInitializedEvent,
        PhysicsRuntimeEvent, SerializedComponentChange, SerializedEntityState, Tile,
        TileRenderFlag, TransformComponentState,
    };
    use crate::{EntityUid, GridId, MapId};
    use butsuri::{AabbShape, Fixture, PhysShape};
    use keisan::{Angle, ApproxEq, Box2, Vector2, Vector2i};
    use std::sync::{Arc, Mutex};

    #[test]
    fn entity_manager_allocates_initializes_and_deletes() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized_at_map(
            Some("test"),
            MapCoordinates::new(Vector2::new(1.0, 2.0), MapId::new(7)),
        );
        assert!(manager.entity_exists(uid));
        let transform = manager.transforms.get(&uid).unwrap();
        assert_eq!(transform.local_position, Vector2::new(1.0, 2.0));
        assert_eq!(transform.map_id, MapId::new(7));
        assert_eq!(manager.metadata.get(&uid).unwrap().map_id, MapId::new(7));
        let init = manager.initialize_entity(uid);
        assert_eq!(init.entity, uid);
        manager.queue_delete_entity(uid);
        manager.flush_queued_deletions();
        assert!(!manager.entity_exists(uid));
    }

    #[test]
    fn entity_manager_set_local_transform_respects_no_local_rotation() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            uid,
            TransformComponentState {
                local_position: Vector2::new(1.0, 1.0),
                rotation: Angle::from_degrees(30.0),
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(9),
                grid_id: GridId::INVALID,
                no_local_rotation: true,
                anchored: false,
            },
        );

        let event = manager
            .set_local_transform(uid, Vector2::new(3.0, 4.0), Angle::from_degrees(90.0))
            .unwrap();
        let transform = manager.transforms.get(&uid).unwrap();
        assert_eq!(transform.local_position, Vector2::new(3.0, 4.0));
        assert_eq!(transform.local_rotation, Angle::from_degrees(30.0));
        assert_eq!(event.sender, uid);
        assert!(
            manager
                .set_local_transform(uid, Vector2::new(3.0, 4.0), Angle::from_degrees(180.0))
                .is_none()
        );
    }

    #[test]
    fn entity_manager_ensures_typed_components() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.ensure_metadata(uid);
        manager.ensure_transform(uid);
        manager.ensure_appearance(uid);
        manager.ensure_lookup(uid);
        manager.ensure_joints(uid);
        manager.ensure_map(MapId::new(3), uid);
        manager.ensure_map_grid(uid);
        manager.ensure_timer(uid);
        manager.ensure_physics(uid);
        manager.ensure_fixtures(uid);
        manager.ensure_broadphase(uid);
        manager.ensure_physics_map(uid);
        assert!(manager.metadata.contains_key(&uid));
        assert!(manager.transforms.contains_key(&uid));
        assert!(manager.appearances.contains_key(&uid));
        assert!(manager.entity_lookups.contains_key(&uid));
        assert!(manager.joint_components.contains_key(&uid));
        assert!(manager.map_components.contains_key(&uid));
        assert!(manager.map_grid_components.contains_key(&uid));
        assert!(manager.physics.contains_key(&uid));
        assert!(manager.physics_maps.contains_key(&uid));
    }

    #[test]
    fn entity_manager_apply_map_grid_component_state_materializes_runtime_grid() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(9),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        manager.ensure_map(MapId::new(9), map);

        let grid = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::new(3.0, 4.0),
                rotation: Angle::from_degrees(15.0),
                parent_id: map,
                map_id: MapId::new(9),
                grid_id: GridId::new(12),
                no_local_rotation: false,
                anchored: false,
            },
        );

        assert!(manager.apply_map_grid_component_state(
            grid,
            MapGridComponentState {
                grid_index: GridId::new(12),
                chunk_size: 8,
            },
        ));

        let runtime = manager.map_grids.get(&grid).unwrap();
        assert_eq!(runtime.parent_map_id, MapId::new(9));
        assert_eq!(runtime.index, GridId::new(12));
        assert_eq!(runtime.world_position, Vector2::new(3.0, 4.0));
        assert_eq!(runtime.world_rotation, Angle::from_degrees(15.0));
        assert_eq!(runtime.chunk_size, 8);
    }

    #[test]
    fn entity_manager_create_entity_uninitialized_as_map_and_grid_materializes_spatial_runtime() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_as_map(None, MapId::new(14));
        let grid = manager.create_entity_uninitialized_as_grid(
            None,
            MapId::new(14),
            map,
            GridId::new(41),
            8,
            Vector2::new(2.0, -1.0),
            Angle::from_degrees(30.0),
        );

        let map_transform = manager.transforms.get(&map).unwrap();
        assert_eq!(map_transform.map_id, MapId::new(14));
        assert_eq!(
            manager.map_components.get(&map).unwrap().world_map,
            MapId::new(14)
        );

        let grid_transform = manager.transforms.get(&grid).unwrap();
        assert_eq!(grid_transform.parent, map);
        assert_eq!(grid_transform.map_id, MapId::new(14));
        assert_eq!(grid_transform.grid_id, GridId::new(41));

        let grid_runtime = manager.map_grids.get(&grid).unwrap();
        assert_eq!(grid_runtime.parent_map_id, MapId::new(14));
        assert_eq!(grid_runtime.index, GridId::new(41));
        assert_eq!(grid_runtime.chunk_size, 8);
        assert_eq!(
            grid_runtime.world_position,
            manager.world_transform(grid).unwrap().world_position
        );
    }

    #[test]
    fn entity_manager_apply_transform_state_updates_existing_map_grid_runtime() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::new(5.0, 1.0),
                rotation: Angle::from_degrees(10.0),
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(4),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        manager.ensure_map(MapId::new(4), map);

        let grid = manager.create_entity_uninitialized(None);
        assert!(manager.apply_map_grid_component_state(
            grid,
            MapGridComponentState {
                grid_index: GridId::new(20),
                chunk_size: 16,
            },
        ));
        assert!(manager.apply_transform_state(
            grid,
            TransformComponentState {
                local_position: Vector2::new(2.0, 3.0),
                rotation: Angle::from_degrees(25.0),
                parent_id: map,
                map_id: MapId::new(4),
                grid_id: GridId::new(20),
                no_local_rotation: false,
                anchored: false,
            },
        ));

        let runtime = manager.map_grids.get(&grid).unwrap();
        let world = manager.world_transform(grid).unwrap();
        assert_eq!(runtime.parent_map_id, MapId::new(4));
        assert_eq!(runtime.world_position, world.world_position);
        assert_eq!(runtime.world_rotation, world.world_rotation);
    }

    #[test]
    fn entity_manager_set_parent_updates_map_grid_runtime_map_context() {
        let mut manager = EntityManager::new();
        let first_map = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(1),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        manager.ensure_map(MapId::new(1), first_map);

        let second_map = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::new(10.0, 0.0),
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(2),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        manager.ensure_map(MapId::new(2), second_map);

        let grid = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::new(1.0, 0.0),
                rotation: Angle::ZERO,
                parent_id: first_map,
                map_id: MapId::new(1),
                grid_id: GridId::new(30),
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(manager.apply_map_grid_component_state(
            grid,
            MapGridComponentState {
                grid_index: GridId::new(30),
                chunk_size: 8,
            },
        ));

        assert!(manager.set_parent(grid, second_map));
        assert!(manager.set_spatial_metadata(grid, MapId::new(2), GridId::new(30)));

        let runtime = manager.map_grids.get(&grid).unwrap();
        assert_eq!(runtime.parent_map_id, MapId::new(2));
        assert_eq!(runtime.world_position, Vector2::new(11.0, 0.0));
    }

    #[test]
    fn entity_manager_materialize_grid_entity_updates_existing_entity_in_one_step() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_as_map(None, MapId::new(22));
        let uid = manager.create_entity_uninitialized(None);

        assert!(manager.materialize_grid_entity(
            uid,
            MapId::new(22),
            map,
            GridId::new(77),
            4,
            Vector2::new(9.0, 3.0),
            Angle::from_degrees(45.0),
        ));

        let transform = manager.transforms.get(&uid).unwrap();
        assert_eq!(transform.parent, map);
        assert_eq!(transform.map_id, MapId::new(22));
        assert_eq!(transform.grid_id, GridId::new(77));
        let grid = manager.map_grids.get(&uid).unwrap();
        assert_eq!(grid.index, GridId::new(77));
        assert_eq!(grid.chunk_size, 4);
        assert_eq!(
            grid.world_rotation,
            manager.world_transform(uid).unwrap().world_rotation
        );
    }

    #[test]
    fn entity_manager_rebuild_runtime_state_reconstructs_spatial_runtime_from_components() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized(None);
        manager.ensure_transform(map);
        assert!(manager.apply_map_component_state(
            map,
            MapComponentState {
                map_id: MapId::new(31),
                lighting_enabled: true,
                map_paused: false,
            },
        ));

        let grid = manager.create_entity_uninitialized(None);
        manager.ensure_transform(grid);
        assert!(manager.apply_map_grid_component_state(
            grid,
            MapGridComponentState {
                grid_index: GridId::new(90),
                chunk_size: 8,
            },
        ));
        assert!(manager.apply_transform_state(
            grid,
            TransformComponentState {
                local_position: Vector2::new(4.0, 1.0),
                rotation: Angle::from_degrees(5.0),
                parent_id: map,
                map_id: MapId::NULLSPACE,
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        ));

        let entity = manager.create_entity_uninitialized(None);
        assert!(manager.apply_transform_state(
            entity,
            TransformComponentState {
                local_position: Vector2::new(0.5, 0.5),
                rotation: Angle::ZERO,
                parent_id: grid,
                map_id: MapId::NULLSPACE,
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        ));

        manager.entity_lookups.clear();
        manager.map_grids.clear();
        manager.broadphases.clear();
        manager.physics_maps.clear();
        manager.rebuild_runtime_state();

        let grid_transform = manager.transforms.get(&grid).unwrap();
        assert_eq!(grid_transform.map_id, MapId::new(31));
        assert_eq!(grid_transform.grid_id, GridId::new(90));

        let entity_transform = manager.transforms.get(&entity).unwrap();
        assert_eq!(entity_transform.map_id, MapId::new(31));
        assert_eq!(entity_transform.grid_id, GridId::new(90));

        let grid_runtime = manager.map_grids.get(&grid).unwrap();
        assert_eq!(grid_runtime.parent_map_id, MapId::new(31));
        assert_eq!(grid_runtime.index, GridId::new(90));
        assert_eq!(
            grid_runtime.world_position,
            manager.world_transform(grid).unwrap().world_position
        );

        assert!(manager.entity_lookups.contains_key(&grid));
    }

    #[test]
    fn entity_manager_apply_map_and_grid_state_create_transform_shells() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized(None);
        assert!(manager.apply_map_component_state(
            map,
            MapComponentState {
                map_id: MapId::new(41),
                lighting_enabled: true,
                map_paused: false,
            },
        ));
        assert!(manager.transforms.contains_key(&map));
        assert_eq!(
            manager.transforms.get(&map).unwrap().map_id,
            MapId::NULLSPACE
        );

        let grid = manager.create_entity_uninitialized(None);
        assert!(manager.apply_map_grid_component_state(
            grid,
            MapGridComponentState {
                grid_index: GridId::new(91),
                chunk_size: 8,
            },
        ));
        assert!(manager.transforms.contains_key(&grid));
        assert_eq!(
            manager.transforms.get(&grid).unwrap().grid_id,
            GridId::INVALID
        );
        assert!(manager.map_grids.contains_key(&grid));
    }

    #[test]
    fn entity_manager_finds_map_and_grid_entities_by_runtime_ids() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_as_map(None, MapId::new(51));
        let grid = manager.create_entity_uninitialized_as_grid(
            None,
            MapId::new(51),
            map,
            GridId::new(101),
            8,
            Vector2::ZERO,
            Angle::ZERO,
        );

        assert_eq!(manager.map_entity_for(MapId::new(51)), Some(map));
        assert_eq!(manager.grid_entity_for(GridId::new(101)), Some(grid));
        assert!(manager.grid_exists(GridId::new(101)));
        assert!(!manager.grid_exists(GridId::new(999)));
    }

    #[test]
    fn entity_manager_reports_spatial_context_for_map_grid_and_child_entities() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_as_map(None, MapId::new(61));
        let grid = manager.create_entity_uninitialized_as_grid(
            None,
            MapId::new(61),
            map,
            GridId::new(121),
            8,
            Vector2::new(1.0, 0.0),
            Angle::ZERO,
        );
        let child = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::new(0.5, 0.5),
                rotation: Angle::ZERO,
                parent_id: grid,
                map_id: MapId::new(61),
                grid_id: GridId::new(121),
                no_local_rotation: false,
                anchored: false,
            },
        );

        assert_eq!(manager.map_id_for(map), MapId::new(61));
        assert_eq!(manager.grid_id_for(map), GridId::INVALID);
        assert!(manager.is_map_entity(map));
        assert!(!manager.is_grid_entity(map));

        assert_eq!(manager.map_id_for(grid), MapId::new(61));
        assert_eq!(manager.grid_id_for(grid), GridId::new(121));
        assert!(!manager.is_map_entity(grid));
        assert!(manager.is_grid_entity(grid));

        assert_eq!(manager.map_id_for(child), MapId::new(61));
        assert_eq!(manager.grid_id_for(child), GridId::new(121));
        assert!(!manager.is_map_entity(child));
        assert!(!manager.is_grid_entity(child));
    }

    #[test]
    fn entity_manager_removes_components_by_net_id_with_shared_lifecycle() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_as_map(None, MapId::new(71));
        let grid = manager.create_entity_uninitialized_as_grid(
            None,
            MapId::new(71),
            map,
            GridId::new(131),
            8,
            Vector2::ZERO,
            Angle::ZERO,
        );
        manager.ensure_appearance(grid);
        manager.ensure_physics(grid);

        assert!(manager.component_exists_for_net_id(grid, EntityManager::MAP_GRID_NET_ID));
        assert!(manager.component_exists_for_net_id(grid, EntityManager::APPEARANCE_NET_ID));
        assert!(manager.component_exists_for_net_id(grid, EntityManager::PHYSICS_NET_ID));

        assert!(manager.remove_component_by_net_id(grid, EntityManager::APPEARANCE_NET_ID));
        assert!(!manager.component_exists_for_net_id(grid, EntityManager::APPEARANCE_NET_ID));

        assert!(manager.remove_component_by_net_id(grid, EntityManager::PHYSICS_NET_ID));
        assert!(!manager.component_exists_for_net_id(grid, EntityManager::PHYSICS_NET_ID));

        assert!(manager.remove_component_by_net_id(grid, EntityManager::MAP_GRID_NET_ID));
        assert!(!manager.component_exists_for_net_id(grid, EntityManager::MAP_GRID_NET_ID));
        assert!(!manager.map_grids.contains_key(&grid));
    }

    #[test]
    fn entity_manager_remove_map_grid_component_clears_runtime_support() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_as_map(None, MapId::new(172));
        let grid = manager.create_entity_uninitialized_as_grid(
            None,
            MapId::new(172),
            map,
            GridId::new(232),
            8,
            Vector2::ZERO,
            Angle::ZERO,
        );
        let child = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::new(0.5, 0.5),
                rotation: Angle::ZERO,
                parent_id: grid,
                map_id: MapId::new(172),
                grid_id: GridId::new(232),
                no_local_rotation: false,
                anchored: false,
            },
        );
        manager.refresh_entity_lookup(child);

        assert_eq!(
            manager.entities_at_tile(GridId::new(232), Vector2i::new(0, 0)),
            vec![child]
        );
        assert!(manager.remove_map_grid_component(grid));
        assert!(!manager.map_grids.contains_key(&grid));
        assert_eq!(
            manager.transforms.get(&grid).unwrap().grid_id,
            GridId::INVALID
        );
        assert!(
            manager
                .entities_at_tile(GridId::new(232), Vector2i::new(0, 0))
                .is_empty()
        );
    }

    #[test]
    fn entity_manager_applies_serialized_component_by_net_id_for_shared_runtime_components() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        let mut serializer = crate::RobustSerializer::new();
        let state = serializer
            .serialize_component_state(&crate::PhysicsComponentState {
                can_collide: true,
                awake: false,
                sleeping_allowed: true,
                fixed_rotation: false,
                status: crate::BodyStatus::InAir,
                linear_velocity: Vector2::new(3.0, 0.0),
                angular_velocity: 2.0,
                body_type: crate::BodyType::Dynamic,
            })
            .unwrap();

        assert!(manager.apply_serialized_component_by_net_id(
            &mut serializer,
            uid,
            EntityManager::PHYSICS_NET_ID,
            &state,
        ));

        let physics = manager.physics.get(&uid).unwrap();
        assert!(physics.can_collide);
        assert!(!physics.awake);
        assert_eq!(physics.linear_velocity, Vector2::ZERO);
        assert_eq!(physics.body_status, crate::BodyStatus::InAir);
        assert_eq!(physics.body_type, crate::BodyType::Dynamic);
    }

    #[test]
    fn entity_manager_creates_entity_shell_from_serialized_state() {
        let mut manager = EntityManager::new();
        let mut serializer = crate::RobustSerializer::new();
        let uid = EntityUid::new(77);
        let state = SerializedEntityState {
            uid,
            component_changes: vec![
                SerializedComponentChange::new(
                    EntityManager::METADATA_NET_ID,
                    true,
                    false,
                    Some(
                        serializer
                            .serialize_component_state(&MetaDataComponentState {
                                name: Some("Grid".to_string()),
                                description: None,
                                prototype_id: Some("grid".to_string()),
                            })
                            .unwrap(),
                    ),
                ),
                SerializedComponentChange::new(
                    EntityManager::TRANSFORM_NET_ID,
                    true,
                    false,
                    Some(
                        serializer
                            .serialize_component_state(&TransformComponentState {
                                local_position: Vector2::new(3.0, 4.0),
                                rotation: Angle::ZERO,
                                parent_id: EntityUid::INVALID,
                                map_id: MapId::new(12),
                                grid_id: GridId::INVALID,
                                no_local_rotation: false,
                                anchored: false,
                            })
                            .unwrap(),
                    ),
                ),
            ],
        };

        assert_eq!(
            manager.create_entity_from_serialized_state(&mut serializer, &state),
            Some(uid)
        );
        assert!(manager.entity_exists(uid));
        assert_eq!(
            manager.metadata.get(&uid).unwrap().prototype_id.as_deref(),
            Some("grid")
        );
        let transform = manager.transforms.get(&uid).unwrap();
        assert_eq!(transform.local_position, Vector2::new(3.0, 4.0));
        assert_eq!(transform.map_id, MapId::new(12));
    }

    #[test]
    fn entity_manager_applies_serialized_entity_state_with_shared_component_loop() {
        let mut manager = EntityManager::new();
        let mut serializer = crate::RobustSerializer::new();
        let uid = EntityUid::new(78);
        let state = SerializedEntityState {
            uid,
            component_changes: vec![
                SerializedComponentChange::new(
                    EntityManager::METADATA_NET_ID,
                    true,
                    false,
                    Some(
                        serializer
                            .serialize_component_state(&MetaDataComponentState {
                                name: Some("Mob".to_string()),
                                description: None,
                                prototype_id: Some("mob".to_string()),
                            })
                            .unwrap(),
                    ),
                ),
                SerializedComponentChange::new(
                    EntityManager::TRANSFORM_NET_ID,
                    true,
                    false,
                    Some(
                        serializer
                            .serialize_component_state(&TransformComponentState {
                                local_position: Vector2::new(5.0, -1.0),
                                rotation: Angle::from_degrees(25.0),
                                parent_id: EntityUid::INVALID,
                                map_id: MapId::new(18),
                                grid_id: GridId::INVALID,
                                no_local_rotation: false,
                                anchored: false,
                            })
                            .unwrap(),
                    ),
                ),
                SerializedComponentChange::new(
                    EntityManager::APPEARANCE_NET_ID,
                    true,
                    false,
                    Some(
                        serializer
                            .serialize_component_state(&crate::AppearanceComponentState {
                                data: std::collections::HashMap::from([(
                                    String::from("mode"),
                                    crate::AppearanceValue::UInt(2),
                                )]),
                            })
                            .unwrap(),
                    ),
                ),
            ],
        };

        manager.apply_serialized_entity_state(&mut serializer, &state);

        assert!(manager.entity_exists(uid));
        assert_eq!(
            manager.metadata.get(&uid).unwrap().prototype_id.as_deref(),
            Some("mob")
        );
        let transform = manager.transforms.get(&uid).unwrap();
        assert_eq!(transform.local_position, Vector2::new(5.0, -1.0));
        assert_eq!(transform.local_rotation, Angle::from_degrees(25.0));
        assert_eq!(transform.map_id, MapId::new(18));
        assert_eq!(
            manager
                .appearances
                .get(&uid)
                .unwrap()
                .get_data::<u32>("mode"),
            Some(2)
        );
        assert_eq!(
            manager.metadata.get(&uid).unwrap().entity_life_stage,
            crate::EntityLifeStage::Initialized
        );
        assert!(manager.drain_entity_runtime_events().contains(
            &EntityRuntimeEvent::EntityInitialized(EntityInitializedMessage { entity: uid })
        ));
    }

    #[test]
    fn entity_manager_apply_serialized_entity_states_with_leaves_created_entities_for_caller_init()
    {
        let mut manager = EntityManager::new();
        let mut serializer = crate::RobustSerializer::new();
        let uid = EntityUid::new(79);
        let states = vec![SerializedEntityState {
            uid,
            component_changes: vec![
                SerializedComponentChange::new(
                    EntityManager::METADATA_NET_ID,
                    true,
                    false,
                    Some(
                        serializer
                            .serialize_component_state(&MetaDataComponentState {
                                name: Some("Mob".to_string()),
                                description: None,
                                prototype_id: Some("mob".to_string()),
                            })
                            .unwrap(),
                    ),
                ),
                SerializedComponentChange::new(
                    EntityManager::TRANSFORM_NET_ID,
                    true,
                    false,
                    Some(
                        serializer
                            .serialize_component_state(&TransformComponentState {
                                local_position: Vector2::new(1.0, 2.0),
                                rotation: Angle::ZERO,
                                parent_id: EntityUid::INVALID,
                                map_id: MapId::new(19),
                                grid_id: GridId::INVALID,
                                no_local_rotation: false,
                                anchored: false,
                            })
                            .unwrap(),
                    ),
                ),
            ],
        }];

        let created = manager.apply_serialized_entity_states_with(
            &mut serializer,
            &states,
            |entity_manager, uid, transform_state| {
                let _ = entity_manager.apply_transform_component_state(uid, transform_state);
            },
        );

        assert_eq!(created, vec![uid]);
        assert!(manager.entity_exists(uid));
        assert_eq!(
            manager.metadata.get(&uid).unwrap().entity_life_stage,
            crate::EntityLifeStage::PreInit
        );
        assert!(!manager.drain_entity_runtime_events().iter().any(
            |event| matches!(event, EntityRuntimeEvent::EntityInitialized(ev) if ev.entity == uid)
        ));
    }

    #[test]
    fn entity_manager_builds_shared_serialized_entity_state_with_deleted_components() {
        let mut manager = EntityManager::new();
        manager.current_tick = jikan::GameTick::new(8);
        let uid = manager.create_entity_uninitialized(None);
        assert!(manager.set_appearance_data(uid, "mode", 5u8));
        assert!(manager.remove_component_by_net_id(uid, EntityManager::APPEARANCE_NET_ID));

        let mut serializer = crate::RobustSerializer::new();
        let state = manager
            .build_serialized_entity_state_since(
                &mut serializer,
                uid,
                jikan::GameTick::new(7),
                &[EntityManager::APPEARANCE_NET_ID],
            )
            .unwrap();

        assert!(
            state.component_changes.iter().any(|change| {
                change.net_id == EntityManager::APPEARANCE_NET_ID && change.deleted
            })
        );
    }

    #[test]
    fn entity_manager_builds_serialized_entity_states_for_sync_with_deleted_callback() {
        let mut manager = EntityManager::new();
        manager.current_tick = jikan::GameTick::new(4);

        let newly_visible = manager.create_entity_uninitialized(Some("new"));
        manager.initialize_entity(newly_visible);

        let dirty = manager.create_entity_uninitialized(Some("dirty"));
        manager.initialize_entity(dirty);

        let clean = manager.create_entity_uninitialized(Some("clean"));
        manager.initialize_entity(clean);

        manager.current_tick = jikan::GameTick::new(5);
        assert!(manager.set_appearance_data(newly_visible, "mode", 1u8));
        assert!(manager.set_appearance_data(dirty, "mode", 2u8));
        manager.dirty_entity(dirty);

        let mut serializer = crate::RobustSerializer::new();
        let states = manager.build_serialized_entity_states_for_sync(
            &mut serializer,
            &[newly_visible, dirty, clean],
            &[newly_visible],
            jikan::GameTick::new(4),
            |_uid, _from_tick| Vec::new(),
        );

        assert_eq!(states.len(), 2);
        assert!(states.iter().any(|state| state.uid == newly_visible));
        assert!(states.iter().any(|state| state.uid == dirty));
        assert!(!states.iter().any(|state| state.uid == clean));
    }

    #[test]
    fn entity_manager_builds_serialized_entity_state_with_deleted_callback() {
        let mut manager = EntityManager::new();
        manager.current_tick = jikan::GameTick::new(8);
        let uid = manager.create_entity_uninitialized(None);
        assert!(manager.set_appearance_data(uid, "mode", 5u8));
        assert!(manager.remove_component_by_net_id(uid, EntityManager::APPEARANCE_NET_ID));

        let mut serializer = crate::RobustSerializer::new();
        let state = manager
            .build_serialized_entity_state_with_deleted_callback(
                &mut serializer,
                uid,
                jikan::GameTick::new(7),
                |_uid, _from_tick| vec![EntityManager::APPEARANCE_NET_ID],
            )
            .unwrap();

        assert!(
            state
                .component_changes
                .iter()
                .any(|change| change.net_id == EntityManager::APPEARANCE_NET_ID && change.deleted)
        );
    }

    #[test]
    fn entity_manager_canonical_appearance_helpers_track_state_and_dirtying() {
        let mut manager = EntityManager::new();
        manager.current_tick = jikan::GameTick::new(4);
        let uid = manager.create_entity_uninitialized(None);

        assert!(manager.set_appearance_data(uid, "mode", 1u8));
        assert_eq!(manager.get_appearance_state(uid).unwrap().data.len(), 1);
        assert_eq!(
            manager
                .appearances
                .get(&uid)
                .unwrap()
                .get_data::<u8>("mode"),
            Some(1)
        );
        assert_eq!(manager.drain_appearance_dirty(), vec![uid]);
        assert!(manager.clear_appearance_dirty(uid));
        assert!(!manager.appearances.get(&uid).unwrap().appearance_dirty);

        assert!(!manager.set_appearance_data(uid, "mode", 1u8));
        assert!(manager.drain_appearance_dirty().is_empty());

        assert!(manager.set_appearance_data(uid, "mode", 2u8));
        assert!(manager.clear_appearance_dirty(uid));
        assert!(manager.drain_appearance_dirty().is_empty());
    }

    #[test]
    fn entity_manager_tracks_appearance_dirty_from_runtime_events_without_builtin_system() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.ensure_appearance(uid);
        assert!(manager.clear_appearance_dirty(uid));

        assert!(manager.set_entity_paused(uid, true));
        assert_eq!(manager.drain_appearance_dirty(), vec![uid]);

        assert!(manager.clear_appearance_dirty(uid));
        manager.queue_delete_entity(uid);
        manager.flush_queued_deletions();
        assert!(manager.drain_appearance_dirty().is_empty());
    }

    #[test]
    fn entity_manager_apply_game_state_map_data_materializes_and_updates_grids() {
        let mut manager = EntityManager::new();
        manager.apply_game_state_map_data(&GameStateMapData {
            grid_data: std::iter::once((
                GridId::new(31),
                GridDatum {
                    coordinates: MapCoordinates::new(Vector2::new(2.0, 3.0), MapId::new(9)),
                    angle: Angle::ZERO,
                    chunk_data: vec![ChunkDatum::create_modified(
                        Vector2i::new(0, 0),
                        vec![Tile::new(4, TileRenderFlag(0), 0)],
                    )],
                },
            ))
            .collect(),
            deleted_grids: Vec::new(),
        });

        let map_uid = manager.map_entity_for(MapId::new(9)).unwrap();
        let grid_uid = manager.grid_entity_for(GridId::new(31)).unwrap();
        assert_eq!(manager.transforms.get(&grid_uid).unwrap().parent, map_uid);
        assert_eq!(
            manager.metadata.get(&map_uid).unwrap().entity_life_stage,
            crate::EntityLifeStage::MapInitialized
        );
        assert_eq!(
            manager.metadata.get(&grid_uid).unwrap().entity_life_stage,
            crate::EntityLifeStage::MapInitialized
        );
        let events = manager.drain_entity_runtime_events();
        assert!(
            events.contains(&EntityRuntimeEvent::MapInit(crate::MapInitEvent {
                entity: map_uid,
            }))
        );
        assert!(
            events.contains(&EntityRuntimeEvent::MapInit(crate::MapInitEvent {
                entity: grid_uid,
            }))
        );
        assert_eq!(
            manager.map_grids.get(&grid_uid).unwrap().parent_map_id,
            MapId::new(9)
        );
        assert_eq!(
            manager
                .map_grids
                .get(&grid_uid)
                .unwrap()
                .get_tile_ref(Vector2i::new(0, 0))
                .tile
                .type_id,
            4
        );

        manager.apply_game_state_map_data(&GameStateMapData {
            grid_data: std::iter::once((
                GridId::new(31),
                GridDatum {
                    coordinates: MapCoordinates::new(Vector2::new(2.0, 3.0), MapId::new(9)),
                    angle: Angle::ZERO,
                    chunk_data: vec![ChunkDatum::create_deleted(Vector2i::new(0, 0))],
                },
            ))
            .collect(),
            deleted_grids: Vec::new(),
        });

        assert!(
            manager
                .map_grids
                .get(&grid_uid)
                .unwrap()
                .try_get_chunk(Vector2i::new(0, 0))
                .is_none()
        );
    }

    #[test]
    fn entity_manager_apply_game_state_map_data_deletes_grids_by_grid_id() {
        let mut manager = EntityManager::new();
        let grid_uid = manager.ensure_grid_entity_shell(GridId::new(44));
        assert!(manager.map_grids.contains_key(&grid_uid));

        manager.apply_game_state_map_data(&GameStateMapData {
            grid_data: Default::default(),
            deleted_grids: vec![GridId::new(44)],
        });

        assert!(manager.grid_entity_for(GridId::new(44)).is_none());
        assert!(!manager.entity_exists(grid_uid));
    }

    #[test]
    fn entity_manager_apply_game_state_delta_handles_create_map_delete_and_rebuild() {
        let mut manager = EntityManager::new();
        let deleted_uid = manager.create_entity_uninitialized(None);
        manager.initialize_entity(deleted_uid);
        let mut serializer = crate::RobustSerializer::new();
        let state = GameState {
            from_sequence: jikan::GameTick::ZERO,
            to_sequence: jikan::GameTick::new(2),
            last_processed_input: 0,
            entity_states: vec![SerializedEntityState {
                uid: EntityUid::new(91),
                component_changes: vec![
                    SerializedComponentChange::new(
                        EntityManager::METADATA_NET_ID,
                        true,
                        false,
                        Some(
                            serializer
                                .serialize_component_state(&MetaDataComponentState {
                                    name: Some("mob".to_string()),
                                    description: None,
                                    prototype_id: Some("mob".to_string()),
                                })
                                .unwrap(),
                        ),
                    ),
                    SerializedComponentChange::new(
                        EntityManager::TRANSFORM_NET_ID,
                        true,
                        false,
                        Some(
                            serializer
                                .serialize_component_state(&TransformComponentState {
                                    local_position: Vector2::new(7.0, 1.0),
                                    rotation: Angle::ZERO,
                                    parent_id: EntityUid::INVALID,
                                    map_id: MapId::new(33),
                                    grid_id: GridId::INVALID,
                                    no_local_rotation: false,
                                    anchored: false,
                                })
                                .unwrap(),
                        ),
                    ),
                ],
            }],
            player_states: Vec::new(),
            entity_deletions: vec![deleted_uid],
            map_data: Some(GameStateMapData {
                grid_data: std::iter::once((
                    GridId::new(51),
                    GridDatum {
                        coordinates: MapCoordinates::new(Vector2::new(2.0, 0.0), MapId::new(33)),
                        angle: Angle::ZERO,
                        chunk_data: vec![ChunkDatum::create_modified(
                            Vector2i::new(0, 0),
                            vec![Tile::new(9, TileRenderFlag(0), 0)],
                        )],
                    },
                ))
                .collect(),
                deleted_grids: Vec::new(),
            }),
            extrapolated: false,
            payload_size: 0,
        };

        let created = manager.apply_game_state_delta_with(
            &mut serializer,
            &state,
            |entity_manager, uid, transform_state| {
                let _ = entity_manager.apply_transform_component_state(uid, transform_state);
            },
        );

        assert_eq!(created, vec![EntityUid::new(91)]);
        assert_eq!(
            manager
                .metadata
                .get(&EntityUid::new(91))
                .unwrap()
                .entity_life_stage,
            crate::EntityLifeStage::MapInitialized
        );
        assert!(!manager.entity_exists(deleted_uid));
        let grid_uid = manager.grid_entity_for(GridId::new(51)).unwrap();
        assert_eq!(
            manager.transforms.get(&grid_uid).unwrap().map_id,
            MapId::new(33)
        );
        assert_eq!(
            manager
                .map_grids
                .get(&grid_uid)
                .unwrap()
                .get_tile_ref(Vector2i::new(0, 0))
                .tile
                .type_id,
            9
        );
    }

    #[test]
    fn entity_manager_exposes_shared_grid_runtime_queries() {
        let mut manager = EntityManager::new();
        let map_uid = manager.create_entity_uninitialized_as_map(None, MapId::new(40));
        let grid_uid = manager.create_entity_uninitialized_as_grid(
            None,
            MapId::new(40),
            map_uid,
            GridId::new(12),
            4,
            Vector2::new(1.0, 2.0),
            Angle::ZERO,
        );
        manager
            .map_grids
            .get_mut(&grid_uid)
            .unwrap()
            .set_tile(Vector2i::new(0, 0), Tile::new(6, TileRenderFlag(0), 0));

        assert_eq!(
            manager.grid_world_position(GridId::new(12)),
            Some(Vector2::new(1.0, 2.0))
        );
        assert!(
            manager
                .grid_world_bounds(GridId::new(12))
                .unwrap()
                .contains(Vector2::new(1.0, 2.0), true)
        );
        assert_eq!(
            manager.try_find_grid_at(MapId::new(40), Vector2::new(1.5, 2.5)),
            Some(GridId::new(12))
        );
        assert_eq!(
            manager
                .tile_ref(GridId::new(12), Vector2i::new(0, 0))
                .unwrap()
                .tile
                .type_id,
            6
        );
    }

    #[test]
    fn entity_manager_delete_parent_recursively_deletes_children() {
        let mut manager = EntityManager::new();
        let parent = manager.create_entity_uninitialized(None);
        let child = manager.create_entity_uninitialized(None);
        manager.transforms.get_mut(&parent).unwrap().local_position = Vector2::new(5.0, 0.0);
        manager
            .transforms
            .get_mut(&parent)
            .unwrap()
            .rebuild_for_manager();
        manager.apply_transform_state(
            child,
            TransformComponentState {
                local_position: Vector2::new(1.0, 2.0),
                rotation: Angle::ZERO,
                parent_id: parent,
                map_id: MapId::NULLSPACE,
                grid_id: crate::GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );

        assert!(
            manager
                .transforms
                .get(&parent)
                .unwrap()
                .children
                .contains(&child)
        );
        assert_eq!(
            manager.world_transform(child).unwrap().world_position,
            Vector2::new(6.0, 2.0)
        );

        manager.queue_delete_entity(parent);
        manager.flush_queued_deletions();

        assert!(!manager.entity_exists(parent));
        assert!(!manager.entity_exists(child));
    }

    #[test]
    fn entity_manager_delete_parent_recursively_deletes_child_physics() {
        let mut manager = EntityManager::new();
        let parent = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            parent,
            TransformComponentState {
                local_position: Vector2::new(4.0, -1.0),
                rotation: Angle::from_degrees(30.0),
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(13),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let parent_body = manager.ensure_physics(parent);
        parent_body.set_body_type(crate::BodyType::Dynamic);
        parent_body.linear_velocity = Vector2::new(1.0, 2.0);
        parent_body.angular_velocity = 0.8;

        let child = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            child,
            TransformComponentState {
                local_position: Vector2::new(2.0, 0.5),
                rotation: Angle::from_degrees(-15.0),
                parent_id: parent,
                map_id: MapId::new(13),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let child_body = manager.ensure_physics(child);
        child_body.set_body_type(crate::BodyType::Dynamic);
        child_body.linear_velocity = Vector2::new(-0.5, 1.5);
        child_body.angular_velocity = 1.2;

        manager.queue_delete_entity(parent);
        manager.flush_queued_deletions();

        assert!(!manager.entity_exists(parent));
        assert!(!manager.entity_exists(child));
        assert!(!manager.physics.contains_key(&parent));
        assert!(!manager.physics.contains_key(&child));
    }

    #[test]
    fn entity_manager_set_parent_preserves_map_velocities_for_physics_bodies() {
        let mut manager = EntityManager::new();
        let first_parent = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            first_parent,
            TransformComponentState {
                local_position: Vector2::new(5.0, 0.0),
                rotation: Angle::from_degrees(90.0),
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(11),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let first_parent_body = manager.ensure_physics(first_parent);
        first_parent_body.set_body_type(crate::BodyType::Dynamic);
        first_parent_body.linear_velocity = Vector2::new(2.0, 1.0);
        first_parent_body.angular_velocity = 0.75;

        let second_parent = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            second_parent,
            TransformComponentState {
                local_position: Vector2::new(-3.0, 2.0),
                rotation: Angle::from_degrees(-30.0),
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(11),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let second_parent_body = manager.ensure_physics(second_parent);
        second_parent_body.set_body_type(crate::BodyType::Dynamic);
        second_parent_body.linear_velocity = Vector2::new(-1.0, 4.0);
        second_parent_body.angular_velocity = -0.25;

        let child = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            child,
            TransformComponentState {
                local_position: Vector2::new(1.5, -0.25),
                rotation: Angle::ZERO,
                parent_id: first_parent,
                map_id: MapId::new(11),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let child_body = manager.ensure_physics(child);
        child_body.set_body_type(crate::BodyType::Dynamic);
        child_body.linear_velocity = Vector2::new(3.0, -2.0);
        child_body.angular_velocity = 1.25;

        let old_linear = manager.map_linear_velocity(child);
        let old_angular = manager.map_angular_velocity(child);

        assert!(manager.set_parent(child, second_parent));

        let new_linear = manager.map_linear_velocity(child);
        let new_angular = manager.map_angular_velocity(child);
        assert!(new_linear.approx_eq_with_tolerance(old_linear, 0.0001));
        assert!((new_angular - old_angular).abs() <= 0.0001);
    }

    #[test]
    fn entity_manager_apply_transform_state_preserves_map_velocities_across_parent_changes() {
        let mut manager = EntityManager::new();
        let first_parent = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            first_parent,
            TransformComponentState {
                local_position: Vector2::new(2.0, 1.0),
                rotation: Angle::from_degrees(45.0),
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(12),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let first_parent_body = manager.ensure_physics(first_parent);
        first_parent_body.set_body_type(crate::BodyType::Dynamic);
        first_parent_body.linear_velocity = Vector2::new(1.0, 3.0);
        first_parent_body.angular_velocity = 0.5;

        let second_parent = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            second_parent,
            TransformComponentState {
                local_position: Vector2::new(-4.0, -2.0),
                rotation: Angle::from_degrees(10.0),
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(12),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let second_parent_body = manager.ensure_physics(second_parent);
        second_parent_body.set_body_type(crate::BodyType::Dynamic);
        second_parent_body.linear_velocity = Vector2::new(-2.5, 1.5);
        second_parent_body.angular_velocity = -1.0;

        let child = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            child,
            TransformComponentState {
                local_position: Vector2::new(0.25, 2.5),
                rotation: Angle::ZERO,
                parent_id: first_parent,
                map_id: MapId::new(12),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let child_body = manager.ensure_physics(child);
        child_body.set_body_type(crate::BodyType::Dynamic);
        child_body.linear_velocity = Vector2::new(0.5, -1.5);
        child_body.angular_velocity = 2.0;

        let old_linear = manager.map_linear_velocity(child);
        let old_angular = manager.map_angular_velocity(child);

        assert!(manager.apply_transform_state(
            child,
            TransformComponentState {
                local_position: Vector2::new(-1.0, 0.75),
                rotation: Angle::from_degrees(15.0),
                parent_id: second_parent,
                map_id: MapId::new(12),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        ));

        let new_linear = manager.map_linear_velocity(child);
        let new_angular = manager.map_angular_velocity(child);
        assert!(new_linear.approx_eq_with_tolerance(old_linear, 0.0001));
        assert!((new_angular - old_angular).abs() <= 0.0001);
    }

    #[test]
    fn entity_manager_delete_cleans_spatial_runtime_references() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized(None);
        manager.ensure_map(MapId::new(1), map);
        manager.transforms.get_mut(&map).unwrap().map_id = MapId::new(1);

        let grid_uid = manager.create_entity_uninitialized(None);
        let grid = manager
            .map_grid_components
            .entry(grid_uid)
            .or_insert_with(crate::MapGridComponent::new);
        grid.base.owner = grid_uid;
        grid.grid_index = GridId::new(3);
        let map_grid = grid.alloc_map_grid(grid_uid, MapId::new(1), 1).clone();
        manager.map_grids.insert(grid_uid, map_grid);
        manager.set_parent(grid_uid, map);
        manager
            .map_grids
            .get_mut(&grid_uid)
            .unwrap()
            .set_tile(Vector2i::new(0, 0), Tile::new(1, TileRenderFlag(0), 0));

        let moving = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            moving,
            TransformComponentState {
                local_position: Vector2::new(0.5, 0.5),
                rotation: Angle::ZERO,
                parent_id: grid_uid,
                map_id: MapId::new(1),
                grid_id: GridId::new(3),
                no_local_rotation: false,
                anchored: false,
            },
        );
        manager.refresh_entity_lookup(moving);

        let anchored = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            anchored,
            TransformComponentState {
                local_position: Vector2::new(0.5, 0.5),
                rotation: Angle::ZERO,
                parent_id: grid_uid,
                map_id: MapId::new(1),
                grid_id: GridId::new(3),
                no_local_rotation: false,
                anchored: true,
            },
        );
        manager.refresh_entity_lookup(anchored);

        assert_eq!(
            manager.entities_at_tile(GridId::new(3), Vector2i::new(0, 0)),
            vec![moving, anchored]
        );
        assert_eq!(
            manager.entities_in_grid_aabb(GridId::new(3), Box2::new(-1.0, -1.0, 1.0, 1.0), true),
            vec![moving, anchored]
        );

        manager.queue_delete_entity(moving);
        manager.flush_queued_deletions();
        assert_eq!(
            manager.entities_at_tile(GridId::new(3), Vector2i::new(0, 0)),
            vec![anchored]
        );

        manager.queue_delete_entity(anchored);
        manager.flush_queued_deletions();
        assert!(
            manager
                .entities_at_tile(GridId::new(3), Vector2i::new(0, 0))
                .is_empty()
        );
    }

    #[test]
    fn entity_manager_removes_physics_component_with_runtime_cleanup() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized(None);
        manager.ensure_map(MapId::new(1), map);
        manager.transforms.get_mut(&map).unwrap().map_id = MapId::new(1);
        manager.ensure_broadphase(map);
        manager.ensure_physics_map(map);

        let first = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            first,
            TransformComponentState {
                local_position: Vector2::new(0.0, 0.0),
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(1),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(manager.configure_physics_body(
            first,
            Some(crate::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        let _ = manager.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "first",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let second = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            second,
            TransformComponentState {
                local_position: Vector2::new(0.5, 0.0),
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(1),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(manager.configure_physics_body(
            second,
            Some(crate::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        let _ = manager.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "second",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        manager.refresh_entities_runtime(&[first, second]);
        assert_eq!(manager.map_contact_count(MapId::new(1)), 1);
        assert_eq!(
            manager.query_aabb_entities(map, Box2::new(-1.0, -1.0, 1.0, 1.0)),
            vec![first, second]
        );

        assert!(manager.remove_physics_component(first));
        assert!(!manager.physics.contains_key(&first));
        assert_eq!(manager.map_contact_count(MapId::new(1)), 0);
        assert_eq!(
            manager.query_aabb_entities(map, Box2::new(-1.0, -1.0, 1.0, 1.0)),
            vec![second]
        );
        assert_eq!(
            manager.entities_in_grid_aabb(GridId::new(3), Box2::new(-0.5, -0.5, 0.5, 0.5), false),
            Vec::<EntityUid>::new()
        );
    }

    #[test]
    fn entity_manager_remove_map_component_preserves_child_map_velocities() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized(None);
        manager.ensure_map(MapId::new(14), map);
        manager.apply_transform_state(
            map,
            TransformComponentState {
                local_position: Vector2::new(0.0, 0.0),
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(14),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(manager.configure_physics_body(
            map,
            Some(crate::BodyType::Dynamic),
            None,
            None,
            None,
        ));
        assert!(manager.mutate_physics_and_reconcile(map, |body| {
            body.linear_velocity = Vector2::new(2.5, -1.0);
            body.angular_velocity = 0.6;
        }));

        let child = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            child,
            TransformComponentState {
                local_position: Vector2::new(1.0, 1.0),
                rotation: Angle::ZERO,
                parent_id: map,
                map_id: MapId::new(14),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(manager.configure_physics_body(
            child,
            Some(crate::BodyType::Dynamic),
            None,
            None,
            None,
        ));
        assert!(manager.mutate_physics_and_reconcile(child, |body| {
            body.linear_velocity = Vector2::new(0.25, 0.75);
            body.angular_velocity = -0.4;
        }));

        let old_linear = manager.map_linear_velocity(child);
        let old_angular = manager.map_angular_velocity(child);

        assert!(manager.remove_map_component(map));

        let child_transform = manager.transforms.get(&child).unwrap();
        assert_eq!(child_transform.parent, EntityUid::INVALID);
        let new_linear = manager.map_linear_velocity(child);
        let new_angular = manager.map_angular_velocity(child);
        assert!(new_linear.approx_eq_with_tolerance(old_linear, 0.0001));
        assert!((new_angular - old_angular).abs() <= 0.0001);
    }

    #[test]
    fn entity_manager_timer_runtime_cleans_empty_components_with_lifecycle() {
        use std::sync::{Arc, Mutex};

        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.initialize_entity(uid);
        let _ = manager.drain_entity_runtime_events();

        let fired = Arc::new(Mutex::new(0));
        let fired_clone = fired.clone();
        let _handle = manager.spawn_timer(uid, 20, move || {
            *fired_clone.lock().unwrap() += 1;
        });
        let _ = manager.drain_entity_runtime_events();

        manager.update_timer_runtime(0.03);

        assert_eq!(*fired.lock().unwrap(), 1);
        assert!(!manager.timers.contains_key(&uid));
        assert_eq!(
            manager.drain_entity_runtime_events(),
            vec![
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "TimerComponent".to_string(),
                    stage: ComponentLifeStage::Stopped,
                }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "TimerComponent".to_string(),
                    stage: ComponentLifeStage::Deleted,
                }),
            ]
        );
    }

    #[test]
    fn entity_manager_remove_timer_component_uses_shared_lifecycle_teardown() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.initialize_entity(uid);
        let _ = manager.spawn_timer(uid, 20, || {});
        let _ = manager.drain_entity_runtime_events();

        assert!(manager.remove_timer_component(uid));
        assert!(!manager.timers.contains_key(&uid));
        assert_eq!(
            manager.drain_entity_runtime_events(),
            vec![
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "TimerComponent".to_string(),
                    stage: ComponentLifeStage::Stopped,
                }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "TimerComponent".to_string(),
                    stage: ComponentLifeStage::Deleted,
                }),
            ]
        );
    }

    #[test]
    fn entity_manager_timer_runtime_skips_paused_entities() {
        use std::sync::{Arc, Mutex};

        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.initialize_entity(uid);
        let fired = Arc::new(Mutex::new(0));
        let fired_clone = fired.clone();
        let _handle = manager.spawn_timer(uid, 20, move || {
            *fired_clone.lock().unwrap() += 1;
        });
        assert!(manager.set_entity_paused(uid, true));

        manager.update_timer_runtime(0.03);
        assert_eq!(*fired.lock().unwrap(), 0);
        assert!(manager.timers.contains_key(&uid));

        assert!(manager.set_entity_paused(uid, false));
        manager.update_timer_runtime(0.03);
        assert_eq!(*fired.lock().unwrap(), 1);
        assert!(!manager.timers.contains_key(&uid));
    }

    #[test]
    fn entity_manager_removes_fixtures_component_with_runtime_cleanup() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized(None);
        manager.ensure_map(MapId::new(2), map);
        manager.transforms.get_mut(&map).unwrap().map_id = MapId::new(2);
        manager.ensure_broadphase(map);
        manager.ensure_physics_map(map);
        manager.ensure_lookup(map);

        let uid = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            uid,
            TransformComponentState {
                local_position: Vector2::new(0.5, 0.5),
                rotation: Angle::ZERO,
                parent_id: map,
                map_id: MapId::new(2),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(manager.configure_physics_body(
            uid,
            Some(crate::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        let _ = manager.insert_fixture_and_reconcile(
            uid,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        manager.refresh_entity_runtime(uid);
        assert_eq!(
            manager.query_aabb_entities(map, Box2::new(-1.0, -1.0, 1.0, 1.0)),
            vec![uid]
        );

        assert!(manager.remove_fixtures_component(uid));
        assert!(!manager.fixtures.contains_key(&uid));
        assert!(
            manager
                .query_aabb_entities(map, Box2::new(-1.0, -1.0, 1.0, 1.0))
                .is_empty()
        );
        assert_eq!(
            manager.entities_in_lookup_aabb(map, Box2::new(-1.0, -1.0, 1.0, 1.0), false),
            vec![uid]
        );
    }

    #[test]
    fn entity_manager_removes_joint_component_symmetrically_and_refreshes_contacts() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized(None);
        manager.ensure_map(MapId::new(3), map);
        manager.transforms.get_mut(&map).unwrap().map_id = MapId::new(3);
        manager.ensure_broadphase(map);
        manager.ensure_physics_map(map);

        let first = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            first,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(3),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(manager.configure_physics_body(
            first,
            Some(crate::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        let _ = manager.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "first",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let second = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            second,
            TransformComponentState {
                local_position: Vector2::new(0.5, 0.0),
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(3),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(manager.configure_physics_body(
            second,
            Some(crate::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        let _ = manager.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "second",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let mut joint =
            butsuri::Joint::new(first.raw(), second.raw(), butsuri::JointType::Distance);
        joint.id = "rope".to_string();
        joint.collide_connected = false;
        assert!(manager.add_joint_between(joint));

        manager.refresh_broadphase_runtime(map, &[first, second]);
        manager.refresh_contact_runtime(map, &[first, second]);
        assert_eq!(manager.owner_contact_count(map), 0);

        assert!(manager.remove_joint_component(first));
        assert!(!manager.joint_components.contains_key(&first));
        assert_eq!(
            manager.joint_components.get(&second).unwrap().joint_count(),
            0
        );
        assert_eq!(manager.owner_contact_count(map), 1);
    }

    #[test]
    fn entity_manager_removes_joints_when_physics_component_is_removed() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized(None);
        manager.ensure_map(MapId::new(4), map);
        manager.transforms.get_mut(&map).unwrap().map_id = MapId::new(4);
        manager.ensure_broadphase(map);
        manager.ensure_physics_map(map);

        let first = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            first,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(4),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(manager.configure_physics_body(
            first,
            Some(crate::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        let _ = manager.insert_fixture_and_reconcile(
            first,
            Fixture::new(
                "first",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let second = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            second,
            TransformComponentState {
                local_position: Vector2::new(0.5, 0.0),
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(4),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(manager.configure_physics_body(
            second,
            Some(crate::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        let _ = manager.insert_fixture_and_reconcile(
            second,
            Fixture::new(
                "second",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        let mut joint =
            butsuri::Joint::new(first.raw(), second.raw(), butsuri::JointType::Distance);
        joint.id = "rope".to_string();
        joint.collide_connected = false;
        assert!(manager.add_joint_between(joint));

        manager.refresh_map_physics_runtime(MapId::new(4));
        assert_eq!(manager.owner_contact_count(map), 0);

        assert!(manager.remove_physics_component(first));
        assert!(!manager.joint_components.contains_key(&first));
        assert_eq!(
            manager.joint_components.get(&second).unwrap().joint_count(),
            0
        );
        assert_eq!(manager.owner_contact_count(map), 0);
    }

    #[test]
    fn entity_manager_refresh_map_physics_runtime_creates_runtime_components_on_demand() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized(None);
        manager.ensure_map(MapId::new(5), map);
        manager.transforms.get_mut(&map).unwrap().map_id = MapId::new(5);

        let uid = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            uid,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(5),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(manager.configure_physics_body(
            uid,
            Some(crate::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));
        let _ = manager.insert_fixture_and_reconcile(
            uid,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );

        manager.refresh_map_physics_runtime(MapId::new(5));

        assert!(manager.has_broadphase_owner(map));
        assert!(manager.has_physics_runtime_owner(map));
        assert_eq!(
            manager.query_aabb_entities(map, Box2::new(-1.0, -1.0, 1.0, 1.0)),
            vec![uid]
        );
        assert!(manager.owner_contains_body(map, uid));
        assert!(manager.owner_contains_awake_body(map, uid));
    }

    #[test]
    fn entity_manager_keeps_metadata_map_id_in_sync_with_transform_and_map_components() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        assert_eq!(manager.metadata.get(&uid).unwrap().map_id, MapId::NULLSPACE);

        assert!(manager.apply_transform_state(
            uid,
            TransformComponentState {
                local_position: Vector2::new(1.0, 2.0),
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(21),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        ));
        assert_eq!(manager.metadata.get(&uid).unwrap().map_id, MapId::new(21));

        manager.ensure_map(MapId::new(22), uid);
        assert_eq!(manager.metadata.get(&uid).unwrap().map_id, MapId::new(22));

        assert!(manager.remove_map_component(uid));
        assert_eq!(manager.metadata.get(&uid).unwrap().map_id, MapId::NULLSPACE);
    }

    #[test]
    fn entity_manager_can_update_spatial_metadata_and_anchor_state_via_helpers() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        assert!(manager.set_spatial_metadata(uid, MapId::new(30), GridId::new(4)));
        let transform = manager.transforms.get(&uid).unwrap();
        assert_eq!(transform.map_id, MapId::new(30));
        assert_eq!(transform.grid_id, GridId::new(4));
        assert_eq!(manager.metadata.get(&uid).unwrap().map_id, MapId::new(30));

        assert!(manager.set_anchored(uid, true));
        assert!(manager.transforms.get(&uid).unwrap().anchored);
        assert!(!manager.set_anchored(uid, true));
    }

    #[test]
    fn entity_manager_delete_removes_joints_from_peer_components() {
        let mut manager = EntityManager::new();
        let first = manager.create_entity_uninitialized(None);
        let second = manager.create_entity_uninitialized(None);
        let mut joint =
            butsuri::Joint::new(first.raw(), second.raw(), butsuri::JointType::Distance);
        joint.id = "rope".to_string();
        assert!(manager.add_joint_between(joint));

        manager.queue_delete_entity(first);
        manager.flush_queued_deletions();

        assert!(!manager.joint_components.contains_key(&first));
        assert_eq!(
            manager.joint_components.get(&second).unwrap().joint_count(),
            0
        );
    }

    #[test]
    fn entity_manager_apply_physics_state_queues_runtime_events_for_snapshot_transitions() {
        let mut manager = EntityManager::new();
        let map_owner = manager.create_entity_uninitialized_as_map(None, MapId::new(50));
        manager.ensure_physics_map(map_owner);

        let uid = manager.create_entity_uninitialized(None);
        let _ = manager.apply_transform_state(
            uid,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(50),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(manager.configure_physics_body(
            uid,
            Some(crate::BodyType::Dynamic),
            Some(true),
            Some(true),
            None,
        ));

        assert!(manager.apply_physics_component_state(
            uid,
            crate::PhysicsComponentState {
                can_collide: false,
                awake: false,
                sleeping_allowed: true,
                fixed_rotation: false,
                status: crate::BodyStatus::OnGround,
                linear_velocity: Vector2::ZERO,
                angular_velocity: 0.0,
                body_type: crate::BodyType::Dynamic,
            },
        ));

        let events = manager
            .physics_maps
            .get_mut(&map_owner)
            .unwrap()
            .drain_runtime_events();
        assert!(
            events.contains(&PhysicsRuntimeEvent::Sleep(crate::PhysicsSleepMessage {
                body: uid,
            }))
        );
        assert!(events.contains(&PhysicsRuntimeEvent::CollisionChange(
            crate::CollisionChangeMessage {
                owner: uid,
                can_collide: false,
            }
        )));
    }

    #[test]
    fn entity_manager_apply_joint_state_queues_runtime_events_for_snapshot_deltas() {
        let mut manager = EntityManager::new();
        let map_owner = manager.create_entity_uninitialized_as_map(None, MapId::new(51));
        manager.ensure_physics_map(map_owner);

        let uid = manager.create_entity_uninitialized(None);
        let _ = manager.apply_transform_state(
            uid,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(51),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );

        let mut joint = butsuri::Joint::new(uid.raw(), 999, butsuri::JointType::Distance);
        joint.id = "rope".to_string();
        assert!(manager.apply_joint_component_state(
            uid,
            crate::JointComponentState {
                joints: vec![joint.clone()],
            },
        ));

        let added = manager
            .physics_maps
            .get_mut(&map_owner)
            .unwrap()
            .drain_runtime_events();
        assert_eq!(
            added,
            vec![PhysicsRuntimeEvent::JointAdded(crate::JointAddedEvent {
                body_a: EntityUid::new(joint.body_a_uid),
                body_b: EntityUid::new(joint.body_b_uid),
                joint_id: "rope".to_string(),
            })]
        );

        assert!(
            manager.apply_joint_component_state(
                uid,
                crate::JointComponentState { joints: Vec::new() },
            )
        );
        let removed = manager
            .physics_maps
            .get_mut(&map_owner)
            .unwrap()
            .drain_runtime_events();
        assert_eq!(
            removed,
            vec![PhysicsRuntimeEvent::JointRemoved(
                crate::JointRemovedEvent {
                    body_a: EntityUid::new(joint.body_a_uid),
                    body_b: EntityUid::new(joint.body_b_uid),
                    joint_id: "rope".to_string(),
                }
            )]
        );
    }

    #[test]
    fn entity_manager_queues_physics_initialized_once_for_new_body() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.drain_entity_runtime_events();

        let _ = manager.ensure_physics(uid);
        assert_eq!(
            manager.drain_entity_runtime_events(),
            vec![EntityRuntimeEvent::ComponentLifecycle(
                ComponentLifecycleEvent {
                    owner: uid,
                    component: "Physics".to_string(),
                    stage: ComponentLifeStage::Added,
                }
            )]
        );

        manager.initialize_entity(uid);
        let init_events = manager.drain_entity_runtime_events();
        assert!(
            init_events.contains(&EntityRuntimeEvent::ComponentLifecycle(
                ComponentLifecycleEvent {
                    owner: uid,
                    component: "Physics".to_string(),
                    stage: ComponentLifeStage::Initialized,
                }
            ))
        );
        assert!(
            init_events.contains(&EntityRuntimeEvent::PhysicsInitialized(
                PhysicsInitializedEvent { uid }
            ))
        );

        let _ = manager.ensure_physics(uid);
        assert!(manager.drain_entity_runtime_events().is_empty());
    }

    #[test]
    fn entity_manager_queues_parent_and_anchor_runtime_events_for_transform_changes() {
        let mut manager = EntityManager::new();
        let parent = manager.create_entity_uninitialized(None);
        let child = manager.create_entity_uninitialized(None);
        manager.drain_entity_runtime_events();

        assert!(manager.set_parent(child, parent));
        assert!(manager.set_anchored(child, true));
        assert!(manager.apply_transform_state(
            child,
            TransformComponentState {
                local_position: Vector2::new(1.0, 0.0),
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::NULLSPACE,
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        ));

        assert_eq!(
            manager.drain_entity_runtime_events(),
            vec![
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: parent,
                    component: "EntityLookupComponent".to_string(),
                    stage: ComponentLifeStage::Added,
                }),
                EntityRuntimeEvent::ParentChanged(EntParentChangedMessage {
                    entity: child,
                    old_parent: None,
                }),
                EntityRuntimeEvent::AnchorStateChanged(AnchorStateChangedEvent {
                    entity: child,
                    anchored: true,
                }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: child,
                    component: "EntityLookupComponent".to_string(),
                    stage: ComponentLifeStage::Added,
                }),
                EntityRuntimeEvent::ParentChanged(EntParentChangedMessage {
                    entity: child,
                    old_parent: Some(parent),
                }),
                EntityRuntimeEvent::AnchorStateChanged(AnchorStateChangedEvent {
                    entity: child,
                    anchored: false,
                }),
            ]
        );
    }

    #[test]
    fn entity_manager_queues_entity_lifecycle_runtime_events() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.drain_entity_runtime_events();

        let init = manager.initialize_entity(uid);
        assert_eq!(init, EntityInitializedMessage { entity: uid });
        assert_eq!(
            manager.drain_entity_runtime_events(),
            vec![
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "TransformComponent".to_string(),
                    stage: ComponentLifeStage::Initialized,
                }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "MetaDataComponent".to_string(),
                    stage: ComponentLifeStage::Initialized,
                }),
                EntityRuntimeEvent::EntityInitialized(EntityInitializedMessage { entity: uid }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "TransformComponent".to_string(),
                    stage: ComponentLifeStage::Running,
                }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "MetaDataComponent".to_string(),
                    stage: ComponentLifeStage::Running,
                }),
            ]
        );

        manager.queue_delete_entity(uid);
        manager.flush_queued_deletions();
        assert_eq!(
            manager.drain_entity_runtime_events(),
            vec![
                EntityRuntimeEvent::EntityTerminating(EntityTerminatingEvent { entity: uid }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "TransformComponent".to_string(),
                    stage: ComponentLifeStage::Stopped,
                }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "TransformComponent".to_string(),
                    stage: ComponentLifeStage::Deleted,
                }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "MetaDataComponent".to_string(),
                    stage: ComponentLifeStage::Stopped,
                }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "MetaDataComponent".to_string(),
                    stage: ComponentLifeStage::Deleted,
                }),
                EntityRuntimeEvent::EntityDeleted(EntityDeletedMessage { entity: uid }),
            ]
        );
    }

    #[test]
    fn entity_manager_queues_component_lifecycle_events_for_new_and_initialized_entities() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.drain_entity_runtime_events();

        let _ = manager.ensure_appearance(uid);
        assert_eq!(
            manager.drain_entity_runtime_events(),
            vec![EntityRuntimeEvent::ComponentLifecycle(
                ComponentLifecycleEvent {
                    owner: uid,
                    component: "AppearanceComponent".to_string(),
                    stage: ComponentLifeStage::Added,
                }
            )]
        );

        manager.initialize_entity(uid);
        assert_eq!(
            manager.drain_entity_runtime_events(),
            vec![
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "TransformComponent".to_string(),
                    stage: ComponentLifeStage::Initialized,
                }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "AppearanceComponent".to_string(),
                    stage: ComponentLifeStage::Initialized,
                }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "MetaDataComponent".to_string(),
                    stage: ComponentLifeStage::Initialized,
                }),
                EntityRuntimeEvent::EntityInitialized(EntityInitializedMessage { entity: uid }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "TransformComponent".to_string(),
                    stage: ComponentLifeStage::Running,
                }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "AppearanceComponent".to_string(),
                    stage: ComponentLifeStage::Running,
                }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "MetaDataComponent".to_string(),
                    stage: ComponentLifeStage::Running,
                }),
            ]
        );

        let _ = manager.ensure_physics(uid);
        assert_eq!(
            manager.drain_entity_runtime_events(),
            vec![
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "Physics".to_string(),
                    stage: ComponentLifeStage::Added,
                }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "Physics".to_string(),
                    stage: ComponentLifeStage::Initialized,
                }),
                EntityRuntimeEvent::PhysicsInitialized(PhysicsInitializedEvent { uid }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "EntityLookupComponent".to_string(),
                    stage: ComponentLifeStage::Added,
                }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "EntityLookupComponent".to_string(),
                    stage: ComponentLifeStage::Initialized,
                }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "EntityLookupComponent".to_string(),
                    stage: ComponentLifeStage::Running,
                }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "Physics".to_string(),
                    stage: ComponentLifeStage::Running,
                }),
            ]
        );
    }

    #[test]
    fn entity_manager_queues_component_shutdown_and_delete_on_explicit_remove() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.initialize_entity(uid);
        manager.drain_entity_runtime_events();

        let _ = manager.ensure_physics(uid);
        manager.drain_entity_runtime_events();

        assert!(manager.remove_physics_component(uid));
        assert_eq!(
            manager.drain_entity_runtime_events(),
            vec![
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "Physics".to_string(),
                    stage: ComponentLifeStage::Stopped,
                }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "Physics".to_string(),
                    stage: ComponentLifeStage::Deleted,
                }),
            ]
        );
    }

    #[test]
    fn entity_manager_queues_metadata_component_shutdown_and_delete_on_explicit_remove() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.initialize_entity(uid);
        manager.drain_entity_runtime_events();

        assert!(manager.remove_component_by_net_id(uid, EntityManager::METADATA_NET_ID));
        assert_eq!(
            manager.drain_entity_runtime_events(),
            vec![
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "MetaDataComponent".to_string(),
                    stage: ComponentLifeStage::Stopped,
                }),
                EntityRuntimeEvent::ComponentLifecycle(ComponentLifecycleEvent {
                    owner: uid,
                    component: "MetaDataComponent".to_string(),
                    stage: ComponentLifeStage::Deleted,
                }),
            ]
        );
    }

    #[test]
    fn entity_manager_raises_component_lifecycle_events_in_rt_order() {
        #[derive(Clone)]
        struct LifecycleProbe {
            info: EntitySystemInfo,
            hits: Arc<Mutex<Vec<&'static str>>>,
        }

        impl EntitySystem for LifecycleProbe {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }

            fn info(&self) -> &EntitySystemInfo {
                &self.info
            }

            fn register_subscriptions(&self, subscriptions: &mut EntitySystemSubscriptions<'_>) {
                let hits = self.hits.clone();
                subscriptions.subscribe_local_event(
                    "Appearance",
                    move |_uid, _component: &str, _ev: &mut crate::ComponentAdd| {
                        hits.lock().unwrap().push("add");
                    },
                    std::iter::empty::<String>(),
                    std::iter::empty::<String>(),
                );
                let hits = self.hits.clone();
                subscriptions.subscribe_local_event(
                    "Appearance",
                    move |_uid, _component: &str, _ev: &mut crate::ComponentInit| {
                        hits.lock().unwrap().push("init");
                    },
                    std::iter::empty::<String>(),
                    std::iter::empty::<String>(),
                );
                let hits = self.hits.clone();
                subscriptions.subscribe_local_event(
                    "Appearance",
                    move |_uid, _component: &str, _ev: &mut crate::EntityInitializedMessage| {
                        hits.lock().unwrap().push("entity-init");
                    },
                    std::iter::empty::<String>(),
                    std::iter::empty::<String>(),
                );
                let hits = self.hits.clone();
                subscriptions.subscribe_local_event(
                    "Appearance",
                    move |_uid, _component: &str, _ev: &mut crate::ComponentStartup| {
                        hits.lock().unwrap().push("startup");
                    },
                    std::iter::empty::<String>(),
                    std::iter::empty::<String>(),
                );
                let hits = self.hits.clone();
                subscriptions.subscribe_local_event(
                    "Appearance",
                    move |_uid, _component: &str, _ev: &mut crate::ComponentShutdown| {
                        hits.lock().unwrap().push("shutdown");
                    },
                    std::iter::empty::<String>(),
                    std::iter::empty::<String>(),
                );
                let hits = self.hits.clone();
                subscriptions.subscribe_local_event(
                    "Appearance",
                    move |_uid, _component: &str, _ev: &mut crate::ComponentRemove| {
                        hits.lock().unwrap().push("remove");
                    },
                    std::iter::empty::<String>(),
                    std::iter::empty::<String>(),
                );
            }
        }

        let hits = Arc::new(Mutex::new(Vec::new()));
        let probe = LifecycleProbe {
            info: EntitySystemInfo::new("lifecycle_probe"),
            hits: hits.clone(),
        };

        let mut manager = EntityManager::new();
        manager.add_entity_system(Box::new(probe));
        manager.initialize_entity_systems();

        let uid = manager.create_entity_uninitialized(None);
        manager.drain_entity_runtime_events();
        let _ = manager.ensure_appearance(uid);
        manager.initialize_entity(uid);
        assert!(manager.remove_component_by_net_id(uid, EntityManager::APPEARANCE_NET_ID));

        assert_eq!(
            hits.lock().unwrap().clone(),
            vec![
                "add",
                "init",
                "entity-init",
                "startup",
                "shutdown",
                "remove"
            ]
        );
    }

    #[test]
    fn entity_manager_delete_entity_queues_base_component_shutdown_and_delete() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.initialize_entity(uid);
        manager.drain_entity_runtime_events();

        manager.queue_delete_entity(uid);
        manager.flush_queued_deletions();

        let events = manager.drain_entity_runtime_events();
        assert!(events.contains(&EntityRuntimeEvent::ComponentLifecycle(
            ComponentLifecycleEvent {
                owner: uid,
                component: "MetaDataComponent".to_string(),
                stage: ComponentLifeStage::Stopped,
            }
        )));
        assert!(events.contains(&EntityRuntimeEvent::ComponentLifecycle(
            ComponentLifecycleEvent {
                owner: uid,
                component: "MetaDataComponent".to_string(),
                stage: ComponentLifeStage::Deleted,
            }
        )));
        assert!(events.contains(&EntityRuntimeEvent::ComponentLifecycle(
            ComponentLifecycleEvent {
                owner: uid,
                component: "TransformComponent".to_string(),
                stage: ComponentLifeStage::Stopped,
            }
        )));
        assert!(events.contains(&EntityRuntimeEvent::ComponentLifecycle(
            ComponentLifecycleEvent {
                owner: uid,
                component: "TransformComponent".to_string(),
                stage: ComponentLifeStage::Deleted,
            }
        )));
        assert!(events.contains(&EntityRuntimeEvent::EntityTerminating(
            EntityTerminatingEvent { entity: uid }
        )));
        assert!(
            events.contains(&EntityRuntimeEvent::EntityDeleted(EntityDeletedMessage {
                entity: uid
            }))
        );
    }

    #[test]
    fn entity_manager_can_queue_pause_events() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.drain_entity_runtime_events();

        assert!(manager.set_entity_paused(uid, true));
        assert!(manager.is_entity_paused(uid));
        assert_eq!(
            manager.drain_entity_runtime_events(),
            vec![EntityRuntimeEvent::EntityPaused(crate::EntityPausedEvent {
                entity: uid,
                paused: true,
            })]
        );
        assert!(!manager.set_entity_paused(uid, true));
    }

    #[test]
    fn entity_manager_runtime_events_publish_into_entity_event_bus() {
        struct PauseSystem {
            info: EntitySystemInfo,
            hits: Arc<Mutex<Vec<(i32, bool)>>>,
        }

        impl EntitySystem for PauseSystem {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }

            fn info(&self) -> &EntitySystemInfo {
                &self.info
            }

            fn register_subscriptions(&self, subscriptions: &mut EntitySystemSubscriptions<'_>) {
                let hits = self.hits.clone();
                subscriptions.subscribe_local_event(
                    "MetaDataComponent",
                    move |uid, _component: &str, ev: &mut EntityPausedEvent| {
                        hits.lock().unwrap().push((uid.raw(), ev.paused));
                    },
                    std::iter::empty::<String>(),
                    std::iter::empty::<String>(),
                );
            }
        }

        let hits = Arc::new(Mutex::new(Vec::new()));
        let mut manager = EntityManager::new();
        manager.add_entity_system(Box::new(PauseSystem {
            info: EntitySystemInfo::new("pause"),
            hits: hits.clone(),
        }));
        let uid = manager.create_entity_uninitialized(None);
        manager.drain_entity_runtime_events();

        assert!(manager.set_entity_paused(uid, true));
        assert_eq!(&*hits.lock().unwrap(), &[(uid.raw(), true)]);
    }

    #[test]
    fn entity_manager_entity_component_names_follow_canonical_runtime_order() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.ensure_appearance(uid);
        manager.ensure_map(MapId::new(80), uid);
        manager.ensure_physics(uid);
        manager.ensure_ignore_pause(uid);
        manager.ensure_timer(uid);

        assert_eq!(
            manager.entity_component_names(uid),
            vec![
                "TransformComponent".to_string(),
                "MetaDataComponent".to_string(),
                "MapComponent".to_string(),
                "Appearance".to_string(),
                "IgnorePauseComponent".to_string(),
                "TimerComponent".to_string(),
                "Physics".to_string(),
            ]
        );
    }

    #[test]
    fn entity_manager_queues_map_pause_events_for_runtime_and_snapshot_state() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        assert!(manager.apply_map_component_state(
            uid,
            MapComponentState {
                map_id: MapId::new(61),
                lighting_enabled: true,
                map_paused: false,
            },
        ));
        manager.drain_entity_runtime_events();

        assert!(manager.set_map_paused(uid, true));
        assert!(manager.is_map_paused(MapId::new(61)));
        assert_eq!(
            manager.drain_entity_runtime_events(),
            vec![
                EntityRuntimeEvent::EntityPaused(crate::EntityPausedEvent {
                    entity: uid,
                    paused: true,
                }),
                EntityRuntimeEvent::MapPaused(crate::MapPausedEvent {
                    entity: uid,
                    paused: true,
                }),
            ]
        );

        assert!(manager.apply_map_component_state(
            uid,
            MapComponentState {
                map_id: MapId::new(61),
                lighting_enabled: true,
                map_paused: false,
            },
        ));
        assert_eq!(
            manager.drain_entity_runtime_events(),
            vec![
                EntityRuntimeEvent::EntityPaused(crate::EntityPausedEvent {
                    entity: uid,
                    paused: false,
                }),
                EntityRuntimeEvent::MapPaused(crate::MapPausedEvent {
                    entity: uid,
                    paused: false,
                }),
            ]
        );
    }

    #[test]
    fn entity_manager_map_pause_propagates_entity_pause_recursively() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_as_map(None, MapId::new(62));
        let grid = manager.create_entity_uninitialized_as_grid(
            None,
            MapId::new(62),
            map,
            GridId::new(162),
            8,
            Vector2::ZERO,
            Angle::ZERO,
        );
        let child = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: grid,
                map_id: MapId::new(62),
                grid_id: GridId::new(162),
                no_local_rotation: false,
                anchored: false,
            },
        );
        manager.drain_entity_runtime_events();

        assert!(manager.set_map_paused(map, true));
        assert!(manager.is_entity_paused(map));
        assert!(manager.is_entity_paused(grid));
        assert!(manager.is_entity_paused(child));

        let events = manager.drain_entity_runtime_events();
        assert!(
            events.contains(&EntityRuntimeEvent::MapPaused(crate::MapPausedEvent {
                entity: map,
                paused: true,
            }))
        );
        assert!(events.contains(&EntityRuntimeEvent::EntityPaused(
            crate::EntityPausedEvent {
                entity: map,
                paused: true,
            }
        )));
        assert!(events.contains(&EntityRuntimeEvent::EntityPaused(
            crate::EntityPausedEvent {
                entity: grid,
                paused: true,
            }
        )));
        assert!(events.contains(&EntityRuntimeEvent::EntityPaused(
            crate::EntityPausedEvent {
                entity: child,
                paused: true,
            }
        )));
    }

    #[test]
    fn entity_manager_map_pre_init_counts_as_paused_and_initialization_state() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_as_map(None, MapId::new(63));
        let child = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: map,
                map_id: MapId::new(63),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        manager.drain_entity_runtime_events();

        assert!(manager.set_map_pre_init(map, true));
        assert!(manager.is_map_paused(MapId::new(63)));
        assert!(!manager.is_map_initialized(MapId::new(63)));
        assert!(manager.is_entity_paused(map));
        assert!(manager.is_entity_paused(child));

        let events = manager.drain_entity_runtime_events();
        assert!(
            events.contains(&EntityRuntimeEvent::MapPaused(crate::MapPausedEvent {
                entity: map,
                paused: true,
            }))
        );

        assert!(manager.set_map_pre_init(map, false));
        assert!(!manager.is_map_paused(MapId::new(63)));
        assert!(manager.is_map_initialized(MapId::new(63)));
        assert!(!manager.is_entity_paused(map));
        assert!(!manager.is_entity_paused(child));
    }

    #[test]
    fn entity_manager_map_initialize_runs_map_init_recursively_for_initialized_entities() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_as_map(None, MapId::new(163));
        let child = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: map,
                map_id: MapId::new(163),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );

        assert!(manager.set_map_pre_init(map, true));
        manager.initialize_entity(map);
        manager.initialize_entity(child);
        manager.drain_entity_runtime_events();

        assert!(manager.set_map_pre_init(map, false));
        let events = manager.drain_entity_runtime_events();
        assert_eq!(
            manager.metadata.get(&map).unwrap().entity_life_stage,
            crate::EntityLifeStage::MapInitialized
        );
        assert_eq!(
            manager.metadata.get(&child).unwrap().entity_life_stage,
            crate::EntityLifeStage::MapInitialized
        );
        assert!(
            events.contains(&EntityRuntimeEvent::MapInit(crate::MapInitEvent {
                entity: map,
            }))
        );
        assert!(
            events.contains(&EntityRuntimeEvent::MapInit(crate::MapInitEvent {
                entity: child,
            }))
        );
    }

    #[test]
    fn entity_manager_initialize_entity_runs_map_init_when_target_map_is_initialized() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_as_map(None, MapId::new(164));
        manager.initialize_entity(map);
        manager.drain_entity_runtime_events();

        let uid = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: map,
                map_id: MapId::new(164),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        manager.drain_entity_runtime_events();

        manager.initialize_entity(uid);
        let events = manager.drain_entity_runtime_events();
        assert_eq!(
            manager.metadata.get(&uid).unwrap().entity_life_stage,
            crate::EntityLifeStage::MapInitialized
        );
        assert!(
            events.contains(&EntityRuntimeEvent::MapInit(crate::MapInitEvent {
                entity: uid,
            }))
        );
    }

    #[test]
    fn entity_manager_transform_to_initialized_map_runs_map_init() {
        let mut manager = EntityManager::new();
        let pre_init_map = manager.create_entity_uninitialized_as_map(None, MapId::new(165));
        let initialized_map = manager.create_entity_uninitialized_as_map(None, MapId::new(166));
        assert!(manager.set_map_pre_init(pre_init_map, true));
        manager.initialize_entity(initialized_map);

        let uid = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: pre_init_map,
                map_id: MapId::new(165),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        manager.initialize_entity(uid);
        manager.drain_entity_runtime_events();

        assert_eq!(
            manager.metadata.get(&uid).unwrap().entity_life_stage,
            crate::EntityLifeStage::Initialized
        );
        assert!(manager.apply_transform_state(
            uid,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: initialized_map,
                map_id: MapId::new(166),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        ));

        let events = manager.drain_entity_runtime_events();
        assert_eq!(
            manager.metadata.get(&uid).unwrap().entity_life_stage,
            crate::EntityLifeStage::MapInitialized
        );
        assert!(
            events.contains(&EntityRuntimeEvent::MapInit(crate::MapInitEvent {
                entity: uid,
            }))
        );
    }

    #[test]
    fn entity_manager_map_component_added_after_initialize_runs_map_init() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.initialize_entity(uid);
        manager.drain_entity_runtime_events();

        assert!(manager.apply_map_component_state(
            uid,
            crate::MapComponentState {
                map_id: MapId::new(167),
                lighting_enabled: true,
                map_paused: false,
            },
        ));
        let events = manager.drain_entity_runtime_events();

        assert_eq!(
            manager.metadata.get(&uid).unwrap().entity_life_stage,
            crate::EntityLifeStage::MapInitialized
        );
        assert!(
            events.contains(&EntityRuntimeEvent::MapInit(crate::MapInitEvent {
                entity: uid,
            }))
        );
    }

    #[test]
    fn entity_manager_transform_map_change_updates_entity_paused_from_target_map() {
        let mut manager = EntityManager::new();
        let paused_map = manager.create_entity_uninitialized_as_map(None, MapId::new(64));
        let clear_map = manager.create_entity_uninitialized_as_map(None, MapId::new(65));
        assert!(manager.set_map_paused(paused_map, true));
        manager.drain_entity_runtime_events();

        let uid = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: clear_map,
                map_id: MapId::new(65),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        manager.drain_entity_runtime_events();
        assert!(!manager.is_entity_paused(uid));

        assert!(manager.apply_transform_state(
            uid,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: paused_map,
                map_id: MapId::new(64),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        ));
        assert!(manager.is_entity_paused(uid));
    }

    #[test]
    fn entity_manager_ignore_pause_component_blocks_map_pause_on_entity() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_as_map(None, MapId::new(66));
        let uid = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: map,
                map_id: MapId::new(66),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(manager.ensure_ignore_pause(uid).base.owner == uid);
        manager.drain_entity_runtime_events();

        assert!(manager.set_map_paused(map, true));
        assert!(!manager.is_entity_paused(uid));
        assert!(manager.is_entity_paused(map));
    }

    #[test]
    fn entity_manager_removing_ignore_pause_restores_pause_from_current_map() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_as_map(None, MapId::new(67));
        let uid = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: map,
                map_id: MapId::new(67),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let _ = manager.ensure_ignore_pause(uid);
        assert!(manager.set_map_paused(map, true));
        manager.drain_entity_runtime_events();

        assert!(manager.remove_ignore_pause_component(uid));
        assert!(manager.is_entity_paused(uid));
        assert!(
            manager
                .drain_entity_runtime_events()
                .contains(&EntityRuntimeEvent::EntityPaused(
                    crate::EntityPausedEvent {
                        entity: uid,
                        paused: true,
                    }
                ))
        );
    }

    #[test]
    fn entity_manager_entity_query_filters_paused_by_default() {
        let mut manager = EntityManager::new();
        let active = manager.create_entity_uninitialized(None);
        let paused = manager.create_entity_uninitialized(None);
        manager.ensure_transform(active);
        manager.ensure_transform(paused);
        assert!(manager.set_entity_paused(paused, true));

        let default = manager.entity_query(&manager.transforms, false);
        assert_eq!(default.len(), 1);
        assert_eq!(default[0].0, active);

        let include_paused = manager.entity_query(&manager.transforms, true);
        assert_eq!(include_paused.len(), 2);
        assert!(include_paused.iter().any(|(uid, _)| *uid == active));
        assert!(include_paused.iter().any(|(uid, _)| *uid == paused));
    }

    #[test]
    fn entity_manager_entity_query2_filters_paused_by_default() {
        let mut manager = EntityManager::new();
        let active = manager.create_entity_uninitialized(None);
        let paused = manager.create_entity_uninitialized(None);
        manager.ensure_transform(active);
        manager.ensure_appearance(active);
        manager.ensure_transform(paused);
        manager.ensure_appearance(paused);
        assert!(manager.set_entity_paused(paused, true));

        let default = manager.entity_query2(&manager.transforms, &manager.appearances, false);
        assert_eq!(default.len(), 1);
        assert_eq!(default[0].0, active);

        let include_paused = manager.entity_query2(&manager.transforms, &manager.appearances, true);
        assert_eq!(include_paused.len(), 2);
    }

    #[test]
    fn entity_manager_entity_query3_and_4_match_rt_like_pause_rules() {
        let mut manager = EntityManager::new();
        let active = manager.create_entity_uninitialized(None);
        let paused = manager.create_entity_uninitialized(None);

        manager.ensure_transform(active);
        manager.ensure_appearance(active);
        manager.ensure_lookup(active);
        manager.ensure_timer(active);

        manager.ensure_transform(paused);
        manager.ensure_appearance(paused);
        manager.ensure_lookup(paused);
        manager.ensure_timer(paused);
        assert!(manager.set_entity_paused(paused, true));

        let query3 = manager.entity_query3(
            &manager.transforms,
            &manager.appearances,
            &manager.entity_lookups,
            false,
        );
        assert_eq!(query3.len(), 1);
        assert_eq!(query3[0].0, active);

        let query4 = manager.entity_query4(
            &manager.transforms,
            &manager.appearances,
            &manager.entity_lookups,
            &manager.timers,
            true,
        );
        assert_eq!(query4.len(), 2);
    }

    #[test]
    fn entity_manager_get_all_components_filters_paused_by_default() {
        let mut manager = EntityManager::new();
        let active = manager.create_entity_uninitialized(None);
        let paused = manager.create_entity_uninitialized(None);
        manager.ensure_physics(active);
        manager.ensure_physics(paused);
        assert!(manager.set_entity_paused(paused, true));

        assert_eq!(manager.get_all_components(&manager.physics, false).len(), 1);
        assert_eq!(manager.get_all_components(&manager.physics, true).len(), 2);
    }

    #[test]
    fn entity_manager_children_of_uses_transform_tree_runtime() {
        let mut manager = EntityManager::new();
        let parent = manager.create_entity_uninitialized(None);
        let child_a = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: parent,
                map_id: MapId::NULLSPACE,
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let child_b = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::new(1.0, 0.0),
                rotation: Angle::ZERO,
                parent_id: parent,
                map_id: MapId::NULLSPACE,
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );

        let mut children = manager.children_of(parent);
        children.sort();
        assert_eq!(children, vec![child_a, child_b]);
    }

    #[test]
    fn entity_manager_physics_bodies_in_map_filters_by_map_fixture_and_pause_flag() {
        let mut manager = EntityManager::new();
        let map_a = MapId::new(68);
        let map_b = MapId::new(69);

        let active = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: map_a,
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let paused = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: map_a,
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let other_map = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: map_b,
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );

        for uid in [active, paused, other_map] {
            let body = manager.ensure_physics(uid);
            body.can_collide = true;
            body.set_body_type(crate::BodyType::Dynamic);
            let _ = manager.insert_fixture_and_reconcile(
                uid,
                butsuri::Fixture::new(
                    "main",
                    butsuri::PhysShape::Aabb(butsuri::AabbShape::new(
                        Box2::new(-0.5, -0.5, 0.5, 0.5),
                        0.0,
                    )),
                ),
            );
        }
        assert!(manager.set_entity_paused(paused, true));

        assert_eq!(manager.physics_bodies_in_map(map_a, false), vec![active]);

        let mut include_paused = manager.physics_bodies_in_map(map_a, true);
        include_paused.sort();
        assert_eq!(include_paused, vec![active, paused]);
        assert_eq!(manager.physics_bodies_in_map(map_b, true), vec![other_map]);
    }

    #[test]
    fn entity_manager_find_grids_intersecting_and_try_find_grid_use_shared_runtime() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_as_map(None, MapId::new(70));
        let first = manager.create_entity_uninitialized_as_grid(
            None,
            MapId::new(70),
            map,
            GridId::new(170),
            8,
            Vector2::ZERO,
            Angle::ZERO,
        );
        let second = manager.create_entity_uninitialized_as_grid(
            None,
            MapId::new(70),
            map,
            GridId::new(171),
            8,
            Vector2::new(10.0, 0.0),
            Angle::ZERO,
        );
        manager
            .map_grids
            .get_mut(&first)
            .unwrap()
            .set_tile(Vector2i::new(0, 0), Tile::new(1, TileRenderFlag(0), 0));
        manager
            .map_grids
            .get_mut(&second)
            .unwrap()
            .set_tile(Vector2i::new(0, 0), Tile::new(1, TileRenderFlag(0), 0));

        assert_eq!(
            manager.try_find_grid_at(MapId::new(70), Vector2::new(0.5, 0.5)),
            Some(GridId::new(170))
        );
        assert_eq!(
            manager.try_find_grid_at(MapId::new(70), Vector2::new(10.5, 0.5)),
            Some(GridId::new(171))
        );
        assert_eq!(
            manager.find_grids_intersecting(MapId::new(70), Box2::new(-1.0, -1.0, 1.0, 1.0), false),
            vec![GridId::new(170)]
        );

        let mut approx =
            manager.find_grids_intersecting(MapId::new(70), Box2::new(-1.0, -1.0, 12.0, 1.0), true);
        approx.sort_by_key(|grid_id| grid_id.raw());
        assert_eq!(approx, vec![GridId::new(170), GridId::new(171)]);
    }

    #[test]
    fn entity_manager_is_grid_paused_follows_parent_map_runtime() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_as_map(None, MapId::new(71));
        let _grid = manager.create_entity_uninitialized_as_grid(
            None,
            MapId::new(71),
            map,
            GridId::new(172),
            8,
            Vector2::ZERO,
            Angle::ZERO,
        );

        assert!(!manager.is_grid_paused(GridId::new(172)));
        assert!(manager.set_map_pre_init(map, true));
        assert!(manager.is_grid_paused(GridId::new(172)));
        assert!(manager.set_map_pre_init(map, false));
        assert!(!manager.is_grid_paused(GridId::new(172)));
        assert!(manager.is_grid_paused(GridId::new(999)));
    }

    #[test]
    fn entity_manager_exposes_shared_tile_and_lookup_queries() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_as_map(None, MapId::new(72));
        let grid = manager.create_entity_uninitialized_as_grid(
            None,
            MapId::new(72),
            map,
            GridId::new(173),
            8,
            Vector2::ZERO,
            Angle::ZERO,
        );
        manager
            .map_grids
            .get_mut(&grid)
            .unwrap()
            .set_tile(Vector2i::new(0, 0), Tile::new(1, TileRenderFlag(0), 0));

        let uid = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::new(0.5, 0.5),
                rotation: Angle::ZERO,
                parent_id: grid,
                map_id: MapId::new(72),
                grid_id: GridId::new(173),
                no_local_rotation: false,
                anchored: false,
            },
        );
        let _ = manager.refresh_entity_lookup(uid);

        assert_eq!(
            manager
                .tile_ref(GridId::new(173), Vector2i::new(0, 0))
                .unwrap()
                .tile
                .type_id,
            1
        );
        assert!(
            manager
                .tile_world_bounds(GridId::new(173), Vector2i::new(0, 0))
                .unwrap()
                .contains(Vector2::new(0.5, 0.5), true)
        );
        assert_eq!(
            manager.entities_at_tile(GridId::new(173), Vector2i::new(0, 0)),
            vec![uid]
        );
        assert_eq!(
            manager.entities_at_tiles(GridId::new(173), [Vector2i::new(0, 0), Vector2i::new(0, 0)]),
            vec![uid]
        );
        assert_eq!(
            manager.entities_in_grid_aabb(GridId::new(173), Box2::new(-1.0, -1.0, 1.0, 1.0), false),
            vec![uid]
        );
        assert!(
            manager
                .entity_world_aabb(uid)
                .unwrap()
                .contains(Vector2::new(0.5, 0.5), true)
        );
    }

    #[test]
    fn entity_manager_exposes_shared_map_physics_queries() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_as_map(None, MapId::new(73));
        let uid = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::new(5.0, 0.0),
                rotation: Angle::ZERO,
                parent_id: map,
                map_id: MapId::new(73),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );

        let body = manager.ensure_physics(uid);
        body.can_collide = true;
        body.set_body_type(crate::BodyType::Dynamic);
        body.awake = true;
        let _ = manager.insert_fixture_and_reconcile(
            uid,
            butsuri::Fixture::new(
                "main",
                butsuri::PhysShape::Aabb(butsuri::AabbShape::new(
                    Box2::new(-1.0, -1.0, 1.0, 1.0),
                    0.0,
                )),
            ),
        );
        manager.refresh_map_physics_runtime(MapId::new(73));

        assert_eq!(
            manager.entities_in_map_aabb(MapId::new(73), Box2::new(4.0, -1.0, 6.0, 1.0)),
            vec![uid]
        );
        assert_eq!(
            manager.intersect_ray(
                MapId::new(73),
                butsuri::CollisionRay::new(Vector2::new(-5.0, 0.0), Vector2::UNIT_X, -1),
                20.0,
                true,
            )[0]
            .entity,
            uid
        );
    }

    #[test]
    fn entity_manager_exposes_map_runtime_presence_and_state_helpers() {
        let mut manager = EntityManager::new();
        let _map = manager.create_entity_uninitialized_as_map(None, MapId::new(76));
        let uid = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(76),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = manager.ensure_physics(uid);
        body.can_collide = true;
        body.set_body_type(crate::BodyType::Dynamic);
        body.awake = true;
        let _ = manager.insert_fixture_and_reconcile(
            uid,
            butsuri::Fixture::new(
                "main",
                butsuri::PhysShape::Aabb(butsuri::AabbShape::new(
                    Box2::new(-0.5, -0.5, 0.5, 0.5),
                    0.0,
                )),
            ),
        );

        manager.refresh_map_physics_runtime(MapId::new(76));

        assert!(manager.has_map_broadphase(MapId::new(76)));
        assert!(manager.has_map_physics_runtime(MapId::new(76)));
        assert!(manager.map_contains_body(MapId::new(76), uid));
        assert!(manager.map_contains_awake_body(MapId::new(76), uid));
        assert_eq!(manager.map_contact_count(MapId::new(76)), 0);
        assert_eq!(manager.map_gravity(MapId::new(76)), Some(Vector2::ZERO));
        assert_eq!(manager.map_auto_clear_forces(MapId::new(76)), Some(false));
    }

    #[test]
    fn entity_manager_refresh_entities_runtime_updates_lookup_and_map_runtime() {
        let mut manager = EntityManager::new();
        let _map = manager.create_entity_uninitialized_as_map(None, MapId::new(77));

        let first = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(77),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = manager.ensure_physics(first);
        body.can_collide = true;
        body.set_body_type(crate::BodyType::Dynamic);
        body.awake = true;
        let _ = manager.insert_fixture_and_reconcile(
            first,
            butsuri::Fixture::new(
                "first",
                butsuri::PhysShape::Aabb(butsuri::AabbShape::new(
                    Box2::new(-0.5, -0.5, 0.5, 0.5),
                    0.0,
                )),
            ),
        );

        let second = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::new(0.5, 0.0),
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(77),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = manager.ensure_physics(second);
        body.can_collide = true;
        body.set_body_type(crate::BodyType::Dynamic);
        body.awake = true;
        let _ = manager.insert_fixture_and_reconcile(
            second,
            butsuri::Fixture::new(
                "second",
                butsuri::PhysShape::Aabb(butsuri::AabbShape::new(
                    Box2::new(-0.5, -0.5, 0.5, 0.5),
                    0.0,
                )),
            ),
        );

        assert_eq!(manager.refresh_entities_runtime(&[first, second]), 2);
        assert_eq!(manager.map_contact_count(MapId::new(77)), 1);
        assert_eq!(
            manager.entities_in_map_aabb(MapId::new(77), Box2::new(-1.0, -1.0, 1.0, 1.0)),
            vec![first, second]
        );
    }

    #[test]
    fn entity_manager_exposes_shared_broadphase_owner_queries() {
        let mut manager = EntityManager::new();
        let owner = manager.create_entity_uninitialized(None);
        manager.ensure_broadphase(owner);

        let uid = manager.create_entity_uninitialized(None);
        let _ = manager.apply_transform_state(
            uid,
            TransformComponentState {
                local_position: Vector2::new(5.0, 0.0),
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::NULLSPACE,
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = manager.ensure_physics(uid);
        body.can_collide = true;
        body.set_body_type(crate::BodyType::Dynamic);
        body.awake = true;
        let _ = manager.insert_fixture_and_reconcile(
            uid,
            butsuri::Fixture::new(
                "main",
                butsuri::PhysShape::Aabb(butsuri::AabbShape::new(
                    Box2::new(-1.0, -1.0, 1.0, 1.0),
                    0.0,
                )),
            ),
        );
        manager.refresh_broadphase_runtime(owner, &[uid]);

        assert_eq!(
            manager.query_aabb_entities(owner, Box2::new(4.0, -1.0, 6.0, 1.0)),
            vec![uid]
        );
        assert_eq!(
            manager.intersect_ray_on(
                owner,
                butsuri::CollisionRay::new(Vector2::new(-5.0, 0.0), Vector2::UNIT_X, -1),
                20.0,
                true,
            )[0]
            .entity,
            uid
        );
    }

    #[test]
    fn entity_manager_drains_map_physics_events_through_shared_helpers() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_as_map(None, MapId::new(74));
        let map_uid = manager.map_entity_for(MapId::new(74)).unwrap_or(map);

        manager
            .ensure_physics_map(map_uid)
            .queue_runtime_event(crate::PhysicsRuntimeEvent::Wake(
                crate::PhysicsWakeMessage {
                    body: EntityUid::new(900),
                },
            ));
        manager.ensure_physics_map(map_uid).queue_contact_event(
            butsuri::ContactStatus::StartTouching,
            butsuri::Contact::new("a", "b", butsuri::ContactType::Aabb),
        );

        assert_eq!(
            manager.drain_map_runtime_events(MapId::new(74)),
            vec![crate::PhysicsRuntimeEvent::Wake(
                crate::PhysicsWakeMessage {
                    body: EntityUid::new(900),
                }
            )]
        );
        assert_eq!(
            manager.drain_map_runtime_events(MapId::new(74)),
            Vec::<crate::PhysicsRuntimeEvent>::new()
        );

        let contact_events = manager.drain_map_contact_events(MapId::new(74));
        assert_eq!(contact_events.len(), 1);
        assert_eq!(
            contact_events[0].status,
            butsuri::ContactStatus::StartTouching
        );
        assert_eq!(contact_events[0].contact.fixture_a, "a");
        assert_eq!(manager.drain_map_contact_events(MapId::new(74)), Vec::new());
    }

    #[test]
    fn entity_manager_entity_uids_and_entities_in_map_radius_follow_shared_visibility_rules() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_as_map(None, MapId::new(75));
        let near = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::new(2.0, 0.0),
                rotation: Angle::ZERO,
                parent_id: map,
                map_id: MapId::new(75),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let far = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::new(30.0, 0.0),
                rotation: Angle::ZERO,
                parent_id: map,
                map_id: MapId::new(75),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let paused = manager.create_entity_uninitialized_with_transform(
            None,
            TransformComponentState {
                local_position: Vector2::new(1.0, 0.0),
                rotation: Angle::ZERO,
                parent_id: map,
                map_id: MapId::new(75),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(manager.set_entity_paused(paused, true));

        let default_entities = manager.entity_uids(false);
        assert!(default_entities.contains(&near));
        assert!(!default_entities.contains(&paused));

        let mut visible =
            manager.entities_in_map_radius(MapId::new(75), Vector2::ZERO, 8.0, true, true);
        visible.sort();
        assert_eq!(visible, vec![map, near, paused]);

        let mut without_paused =
            manager.entities_in_map_radius(MapId::new(75), Vector2::ZERO, 8.0, false, true);
        without_paused.sort();
        assert_eq!(without_paused, vec![map, near]);
        assert!(!without_paused.contains(&far));
    }

    #[test]
    fn entity_manager_physics_body_entities_respects_pause_filter() {
        let mut manager = EntityManager::new();
        let active = manager.create_entity_uninitialized(None);
        let paused = manager.create_entity_uninitialized(None);
        manager.ensure_physics(active);
        manager.ensure_physics(paused);
        assert!(manager.set_entity_paused(paused, true));

        assert_eq!(manager.physics_body_entities(false), vec![active]);

        let mut include_paused = manager.physics_body_entities(true);
        include_paused.sort();
        assert_eq!(include_paused, vec![active, paused]);
    }

    #[test]
    fn entity_manager_registers_builtin_event_driven_systems() {
        let manager = EntityManager::new();
        let systems = manager.entity_system_names();

        assert!(systems.is_empty());
        assert_eq!(manager.entity_system_count(), 0);
    }

    #[test]
    fn entity_manager_entity_system_manager_stays_empty_without_builtin_runtime_systems() {
        let mut manager = EntityManager::new();
        assert!(!manager.entity_systems_initialized());

        manager.initialize_entity_systems();

        assert!(manager.entity_systems_initialized());
        assert!(
            !manager
                .entity_system_names()
                .contains(&"shared_physics".to_string())
        );
        assert!(
            !manager
                .entity_system_names()
                .contains(&"entity_lookup".to_string())
        );
        assert!(
            !manager
                .entity_system_names()
                .contains(&"collision_wake".to_string())
        );
        assert!(
            !manager
                .entity_system_names()
                .contains(&"collide_on_anchor".to_string())
        );
        assert!(
            !manager
                .entity_system_names()
                .contains(&"transform".to_string())
        );
        assert!(
            !manager
                .entity_system_names()
                .contains(&"SharedAppearanceSystem".to_string())
        );

        manager.shutdown_entity_systems();
        assert!(!manager.entity_systems_initialized());
    }

    #[test]
    fn entity_manager_routes_pause_events_into_canonical_runtime_queue() {
        let mut manager = EntityManager::new();
        let metadata_uid = manager.create_entity_uninitialized(None);
        manager.drain_entity_runtime_events();
        let event = crate::EntityPausedEvent {
            entity: metadata_uid,
            paused: true,
        };
        manager.queue_entity_event(event);
        assert_eq!(
            manager.drain_entity_runtime_events(),
            vec![crate::EntityRuntimeEvent::EntityPaused(
                crate::EntityPausedEvent {
                    entity: metadata_uid,
                    paused: true,
                }
            )]
        );
    }

    #[test]
    fn entity_manager_processes_collision_wake_side_effects_immediately() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        let _ = manager.apply_transform_state(
            uid,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::new(99),
                map_id: MapId::new(1),
                grid_id: GridId::new(5),
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = manager.ensure_physics(uid);
        body.can_collide = true;
        body.awake = false;
        manager.ensure_collision_wake(uid);

        let _ = manager.queue_entity_runtime_event(
            uid,
            crate::PhysicsRuntimeEvent::Sleep(crate::PhysicsSleepMessage { body: uid }),
        );

        assert!(!manager.physics.get(&uid).unwrap().can_collide);

        assert!(manager.remove_component_by_net_id(uid, EntityManager::COLLISION_WAKE_NET_ID));
        assert!(manager.physics.get(&uid).unwrap().can_collide);
    }

    #[test]
    fn entity_manager_processes_shared_physics_map_init_side_effects_immediately() {
        let mut manager = EntityManager::new();
        let map_uid = manager.create_entity_uninitialized(None);
        manager.initialize_entity(map_uid);
        let _ = manager.apply_transform_state(
            map_uid,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(44),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        manager.ensure_map(MapId::new(44), map_uid);

        assert!(!manager.broadphases.contains_key(&map_uid));
        manager.ensure_physics_map(map_uid);
        assert!(manager.broadphases.contains_key(&map_uid));
        assert!(manager.physics_maps.contains_key(&map_uid));
    }

    #[test]
    fn entity_manager_processes_collide_on_anchor_startup_side_effects_immediately() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        let _ = manager.apply_transform_state(
            uid,
            TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(3),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: true,
            },
        );
        let body = manager.ensure_physics(uid);
        body.can_collide = false;
        manager.ensure_collide_on_anchor(uid).enable = true;

        manager.initialize_entity(uid);

        assert!(manager.physics.get(&uid).unwrap().can_collide);
    }

    #[test]
    fn entity_manager_processes_lookup_side_effects_immediately() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized_as_map(None, MapId::new(44));
        manager.ensure_lookup(map);

        let uid = manager.create_entity_uninitialized_at_map(
            None,
            MapCoordinates::new(Vector2::ZERO, MapId::new(44)),
        );
        let mut move_event = manager
            .set_local_transform(uid, Vector2::new(2.0, 0.0), Angle::ZERO)
            .unwrap();
        manager.raise_component_event(uid, "TransformComponent", &mut move_event);

        assert!(
            manager
                .entity_lookups
                .get(&map)
                .unwrap()
                .entities
                .contains_key(&uid)
        );

        assert!(manager.delete_entity(uid).is_some());
        assert!(
            !manager
                .entity_lookups
                .get(&map)
                .unwrap()
                .entities
                .contains_key(&uid)
        );
    }
}
