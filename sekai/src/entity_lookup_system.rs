use crate::transform_component::TransformResolver;
use crate::{EntityLookupComponent, EntityManager, EntityUid, GridId, TileRef, TransformComponent};
use keisan::{Angle, Box2, Vector2, Vector2i};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LookupFlags(pub u8);

impl LookupFlags {
    pub const NONE: Self = Self(0);
    pub const APPROXIMATE: Self = Self(1 << 0);
    pub const INCLUDE_ANCHORED: Self = Self(1 << 1);

    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct EntityLookupSystem;

impl EntityLookupSystem {
    fn remove_from_snap_grids(&self, manager: &mut EntityManager, entity: EntityUid) {
        let grid_owners = manager.map_grids.keys().copied().collect::<Vec<_>>();
        for owner in grid_owners {
            let (chunk_indices, chunk_size) = manager
                .map_grids
                .get(&owner)
                .map(|grid| (grid.chunk_indices(), grid.chunk_size))
                .unwrap_or_default();
            if let Some(grid) = manager.map_grids.get_mut(&owner) {
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

    fn get_anchor_registration(
        &self,
        manager: &EntityManager,
        entity: EntityUid,
        xform: &TransformComponent,
    ) -> Option<(EntityUid, Vector2i)> {
        if !xform.anchored {
            return None;
        }

        let grid_uid = if manager.map_grids.contains_key(&xform.parent) {
            xform.parent
        } else if xform.grid_id.is_valid() {
            manager
                .map_grids
                .iter()
                .find_map(|(uid, grid)| (grid.index == xform.grid_id).then_some(*uid))?
        } else {
            return None;
        };

        let world = manager.world_transform(entity)?;
        let grid = manager.map_grids.get(&grid_uid)?;
        Some((grid_uid, grid.world_to_tile(world.world_position)))
    }

    fn touch_lookup_component(&self, manager: &mut EntityManager, owner: EntityUid) {
        let tick = manager.current_tick;
        if let Some(lookup) = manager.entity_lookups.get_mut(&owner) {
            lookup.base.last_modified_tick = tick;
        }
        if manager.entity_exists(owner) {
            manager.dirty_entity(owner);
        }
    }

    pub fn ensure_lookup<'a>(
        &self,
        manager: &'a mut EntityManager,
        owner: EntityUid,
    ) -> &'a mut EntityLookupComponent {
        manager.entity_lookups.entry(owner).or_insert_with(|| {
            let mut lookup = EntityLookupComponent::new();
            lookup.base.owner = owner;
            lookup
        })
    }

    pub fn get_local_bounds(&self, grid_indices: Vector2i, tile_size: u16) -> Box2 {
        Box2::new(
            grid_indices.x as f32 * tile_size as f32,
            grid_indices.y as f32 * tile_size as f32,
            (grid_indices.x + 1) as f32 * tile_size as f32,
            (grid_indices.y + 1) as f32 * tile_size as f32,
        )
    }

    pub fn get_world_aabb(&self, manager: &EntityManager, entity: EntityUid) -> Option<Box2> {
        if let Some(body_bounds) = manager
            .physics
            .contains_key(&entity)
            .then(|| crate::SharedPhysicsSystem.get_world_aabb(manager, entity))
            .flatten()
        {
            return Some(body_bounds);
        }

        let xform = manager.transforms.get(&entity)?;
        let world = manager.world_transform(entity)?;
        let point = world.world_position;
        let radius = if xform.anchored { 0.0 } else { 0.5 };
        Some(Box2::new(
            point.x - radius,
            point.y - radius,
            point.x + radius,
            point.y + radius,
        ))
    }

    pub fn update_bounds(&self, manager: &mut EntityManager, entity: EntityUid) -> bool {
        let Some(xform) = manager.transforms.get(&entity).cloned() else {
            return false;
        };
        self.remove_from_entity_tree(manager, entity, false);
        if xform.anchored {
            let Some((grid_uid, tile)) = self.get_anchor_registration(manager, entity, &xform)
            else {
                return false;
            };
            return manager
                .map_grids
                .get_mut(&grid_uid)
                .map(|grid| grid.add_to_snap_grid_cell(tile, entity))
                .unwrap_or(false);
        }
        let Some(lookup_owner) = self.get_lookup_owner(manager, entity, &xform) else {
            return false;
        };
        let Some(aabb) = self.get_world_aabb(manager, entity) else {
            return false;
        };
        let changed = {
            let lookup = self.ensure_lookup(manager, lookup_owner);
            if lookup.entities.get(&entity).copied() == Some(aabb) {
                false
            } else {
                lookup.add_or_update(entity, aabb);
                true
            }
        };
        if changed {
            self.touch_lookup_component(manager, lookup_owner);
        }
        true
    }

    pub fn remove_from_entity_tree(
        &self,
        manager: &mut EntityManager,
        entity: EntityUid,
        recursive: bool,
    ) {
        self.remove_from_snap_grids(manager, entity);

        let owners = manager.entity_lookups.keys().copied().collect::<Vec<_>>();
        let mut touched = Vec::new();
        for owner in owners {
            let removed = manager
                .entity_lookups
                .get_mut(&owner)
                .map(|lookup| lookup.entities.remove(&entity).is_some())
                .unwrap_or(false);
            if removed {
                touched.push(owner);
            }
        }
        for owner in touched {
            self.touch_lookup_component(manager, owner);
        }

        if recursive {
            let children = manager
                .transforms
                .iter()
                .filter_map(|(uid, transform)| (transform.parent == entity).then_some(*uid))
                .collect::<Vec<_>>();
            for child in children {
                self.remove_from_entity_tree(manager, child, true);
            }
        }
    }

    pub fn update_subtree_bounds(&self, manager: &mut EntityManager, root: EntityUid) {
        let mut stack = vec![root];
        while let Some(uid) = stack.pop() {
            let _ = self.update_bounds(manager, uid);
            let children = manager
                .transforms
                .iter()
                .filter_map(|(child, transform)| (transform.parent == uid).then_some(*child))
                .collect::<Vec<_>>();
            stack.extend(children);
        }
    }

    pub fn get_entities_intersecting(
        &self,
        manager: &EntityManager,
        grid_id: GridId,
        grid_indices: Vector2i,
    ) -> Vec<EntityUid> {
        let Some(grid) = manager
            .map_grids
            .values()
            .find(|grid| grid.index == grid_id)
        else {
            return Vec::new();
        };

        let lookup = manager.entity_lookups.get(&grid.grid_entity_id);
        let local = self.get_local_bounds(grid_indices, grid.tile_size);
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

        results.extend(grid.get_anchored_entities(grid_indices));
        results.sort();
        results.dedup();
        results
    }

    pub fn get_entities_intersecting_many<I>(
        &self,
        manager: &EntityManager,
        grid_id: GridId,
        grid_indices: I,
    ) -> Vec<EntityUid>
    where
        I: IntoIterator<Item = Vector2i>,
    {
        let mut results = Vec::new();
        for tile in grid_indices {
            results.extend(self.get_entities_intersecting(manager, grid_id, tile));
        }
        results.sort();
        results.dedup();
        results
    }

    pub fn get_entities_intersecting_world_aabb(
        &self,
        manager: &EntityManager,
        lookup_owner: EntityUid,
        world_aabb: Box2,
        include_anchored: bool,
    ) -> Vec<EntityUid> {
        let mut results = manager
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

        if include_anchored && let Some(grid) = manager.map_grids.get(&lookup_owner) {
            let tile_hits = grid
                .get_map_chunks_intersecting(world_aabb)
                .flat_map(|chunk_index| {
                    let Some(chunk) = grid.try_get_chunk(chunk_index) else {
                        return Vec::new().into_iter();
                    };
                    let mut anchored = Vec::new();
                    for x in 0..chunk.chunk_size() {
                        for y in 0..chunk.chunk_size() {
                            let grid_tile =
                                chunk.chunk_tile_to_grid_tile(Vector2i::new(x as i32, y as i32));
                            let bounds = self.get_local_bounds(grid_tile, grid.tile_size);
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

        results.sort();
        results.dedup();
        results
    }

    pub fn get_world_bounds(
        &self,
        manager: &EntityManager,
        tile_ref: TileRef,
        _angle: Option<Angle>,
    ) -> Option<Box2> {
        let grid = manager
            .map_grids
            .values()
            .find(|grid| grid.index == tile_ref.grid_index)?;
        let center = grid.grid_tile_to_world_pos(tile_ref.grid_indices);
        let size = Vector2::new(grid.tile_size as f32, grid.tile_size as f32);
        Some(Box2::centered_around(center, size))
    }

    fn get_lookup_owner(
        &self,
        manager: &EntityManager,
        entity: EntityUid,
        xform: &TransformComponent,
    ) -> Option<EntityUid> {
        if manager.map_components.contains_key(&entity) || manager.map_grids.contains_key(&entity) {
            return None;
        }

        if xform.parent.is_valid() {
            return Some(xform.parent);
        }

        let mover = crate::SharedTransformSystem::new().get_mover_coordinates(manager, xform);
        Some(mover.entity_id)
    }
}

#[cfg(test)]
mod tests {
    use super::EntityLookupSystem;
    use crate::{EntityManager, GridId, MapId, Tile, TileRenderFlag};
    use keisan::{Box2, Vector2, Vector2i};

    #[test]
    fn lookup_system_can_query_entities_intersecting_grid_tiles() {
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

        manager
            .map_grids
            .get_mut(&grid_uid)
            .unwrap()
            .set_tile(Vector2i::new(0, 0), Tile::new(1, TileRenderFlag(0), 0));

        let entity = manager.create_entity_uninitialized(None);
        manager.transforms.get_mut(&entity).unwrap().base.owner = entity;
        manager.transforms.get_mut(&entity).unwrap().parent = grid_uid;
        manager.transforms.get_mut(&entity).unwrap().local_position =
            keisan::Vector2::new(0.5, 0.5);
        manager
            .transforms
            .get_mut(&entity)
            .unwrap()
            .rebuild_for_manager();

        let system = EntityLookupSystem;
        system.ensure_lookup(&mut manager, grid_uid);
        system.update_bounds(&mut manager, entity);
        let hits = system.get_entities_intersecting(&manager, GridId::new(3), Vector2i::new(0, 0));
        assert!(hits.contains(&entity));
    }

    #[test]
    fn lookup_system_can_query_entities_by_world_aabb_for_owner() {
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
        manager
            .map_grids
            .get_mut(&grid_uid)
            .unwrap()
            .set_tile(Vector2i::new(0, 0), Tile::new(1, TileRenderFlag(0), 0));

        let entity = manager.create_entity_uninitialized(None);
        manager.transforms.get_mut(&entity).unwrap().base.owner = entity;
        manager.transforms.get_mut(&entity).unwrap().parent = grid_uid;
        manager.transforms.get_mut(&entity).unwrap().local_position =
            keisan::Vector2::new(0.5, 0.5);
        manager
            .transforms
            .get_mut(&entity)
            .unwrap()
            .rebuild_for_manager();

        let system = EntityLookupSystem;
        system.ensure_lookup(&mut manager, grid_uid);
        system.update_bounds(&mut manager, entity);
        let hits = system.get_entities_intersecting_world_aabb(
            &manager,
            grid_uid,
            Box2::new(-1.0, -1.0, 1.0, 1.0),
            false,
        );
        assert_eq!(hits, vec![entity]);
    }

    #[test]
    fn lookup_system_removes_stale_owner_entries_and_refreshes_subtrees() {
        let mut manager = EntityManager::new();
        let map = manager.create_entity_uninitialized(None);
        manager.ensure_map(MapId::new(1), map);
        manager.transforms.get_mut(&map).unwrap().map_id = MapId::new(1);

        let first_grid = manager.create_entity_uninitialized(None);
        let first_grid_component = manager.map_grid_components.entry(first_grid).or_default();
        first_grid_component.base.owner = first_grid;
        first_grid_component.grid_index = GridId::new(3);
        let first_map_grid = first_grid_component
            .alloc_map_grid(first_grid, MapId::new(1), 1)
            .clone();
        manager.map_grids.insert(first_grid, first_map_grid);
        manager.set_parent(first_grid, map);

        let second_grid = manager.create_entity_uninitialized(None);
        let second_grid_component = manager.map_grid_components.entry(second_grid).or_default();
        second_grid_component.base.owner = second_grid;
        second_grid_component.grid_index = GridId::new(4);
        let second_map_grid = second_grid_component
            .alloc_map_grid(second_grid, MapId::new(1), 1)
            .clone();
        manager.map_grids.insert(second_grid, second_map_grid);
        manager.set_parent(second_grid, map);

        let entity = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            entity,
            crate::TransformComponentState {
                local_position: keisan::Vector2::new(0.5, 0.5),
                rotation: keisan::Angle::ZERO,
                parent_id: first_grid,
                map_id: MapId::new(1),
                grid_id: GridId::new(3),
                no_local_rotation: false,
                anchored: false,
            },
        );

        let system = EntityLookupSystem;
        system.ensure_lookup(&mut manager, first_grid);
        system.ensure_lookup(&mut manager, second_grid);
        assert!(system.update_bounds(&mut manager, entity));
        assert!(
            manager
                .entity_lookups
                .get(&first_grid)
                .unwrap()
                .entities
                .contains_key(&entity)
        );

        manager.apply_transform_state(
            entity,
            crate::TransformComponentState {
                local_position: keisan::Vector2::new(0.5, 0.5),
                rotation: keisan::Angle::ZERO,
                parent_id: second_grid,
                map_id: MapId::new(1),
                grid_id: GridId::new(4),
                no_local_rotation: false,
                anchored: false,
            },
        );
        assert!(system.update_bounds(&mut manager, entity));
        assert!(
            !manager
                .entity_lookups
                .get(&first_grid)
                .unwrap()
                .entities
                .contains_key(&entity)
        );
        assert!(
            manager
                .entity_lookups
                .get(&second_grid)
                .unwrap()
                .entities
                .contains_key(&entity)
        );

        manager
            .transforms
            .get_mut(&second_grid)
            .unwrap()
            .local_position = keisan::Vector2::new(10.0, 0.0);
        manager
            .transforms
            .get_mut(&second_grid)
            .unwrap()
            .rebuild_for_manager();
        system.update_subtree_bounds(&mut manager, second_grid);
        let moved_bounds = manager
            .entity_lookups
            .get(&second_grid)
            .unwrap()
            .entities
            .get(&entity)
            .copied()
            .unwrap();
        assert!(moved_bounds.contains(keisan::Vector2::new(10.5, 0.5), true));
    }

    #[test]
    fn lookup_system_registers_anchored_entities_in_snap_grid_queries() {
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

        let entity = manager.create_entity_uninitialized(None);
        manager.apply_transform_state(
            entity,
            crate::TransformComponentState {
                local_position: Vector2::new(0.5, 0.5),
                rotation: keisan::Angle::ZERO,
                parent_id: grid_uid,
                map_id: MapId::new(1),
                grid_id: GridId::new(3),
                no_local_rotation: false,
                anchored: true,
            },
        );

        let system = EntityLookupSystem;
        assert!(system.update_bounds(&mut manager, entity));
        assert_eq!(
            system.get_entities_intersecting(&manager, GridId::new(3), Vector2i::new(0, 0)),
            vec![entity]
        );
        assert_eq!(
            system.get_entities_intersecting_world_aabb(
                &manager,
                grid_uid,
                Box2::new(-0.1, -0.1, 1.1, 1.1),
                true,
            ),
            vec![entity]
        );

        manager.transforms.get_mut(&entity).unwrap().anchored = false;
        manager.transforms.get_mut(&entity).unwrap().local_position = Vector2::new(2.5, 0.5);
        manager
            .transforms
            .get_mut(&entity)
            .unwrap()
            .rebuild_for_manager();
        assert!(system.update_bounds(&mut manager, entity));
        assert!(
            system
                .get_entities_intersecting(&manager, GridId::new(3), Vector2i::new(0, 0))
                .is_empty()
        );
    }
}
