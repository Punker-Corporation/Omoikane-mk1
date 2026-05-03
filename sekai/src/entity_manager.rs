use crate::{
    AppearanceComponent, BroadphaseComponent, Component, ComponentFactory, EntityDeletedMessage,
    EntityInitializedMessage, EntityLookupComponent, EntitySystemManager, EntityTerminatingEvent,
    EntityUid, FixturesComponent, JointComponent, MapComponent, MapCoordinates, MapGrid,
    MapGridComponent, MapId, MetaDataComponent, PhysicsComponent, SharedPhysicsMapComponent,
    TimerComponent, TransformComponent, TransformComponentState, TransformResolver, WorldTransform,
};
use jikan::GameTick;
use keisan::Vector2;
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
    pub component_factory: ComponentFactory,
    pub entity_sys_manager: EntitySystemManager,
    pub queued_deletions: VecDeque<EntityUid>,
    pub queued_deletions_set: HashSet<EntityUid>,
    pub entities: HashSet<EntityUid>,
    pub next_entity_uid: i32,
    pub metadata: HashMap<EntityUid, MetaDataComponent>,
    pub transforms: HashMap<EntityUid, TransformComponent>,
    pub map_components: HashMap<EntityUid, MapComponent>,
    pub appearances: HashMap<EntityUid, AppearanceComponent>,
    pub entity_lookups: HashMap<EntityUid, EntityLookupComponent>,
    pub joint_components: HashMap<EntityUid, JointComponent>,
    pub timers: HashMap<EntityUid, TimerComponent>,
    pub physics: HashMap<EntityUid, PhysicsComponent>,
    pub fixtures: HashMap<EntityUid, FixturesComponent>,
    pub broadphases: HashMap<EntityUid, BroadphaseComponent>,
    pub physics_maps: HashMap<EntityUid, SharedPhysicsMapComponent>,
    pub map_grids: HashMap<EntityUid, MapGrid>,
    pub map_grid_components: HashMap<EntityUid, MapGridComponent>,
    pub components: HashMap<EntityUid, Vec<Component>>,
}

impl Default for EntityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityManager {
    pub fn new() -> Self {
        Self {
            current_tick: GameTick::ZERO,
            component_factory: ComponentFactory::new(),
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
            timers: HashMap::new(),
            physics: HashMap::new(),
            fixtures: HashMap::new(),
            broadphases: HashMap::new(),
            physics_maps: HashMap::new(),
            map_grids: HashMap::new(),
            map_grid_components: HashMap::new(),
            components: HashMap::new(),
        }
    }

    pub fn create_entity_uninitialized(&mut self, prototype_name: Option<&str>) -> EntityUid {
        let uid = self.generate_entity_uid();
        self.alloc_entity(uid, prototype_name);
        uid
    }

    pub fn create_entity_uninitialized_at_map(
        &mut self,
        prototype_name: Option<&str>,
        coordinates: MapCoordinates,
    ) -> EntityUid {
        let uid = self.create_entity_uninitialized(prototype_name);
        if let Some(meta) = self.metadata.get_mut(&uid) {
            meta.map_id = coordinates.map_id;
        }
        if let Some(xform) = self.transforms.get_mut(&uid) {
            xform.local_position = coordinates.position;
        }
        uid
    }

    pub fn initialize_entity(&mut self, entity: EntityUid) -> EntityInitializedMessage {
        if let Some(meta) = self.metadata.get_mut(&entity) {
            meta.entity_life_stage = crate::EntityLifeStage::Initialized;
            meta.dirty(self.current_tick);
        }
        EntityInitializedMessage { entity }
    }

    pub fn dirty_entity(&mut self, uid: EntityUid) {
        let Some(metadata) = self.metadata.get_mut(&uid) else {
            panic!("Entity {uid} does not exist, cannot dirty it.");
        };
        if metadata.entity_last_modified_tick != self.current_tick {
            metadata.entity_last_modified_tick = self.current_tick;
        }
    }

    pub fn dirty_component(&mut self, component: &mut Component) {
        let owner = component.owner;
        if !owner.is_valid() || component.deleted() || !component.net_sync_enabled {
            return;
        }
        self.dirty_entity(owner);
        component.last_modified_tick = self.current_tick;
    }

    pub fn stamp_component_created(&self, component: &mut Component) {
        component.creation_tick = self.current_tick;
        component.last_modified_tick = self.current_tick;
    }

    pub fn delete_entity(
        &mut self,
        uid: EntityUid,
    ) -> Option<(EntityTerminatingEvent, EntityDeletedMessage)> {
        let meta = self.metadata.get(&uid)?;
        if meta.entity_deleted() {
            return None;
        }

        crate::EntityLookupSystem.remove_from_entity_tree(self, uid, false);

        let parent = self.transforms.get(&uid).map(|transform| transform.parent);
        let child_world_states = self
            .transforms
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
            .unwrap_or_default();

        if let Some(parent) = parent.filter(|parent| parent.is_valid())
            && let Some(parent_transform) = self.transforms.get_mut(&parent)
        {
            parent_transform.children.remove(&uid);
        }

        for (child, map_id, grid_id, world_position, world_rotation) in child_world_states {
            if let Some(transform) = self.transforms.get_mut(&child) {
                transform.parent = EntityUid::INVALID;
                transform.map_id = map_id;
                transform.grid_id = grid_id;
                transform.local_position = world_position;
                transform.local_rotation = world_rotation;
                transform.rebuild_for_manager();
            }
        }

        let terminate = EntityTerminatingEvent { entity: uid };
        if let Some(meta) = self.metadata.get_mut(&uid) {
            meta.entity_life_stage = crate::EntityLifeStage::Deleted;
        }
        self.metadata.remove(&uid);
        self.transforms.remove(&uid);
        self.map_components.remove(&uid);
        self.appearances.remove(&uid);
        self.entity_lookups.remove(&uid);
        self.joint_components.remove(&uid);
        self.timers.remove(&uid);
        self.physics.remove(&uid);
        self.fixtures.remove(&uid);
        self.broadphases.remove(&uid);
        self.physics_maps.remove(&uid);
        self.map_grids.remove(&uid);
        self.map_grid_components.remove(&uid);
        self.components.remove(&uid);
        self.entities.remove(&uid);
        Some((terminate, EntityDeletedMessage { entity: uid }))
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

    pub fn deleted(&self, uid: EntityUid) -> bool {
        self.metadata
            .get(&uid)
            .map(|m| m.entity_deleted())
            .unwrap_or(true)
    }

    pub fn ensure_appearance(&mut self, uid: EntityUid) -> &mut AppearanceComponent {
        self.appearances.entry(uid).or_insert_with(|| {
            let mut component = AppearanceComponent::new();
            component.base.owner = uid;
            component.base.creation_tick = self.current_tick;
            component.base.last_modified_tick = self.current_tick;
            component
        })
    }

    pub fn ensure_lookup(&mut self, uid: EntityUid) -> &mut EntityLookupComponent {
        self.entity_lookups.entry(uid).or_insert_with(|| {
            let mut component = EntityLookupComponent::new();
            component.base.owner = uid;
            component.base.creation_tick = self.current_tick;
            component.base.last_modified_tick = self.current_tick;
            component
        })
    }

    pub fn ensure_joints(&mut self, uid: EntityUid) -> &mut JointComponent {
        self.joint_components.entry(uid).or_insert_with(|| {
            let mut component = JointComponent::new();
            component.base.owner = uid;
            component.base.creation_tick = self.current_tick;
            component.base.last_modified_tick = self.current_tick;
            component
        })
    }

    pub fn ensure_map(&mut self, map_id: MapId, uid: EntityUid) -> &mut MapComponent {
        self.map_components.entry(uid).or_insert_with(|| {
            let mut component = MapComponent::new();
            component.base.owner = uid;
            component.base.creation_tick = self.current_tick;
            component.base.last_modified_tick = self.current_tick;
            component.world_map = map_id;
            component
        })
    }

    pub fn ensure_timer(&mut self, uid: EntityUid) -> &mut TimerComponent {
        self.timers.entry(uid).or_insert_with(|| {
            let mut component = TimerComponent::new();
            component.base.owner = uid;
            component.base.creation_tick = self.current_tick;
            component.base.last_modified_tick = self.current_tick;
            component
        })
    }

    pub fn ensure_physics(&mut self, uid: EntityUid) -> &mut PhysicsComponent {
        self.physics.entry(uid).or_insert_with(|| {
            let mut component = PhysicsComponent::new();
            component.base.owner = uid;
            component.base.creation_tick = self.current_tick;
            component.base.last_modified_tick = self.current_tick;
            component
        })
    }

    pub fn ensure_fixtures(&mut self, uid: EntityUid) -> &mut FixturesComponent {
        self.fixtures.entry(uid).or_insert_with(|| {
            let mut component = FixturesComponent::new();
            component.base.owner = uid;
            component.base.creation_tick = self.current_tick;
            component.base.last_modified_tick = self.current_tick;
            component
        })
    }

    pub fn ensure_broadphase(&mut self, uid: EntityUid) -> &mut BroadphaseComponent {
        self.broadphases.entry(uid).or_insert_with(|| {
            let mut component = BroadphaseComponent::new();
            component.base.owner = uid;
            component.base.creation_tick = self.current_tick;
            component.base.last_modified_tick = self.current_tick;
            component
        })
    }

    pub fn ensure_physics_map(&mut self, uid: EntityUid) -> &mut SharedPhysicsMapComponent {
        self.physics_maps.entry(uid).or_insert_with(|| {
            let mut component = SharedPhysicsMapComponent::new();
            component.base.owner = uid;
            component.base.creation_tick = self.current_tick;
            component.base.last_modified_tick = self.current_tick;
            component
        })
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

        crate::EntityLookupSystem.remove_from_entity_tree(self, uid, false);

        let parent = self.transforms.get(&uid).map(|transform| transform.parent);
        let child_world_states = self
            .transforms
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
            .unwrap_or_default();

        if let Some(parent) = parent.filter(|parent| parent.is_valid())
            && let Some(parent_transform) = self.transforms.get_mut(&parent)
        {
            parent_transform.children.remove(&uid);
        }

        for (child, map_id, grid_id, world_position, world_rotation) in child_world_states {
            if let Some(transform) = self.transforms.get_mut(&child) {
                transform.parent = EntityUid::INVALID;
                transform.map_id = map_id;
                transform.grid_id = grid_id;
                transform.local_position = world_position;
                transform.local_rotation = world_rotation;
                transform.rebuild_for_manager();
            }
        }

        self.transforms.remove(&uid).is_some()
    }

    pub fn remove_map_grid_component(&mut self, uid: EntityUid) -> bool {
        let removed_component = self.map_grid_components.remove(&uid).is_some();
        let removed_grid = self.map_grids.remove(&uid).is_some();
        if removed_component || removed_grid {
            crate::EntityLookupSystem.remove_from_entity_tree(self, uid, true);
            if let Some(transform) = self.transforms.get_mut(&uid) {
                transform.grid_id = crate::GridId::INVALID;
            }
            return true;
        }
        false
    }

    pub fn remove_map_component(&mut self, uid: EntityUid) -> bool {
        let Some(map_id) = self
            .map_components
            .get(&uid)
            .map(|component| component.world_map)
        else {
            return false;
        };

        let children = self
            .transforms
            .iter()
            .filter_map(|(child, transform)| (transform.parent == uid).then_some(*child))
            .collect::<Vec<_>>();
        for child in children {
            let world = self.world_transform(child);
            if let Some(transform) = self.transforms.get_mut(&child) {
                transform.parent = EntityUid::INVALID;
                transform.map_id = MapId::NULLSPACE;
                transform.grid_id = crate::GridId::INVALID;
                if let Some(world) = world {
                    transform.local_position = world.world_position;
                    transform.local_rotation = world.world_rotation;
                }
                transform.rebuild_for_manager();
            }
            if let Some(grid) = self.map_grids.get_mut(&child) {
                grid.parent_map_id = MapId::NULLSPACE;
            }
        }

        if let Some(transform) = self.transforms.get_mut(&uid) {
            transform.map_id = MapId::NULLSPACE;
        }
        self.broadphases.remove(&uid);
        self.physics_maps.remove(&uid);
        self.map_components.remove(&uid).is_some() || map_id == MapId::NULLSPACE
    }

    pub fn set_parent(&mut self, uid: EntityUid, parent: EntityUid) -> bool {
        let Some(old_parent) = self.transforms.get(&uid).map(|transform| transform.parent) else {
            return false;
        };

        if old_parent == parent {
            return true;
        }

        if old_parent.is_valid()
            && let Some(transform) = self.transforms.get_mut(&old_parent)
        {
            transform.children.remove(&uid);
        }

        if let Some(transform) = self.transforms.get_mut(&uid) {
            transform.parent = parent;
            transform.rebuild_for_manager();
        }

        if parent.is_valid()
            && let Some(transform) = self.transforms.get_mut(&parent)
        {
            transform.children.insert(uid);
        }

        true
    }

    pub fn apply_transform_state(
        &mut self,
        uid: EntityUid,
        state: TransformComponentState,
    ) -> bool {
        let Some(old_parent) = self.transforms.get(&uid).map(|transform| transform.parent) else {
            return false;
        };

        if old_parent != state.parent_id {
            if old_parent.is_valid()
                && let Some(transform) = self.transforms.get_mut(&old_parent)
            {
                transform.children.remove(&uid);
            }
            if state.parent_id.is_valid()
                && let Some(transform) = self.transforms.get_mut(&state.parent_id)
            {
                transform.children.insert(uid);
            }
        }

        if let Some(transform) = self.transforms.get_mut(&uid) {
            transform.handle_transform_state(state);
            return true;
        }

        false
    }

    fn alloc_entity(&mut self, uid: EntityUid, prototype_name: Option<&str>) {
        self.entities.insert(uid);
        let mut meta = MetaDataComponent::new();
        meta.base.owner = uid;
        meta.base.creation_tick = self.current_tick;
        meta.base.last_modified_tick = self.current_tick;
        meta.prototype_id = prototype_name.map(str::to_string);
        self.metadata.insert(uid, meta);

        let mut xform = TransformComponent::new();
        xform.base.owner = uid;
        xform.base.creation_tick = self.current_tick;
        xform.base.last_modified_tick = self.current_tick;
        xform.map_id = MapId::NULLSPACE;
        self.transforms.insert(uid, xform);
        self.components.insert(uid, Vec::new());
    }

    pub fn alloc_entity_external(&mut self, uid: EntityUid, prototype_name: Option<&str>) {
        self.alloc_entity(uid, prototype_name);
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
    use crate::{GridId, MapId};
    use crate::{MapCoordinates, Tile, TileRenderFlag, TransformComponentState};
    use keisan::{Angle, Box2, Vector2, Vector2i};

    #[test]
    fn entity_manager_allocates_initializes_and_deletes() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized_at_map(
            Some("test"),
            MapCoordinates::new(Vector2::new(1.0, 2.0), MapId::new(7)),
        );
        assert!(manager.entity_exists(uid));
        let init = manager.initialize_entity(uid);
        assert_eq!(init.entity, uid);
        manager.queue_delete_entity(uid);
        manager.flush_queued_deletions();
        assert!(!manager.entity_exists(uid));
    }

    #[test]
    fn entity_manager_ensures_typed_components() {
        let mut manager = EntityManager::new();
        let uid = manager.create_entity_uninitialized(None);
        manager.ensure_appearance(uid);
        manager.ensure_lookup(uid);
        manager.ensure_joints(uid);
        manager.ensure_map(MapId::new(3), uid);
        manager.ensure_timer(uid);
        manager.ensure_physics(uid);
        manager.ensure_fixtures(uid);
        manager.ensure_broadphase(uid);
        manager.ensure_physics_map(uid);
        assert!(manager.appearances.contains_key(&uid));
        assert!(manager.entity_lookups.contains_key(&uid));
        assert!(manager.joint_components.contains_key(&uid));
        assert!(manager.map_components.contains_key(&uid));
        assert!(manager.physics.contains_key(&uid));
        assert!(manager.physics_maps.contains_key(&uid));
    }

    #[test]
    fn entity_manager_tracks_transform_parent_links_and_preserves_child_world_space_on_delete() {
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

        let child_transform = manager.transforms.get(&child).unwrap();
        assert_eq!(child_transform.parent, crate::EntityUid::INVALID);
        assert_eq!(child_transform.local_position, Vector2::new(6.0, 2.0));
    }

    #[test]
    fn entity_manager_delete_cleans_spatial_runtime_references() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized(None);
        manager.ensure_map(MapId::new(1), map);
        manager.transforms.get_mut(&map).unwrap().map_id = MapId::new(1);

        let grid_uid = manager.create_entity_uninitialized(None);
        let grid = manager.map_grid_components.entry(grid_uid).or_default();
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
        crate::EntityLookupSystem.update_bounds(&mut manager, moving);

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
        crate::EntityLookupSystem.update_bounds(&mut manager, anchored);

        assert_eq!(
            crate::EntityLookupSystem.get_entities_intersecting(
                &manager,
                GridId::new(3),
                Vector2i::new(0, 0)
            ),
            vec![moving, anchored]
        );
        assert_eq!(
            crate::EntityLookupSystem.get_entities_intersecting_world_aabb(
                &manager,
                grid_uid,
                Box2::new(-1.0, -1.0, 1.0, 1.0),
                true,
            ),
            vec![moving, anchored]
        );

        manager.queue_delete_entity(moving);
        manager.flush_queued_deletions();
        assert_eq!(
            crate::EntityLookupSystem.get_entities_intersecting(
                &manager,
                GridId::new(3),
                Vector2i::new(0, 0)
            ),
            vec![anchored]
        );

        manager.queue_delete_entity(anchored);
        manager.flush_queued_deletions();
        assert!(
            crate::EntityLookupSystem
                .get_entities_intersecting(&manager, GridId::new(3), Vector2i::new(0, 0))
                .is_empty()
        );
    }
}
