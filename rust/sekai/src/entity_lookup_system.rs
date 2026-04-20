use crate::{EntityLookupComponent, EntityManager, EntityUid, GridId, TileRef, TransformComponent};
use crate::transform_component::TransformResolver;
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
        if xform.anchored {
            self.remove_from_entity_tree(manager, entity, false);
            return false;
        }
        let Some(lookup_owner) = self.get_lookup_owner(manager, entity, &xform) else {
            return false;
        };
        let Some(aabb) = self.get_world_aabb(manager, entity) else {
            return false;
        };
        self.ensure_lookup(manager, lookup_owner)
            .add_or_update(entity, aabb);
        true
    }

    pub fn remove_from_entity_tree(&self, manager: &mut EntityManager, entity: EntityUid, recursive: bool) {
        for lookup in manager.entity_lookups.values_mut() {
            lookup.remove(entity);
        }

        if recursive {
            let children = manager
                .transforms
                .get(&entity)
                .map(|x| x.children.iter().copied().collect::<Vec<_>>())
                .unwrap_or_default();
            for child in children {
                self.remove_from_entity_tree(manager, child, true);
            }
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
    use keisan::Vector2i;

    #[test]
    fn lookup_system_can_query_entities_intersecting_grid_tiles() {
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

        manager
            .map_grids
            .get_mut(&grid_uid)
            .unwrap()
            .set_tile(Vector2i::new(0, 0), Tile::new(1, TileRenderFlag(0), 0));

        let entity = manager.create_entity_uninitialized(None);
        manager.transforms.get_mut(&entity).unwrap().base.owner = entity;
        manager.transforms.get_mut(&entity).unwrap().parent = grid_uid;
        manager.transforms.get_mut(&entity).unwrap().local_position = keisan::Vector2::new(0.5, 0.5);
        manager.transforms.get_mut(&entity).unwrap().rebuild_for_manager();

        let system = EntityLookupSystem;
        system.ensure_lookup(&mut manager, grid_uid);
        system.update_bounds(&mut manager, entity);
        let hits = system.get_entities_intersecting(&manager, GridId::new(3), Vector2i::new(0, 0));
        assert!(hits.contains(&entity));
    }
}
