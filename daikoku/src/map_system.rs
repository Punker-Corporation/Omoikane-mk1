use crate::{PhysicsSystem, ServerEntityManager, TransformSystem};
use butsuri::CollisionRay;
use keisan::{Box2, Vector2, Vector2i};
use sekai::{
    EntityLookupSystem, EntityUid, GridId, MapGrid, MapId, MapManager, PhysicsQueryHit, TileRef,
};

#[derive(Debug, Clone)]
pub struct MapSystem {
    pub delete_empty_grids: bool,
    lookup: EntityLookupSystem,
}

impl MapSystem {
    pub fn new() -> Self {
        Self {
            delete_empty_grids: false,
            lookup: EntityLookupSystem,
        }
    }

    pub fn set_grid_deletion(
        &mut self,
        value: bool,
        maps: &mut MapManager,
        entities: &mut ServerEntityManager,
    ) {
        self.delete_empty_grids = value;
        if !value {
            return;
        }
        let empty: Vec<_> = entities
            .inner
            .map_grids
            .values()
            .filter(|grid| self.grid_empty(grid))
            .map(|grid| grid.index)
            .collect();
        for grid_id in empty {
            maps.delete_grid(&mut entities.inner, grid_id);
        }
    }

    pub fn on_grid_empty(
        &self,
        maps: &mut MapManager,
        entities: &mut ServerEntityManager,
        grid_id: GridId,
    ) -> bool {
        if !self.delete_empty_grids {
            return false;
        }
        maps.delete_grid(&mut entities.inner, grid_id);
        true
    }

    pub fn handle_map_created(
        &self,
        maps: &mut MapManager,
        _entities: &mut ServerEntityManager,
        map: MapId,
    ) -> Option<EntityUid> {
        (map != MapId::NULLSPACE).then(|| maps.get_map_entity_id_or_throw(map))
    }

    pub fn map_entity(&self, maps: &MapManager, map: MapId) -> Option<EntityUid> {
        (map != MapId::NULLSPACE)
            .then(|| maps.get_map_entity_id(map))
            .filter(|uid| uid.is_valid())
    }

    pub fn grid_entity(&self, maps: &MapManager, grid_id: GridId) -> Option<EntityUid> {
        maps.get_grid_euid(grid_id)
    }

    pub fn grid(
        &self,
        maps: &MapManager,
        entities: &ServerEntityManager,
        grid_id: GridId,
    ) -> Option<MapGrid> {
        maps.get_grid(&entities.inner, grid_id).cloned()
    }

    pub fn try_find_grid_at(
        &self,
        maps: &MapManager,
        entities: &ServerEntityManager,
        map_id: MapId,
        world_pos: Vector2,
    ) -> Option<GridId> {
        maps.try_find_grid_at(&entities.inner, map_id, world_pos)
            .map(|grid| grid.index)
    }

    pub fn find_grids_intersecting(
        &self,
        maps: &MapManager,
        entities: &ServerEntityManager,
        map_id: MapId,
        world_aabb: Box2,
        approx: bool,
    ) -> Vec<GridId> {
        maps.find_grids_intersecting(&entities.inner, map_id, world_aabb, approx)
            .into_iter()
            .map(|grid| grid.index)
            .collect()
    }

    pub fn tile_ref(
        &self,
        maps: &MapManager,
        entities: &ServerEntityManager,
        grid_id: GridId,
        tile_indices: Vector2i,
    ) -> Option<TileRef> {
        maps.get_grid(&entities.inner, grid_id)
            .map(|grid| grid.get_tile_ref(tile_indices))
    }

    pub fn tile_world_bounds(
        &self,
        maps: &MapManager,
        entities: &ServerEntityManager,
        grid_id: GridId,
        tile_indices: Vector2i,
    ) -> Option<Box2> {
        let tile_ref = self.tile_ref(maps, entities, grid_id, tile_indices)?;
        self.lookup
            .get_world_bounds(&entities.inner, tile_ref, None)
    }

    pub fn entities_at_tile(
        &self,
        entities: &ServerEntityManager,
        grid_id: GridId,
        tile_indices: Vector2i,
    ) -> Vec<EntityUid> {
        self.lookup
            .get_entities_intersecting(&entities.inner, grid_id, tile_indices)
    }

    pub fn entities_at_tiles<I>(
        &self,
        entities: &ServerEntityManager,
        grid_id: GridId,
        tile_indices: I,
    ) -> Vec<EntityUid>
    where
        I: IntoIterator<Item = Vector2i>,
    {
        self.lookup
            .get_entities_intersecting_many(&entities.inner, grid_id, tile_indices)
    }

    pub fn entities_in_grid_aabb(
        &self,
        maps: &MapManager,
        entities: &ServerEntityManager,
        grid_id: GridId,
        world_aabb: Box2,
        include_anchored: bool,
    ) -> Vec<EntityUid> {
        let Some(grid_uid) = maps.get_grid_euid(grid_id) else {
            return Vec::new();
        };
        self.lookup.get_entities_intersecting_world_aabb(
            &entities.inner,
            grid_uid,
            world_aabb,
            include_anchored,
        )
    }

    pub fn entity_world_aabb(
        &self,
        entities: &ServerEntityManager,
        entity: EntityUid,
    ) -> Option<Box2> {
        self.lookup.get_world_aabb(&entities.inner, entity)
    }

    pub fn entities_in_map_aabb(
        &self,
        maps: &MapManager,
        entities: &ServerEntityManager,
        physics: &PhysicsSystem,
        map_id: MapId,
        world_aabb: Box2,
    ) -> Vec<EntityUid> {
        let Some(map_owner) = self.map_entity(maps, map_id) else {
            return Vec::new();
        };
        physics.query_aabb_entities(entities, map_owner, world_aabb)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn intersect_ray(
        &self,
        maps: &MapManager,
        entities: &ServerEntityManager,
        physics: &PhysicsSystem,
        map_id: MapId,
        ray: CollisionRay,
        max_length: f32,
        return_on_first_hit: bool,
    ) -> Vec<PhysicsQueryHit> {
        let Some(map_owner) = self.map_entity(maps, map_id) else {
            return Vec::new();
        };
        physics.intersect_ray(entities, map_owner, ray, max_length, return_on_first_hit)
    }

    pub fn set_grid_position(
        &self,
        maps: &MapManager,
        entities: &mut ServerEntityManager,
        transforms: &mut TransformSystem,
        grid_id: GridId,
        world_position: Vector2,
    ) -> bool {
        let Some(uid) = maps.get_grid_euid(grid_id) else {
            return false;
        };
        transforms.set_local_position(entities, uid, world_position)
    }

    pub fn set_grid_rotation(
        &self,
        maps: &MapManager,
        entities: &mut ServerEntityManager,
        transforms: &mut TransformSystem,
        grid_id: GridId,
        rotation: keisan::Angle,
    ) -> bool {
        let Some(uid) = maps.get_grid_euid(grid_id) else {
            return false;
        };
        transforms.set_local_rotation(entities, uid, rotation)
    }

    fn grid_empty(&self, grid: &sekai::MapGrid) -> bool {
        !grid
            .get_all_tiles(true)
            .into_iter()
            .any(|tile| !tile.tile.is_empty())
    }
}

impl Default for MapSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::MapSystem;
    use crate::{PhysicsSystem, ServerEntityManager, TransformSystem};
    use butsuri::CollisionRay;
    use keisan::{Angle, Box2, Vector2, Vector2i};
    use sekai::{BodyType, GridId, MapId, MapManager, Tile, TileRenderFlag};

    #[test]
    fn map_system_can_delete_empty_grids_when_enabled() {
        let mut maps = MapManager::new();
        let mut entities = ServerEntityManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(2)));
        let grid_id = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(5)), 8);
        let mut system = MapSystem::new();
        system.set_grid_deletion(true, &mut maps, &mut entities);
        assert!(!maps.grid_exists(grid_id));
    }

    #[test]
    fn map_system_can_query_and_move_grids() {
        let mut maps = MapManager::new();
        let mut entities = ServerEntityManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(2)));
        let grid_id = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(5)), 8);
        let grid_uid = maps.get_grid_euid(grid_id).unwrap();
        entities
            .inner
            .map_grids
            .get_mut(&grid_uid)
            .unwrap()
            .set_tile(Vector2i::new(0, 0), Tile::new(1, TileRenderFlag(0), 0));

        let system = MapSystem::new();
        assert_eq!(
            system.map_entity(&maps, map_id),
            Some(maps.get_map_entity_id(map_id))
        );
        assert_eq!(system.grid_entity(&maps, grid_id), Some(grid_uid));
        assert_eq!(
            system.try_find_grid_at(&maps, &entities, map_id, Vector2::ZERO),
            Some(grid_id)
        );
        assert_eq!(
            system.find_grids_intersecting(
                &maps,
                &entities,
                map_id,
                Box2::new(-1.0, -1.0, 1.0, 1.0),
                false
            ),
            vec![grid_id]
        );

        let mut transforms = TransformSystem::new();
        assert!(system.set_grid_position(
            &maps,
            &mut entities,
            &mut transforms,
            grid_id,
            Vector2::new(2.0, 0.0)
        ));
        assert!(system.set_grid_rotation(
            &maps,
            &mut entities,
            &mut transforms,
            grid_id,
            Angle::from_degrees(30.0)
        ));
        let _ = transforms.process_deferred_moves(&mut entities, &mut maps);
        let grid = system.grid(&maps, &entities, grid_id).unwrap();
        assert_eq!(grid.world_position, Vector2::new(2.0, 0.0));
        assert_eq!(grid.world_rotation, Angle::from_degrees(30.0));
    }

    #[test]
    fn map_system_can_query_tiles_and_entities() {
        let mut maps = MapManager::new();
        let mut entities = ServerEntityManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(2)));
        let grid_id = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(5)), 8);
        let grid_uid = maps.get_grid_euid(grid_id).unwrap();
        entities
            .inner
            .map_grids
            .get_mut(&grid_uid)
            .unwrap()
            .set_tile(Vector2i::new(0, 0), Tile::new(1, TileRenderFlag(0), 0));

        let uid = entities.create_entity(None);
        entities.initialize_entity(uid);
        {
            let transform = entities.inner.transforms.get_mut(&uid).unwrap();
            transform.map_id = map_id;
            transform.parent = grid_uid;
            transform.local_position = Vector2::new(0.5, 0.5);
            transform.rebuild_for_manager();
        }
        entities.inner.ensure_lookup(grid_uid);
        sekai::EntityLookupSystem.update_bounds(&mut entities.inner, uid);

        let body = entities.inner.ensure_physics(uid);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        body.awake = true;
        entities
            .inner
            .ensure_fixtures(uid)
            .insert_fixture(butsuri::Fixture::new(
                "main",
                butsuri::PhysShape::Aabb(butsuri::AabbShape::new(
                    Box2::new(-0.5, -0.5, 0.5, 0.5),
                    0.0,
                )),
            ));
        let physics = PhysicsSystem::new();
        let _ = physics.sync_map_physics(&mut entities, &maps);

        let system = MapSystem::new();
        assert_eq!(
            system
                .tile_ref(&maps, &entities, grid_id, Vector2i::new(0, 0))
                .unwrap()
                .tile
                .type_id,
            1
        );
        assert_eq!(
            system.entities_at_tile(&entities, grid_id, Vector2i::new(0, 0)),
            vec![uid]
        );
        assert_eq!(
            system.entities_at_tiles(
                &entities,
                grid_id,
                [Vector2i::new(0, 0), Vector2i::new(0, 0)]
            ),
            vec![uid]
        );
        assert_eq!(
            system.entities_in_grid_aabb(
                &maps,
                &entities,
                grid_id,
                Box2::new(-1.0, -1.0, 1.0, 1.0),
                false
            ),
            vec![uid]
        );
        assert!(
            system
                .tile_world_bounds(&maps, &entities, grid_id, Vector2i::new(0, 0))
                .unwrap()
                .contains(Vector2::new(0.5, 0.5), true)
        );
        assert!(
            system
                .entity_world_aabb(&entities, uid)
                .unwrap()
                .contains(Vector2::new(0.5, 0.5), true)
        );
        assert_eq!(
            system.entities_in_map_aabb(
                &maps,
                &entities,
                &physics,
                map_id,
                Box2::new(-1.0, -1.0, 1.0, 1.0)
            ),
            vec![uid]
        );
        assert_eq!(
            system.intersect_ray(
                &maps,
                &entities,
                &physics,
                map_id,
                CollisionRay::new(Vector2::new(-5.0, 0.5), Vector2::UNIT_X, -1),
                10.0,
                true,
            )[0]
            .entity,
            uid
        );
    }

    #[test]
    fn map_system_can_query_anchored_entities_in_tiles_and_grid_aabbs() {
        let mut maps = MapManager::new();
        let mut entities = ServerEntityManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(2)));
        let grid_id = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(5)), 8);
        let grid_uid = maps.get_grid_euid(grid_id).unwrap();
        entities
            .inner
            .map_grids
            .get_mut(&grid_uid)
            .unwrap()
            .set_tile(Vector2i::new(0, 0), Tile::new(1, TileRenderFlag(0), 0));

        let uid = entities.create_entity(None);
        entities.initialize_entity(uid);
        let _ = entities.inner.apply_transform_state(
            uid,
            sekai::TransformComponentState {
                local_position: Vector2::new(0.5, 0.5),
                rotation: Angle::ZERO,
                parent_id: grid_uid,
                map_id,
                grid_id,
                no_local_rotation: false,
                anchored: true,
            },
        );
        assert!(sekai::EntityLookupSystem.update_bounds(&mut entities.inner, uid));

        let system = MapSystem::new();
        assert_eq!(
            system.entities_at_tile(&entities, grid_id, Vector2i::new(0, 0)),
            vec![uid]
        );
        assert_eq!(
            system.entities_in_grid_aabb(
                &maps,
                &entities,
                grid_id,
                Box2::new(-1.0, -1.0, 1.0, 1.0),
                true
            ),
            vec![uid]
        );
    }

    #[test]
    fn map_system_intersect_ray_respects_collision_mask() {
        let mut maps = MapManager::new();
        let mut entities = ServerEntityManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(2)));

        let first = entities.create_entity(None);
        entities.initialize_entity(first);
        {
            let transform = entities.inner.transforms.get_mut(&first).unwrap();
            transform.map_id = map_id;
            transform.local_position = Vector2::new(5.0, 0.0);
            transform.rebuild_for_manager();
        }
        let body = entities.inner.ensure_physics(first);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        body.awake = true;
        let mut first_fixture = butsuri::Fixture::new(
            "first",
            butsuri::PhysShape::Aabb(butsuri::AabbShape::new(
                Box2::new(-1.0, -1.0, 1.0, 1.0),
                0.0,
            )),
        );
        first_fixture.collision_layer = 1 << 0;
        entities
            .inner
            .ensure_fixtures(first)
            .insert_fixture(first_fixture);

        let second = entities.create_entity(None);
        entities.initialize_entity(second);
        {
            let transform = entities.inner.transforms.get_mut(&second).unwrap();
            transform.map_id = map_id;
            transform.local_position = Vector2::new(8.0, 0.0);
            transform.rebuild_for_manager();
        }
        let body = entities.inner.ensure_physics(second);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        body.awake = true;
        let mut second_fixture = butsuri::Fixture::new(
            "second",
            butsuri::PhysShape::Aabb(butsuri::AabbShape::new(
                Box2::new(-1.0, -1.0, 1.0, 1.0),
                0.0,
            )),
        );
        second_fixture.collision_layer = 1 << 1;
        entities
            .inner
            .ensure_fixtures(second)
            .insert_fixture(second_fixture);

        let physics = PhysicsSystem::new();
        let _ = physics.sync_map_physics(&mut entities, &maps);

        let system = MapSystem::new();
        let hits = system.intersect_ray(
            &maps,
            &entities,
            &physics,
            map_id,
            CollisionRay::new(Vector2::ZERO, Vector2::UNIT_X, 1 << 1),
            10.0,
            false,
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entity, second);
    }
}
