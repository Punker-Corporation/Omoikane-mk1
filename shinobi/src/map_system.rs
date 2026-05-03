use crate::ClientEntityManager;
use butsuri::CollisionRay;
use keisan::{Box2, Vector2, Vector2i};
use sekai::{
    EntityLookupSystem, EntityUid, GameStateMapData, GridId, MapId, PhysicsQueryHit, TileRef,
};

use crate::PhysicsSystem;

#[derive(Debug, Clone, Default)]
pub struct MapSystem {
    lookup: EntityLookupSystem,
}

impl MapSystem {
    pub fn new() -> Self {
        Self {
            lookup: EntityLookupSystem,
        }
    }

    pub fn apply_game_state(
        &self,
        entities: &mut ClientEntityManager,
        map_data: &GameStateMapData,
    ) {
        entities.apply_map_data(map_data);
    }

    pub fn grid_exists(&self, entities: &ClientEntityManager, grid_id: GridId) -> bool {
        entities
            .inner
            .map_grid_components
            .values()
            .any(|component| component.grid_index == grid_id)
    }

    pub fn map_entity(&self, entities: &ClientEntityManager, map_id: MapId) -> Option<EntityUid> {
        entities.map_entity_for(map_id)
    }

    pub fn grid_entity(
        &self,
        entities: &ClientEntityManager,
        grid_id: GridId,
    ) -> Option<EntityUid> {
        entities
            .inner
            .map_grid_components
            .iter()
            .find(|(_, component)| component.grid_index == grid_id)
            .map(|(uid, _)| *uid)
    }

    pub fn grid_world_position(
        &self,
        entities: &ClientEntityManager,
        grid_id: GridId,
    ) -> Option<Vector2> {
        let uid = self.grid_entity(entities, grid_id)?;
        entities
            .inner
            .map_grids
            .get(&uid)
            .map(|grid| grid.world_position)
    }

    pub fn grid_world_bounds(
        &self,
        entities: &ClientEntityManager,
        grid_id: GridId,
    ) -> Option<Box2> {
        let uid = self.grid_entity(entities, grid_id)?;
        entities
            .inner
            .map_grids
            .get(&uid)
            .map(|grid| grid.world_bounds())
    }

    pub fn try_find_grid_at(
        &self,
        entities: &ClientEntityManager,
        map_id: MapId,
        world_pos: Vector2,
    ) -> Option<GridId> {
        entities
            .inner
            .map_grids
            .values()
            .find(|grid| {
                grid.parent_map_id == map_id
                    && grid.collides_with_grid(grid.world_to_tile(world_pos))
            })
            .map(|grid| grid.index)
    }

    pub fn tile_ref(
        &self,
        entities: &ClientEntityManager,
        grid_id: GridId,
        tile_indices: Vector2i,
    ) -> Option<TileRef> {
        let uid = self.grid_entity(entities, grid_id)?;
        entities
            .inner
            .map_grids
            .get(&uid)
            .map(|grid| grid.get_tile_ref(tile_indices))
    }

    pub fn tile_world_bounds(
        &self,
        entities: &ClientEntityManager,
        grid_id: GridId,
        tile_indices: Vector2i,
    ) -> Option<Box2> {
        let tile_ref = self.tile_ref(entities, grid_id, tile_indices)?;
        self.lookup
            .get_world_bounds(&entities.inner, tile_ref, None)
    }

    pub fn entities_at_tile(
        &self,
        entities: &ClientEntityManager,
        grid_id: GridId,
        tile_indices: Vector2i,
    ) -> Vec<EntityUid> {
        self.lookup
            .get_entities_intersecting(&entities.inner, grid_id, tile_indices)
    }

    pub fn entities_at_tiles<I>(
        &self,
        entities: &ClientEntityManager,
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
        entities: &ClientEntityManager,
        grid_id: GridId,
        world_aabb: Box2,
        include_anchored: bool,
    ) -> Vec<EntityUid> {
        let Some(grid_uid) = self.grid_entity(entities, grid_id) else {
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
        entities: &ClientEntityManager,
        entity: EntityUid,
    ) -> Option<Box2> {
        self.lookup.get_world_aabb(&entities.inner, entity)
    }

    pub fn entities_in_map_aabb(
        &self,
        entities: &ClientEntityManager,
        physics: &PhysicsSystem,
        map_id: MapId,
        world_aabb: Box2,
    ) -> Vec<EntityUid> {
        let Some(map_owner) = self.map_entity(entities, map_id) else {
            return Vec::new();
        };
        physics.query_aabb_entities(entities, map_owner, world_aabb)
    }

    pub fn intersect_ray(
        &self,
        entities: &ClientEntityManager,
        physics: &PhysicsSystem,
        map_id: MapId,
        ray: CollisionRay,
        max_length: f32,
        return_on_first_hit: bool,
    ) -> Vec<PhysicsQueryHit> {
        let Some(map_owner) = self.map_entity(entities, map_id) else {
            return Vec::new();
        };
        physics.intersect_ray(entities, map_owner, ray, max_length, return_on_first_hit)
    }
}

#[cfg(test)]
mod tests {
    use super::MapSystem;
    use crate::{ClientEntityManager, PhysicsSystem};
    use butsuri::{AabbShape, CollisionRay, Fixture, PhysShape};
    use keisan::{Angle, Box2, Vector2, Vector2i};
    use sekai::{
        BodyType, ChunkDatum, EntityUid, GameStateMapData, GridDatum, GridId, MapCoordinates,
        MapId, Tile, TileRenderFlag,
    };

    #[test]
    fn map_system_applies_grid_state() {
        let mut entities = ClientEntityManager::new();
        let system = MapSystem::new();
        system.apply_game_state(
            &mut entities,
            &GameStateMapData {
                grid_data: std::iter::once((
                    GridId::new(4),
                    GridDatum {
                        coordinates: MapCoordinates::new(Vector2::ZERO, MapId::new(1)),
                        angle: Angle::ZERO,
                        chunk_data: vec![ChunkDatum::create_modified(
                            Vector2i::new(0, 0),
                            vec![Tile::new(2, TileRenderFlag(0), 0)],
                        )],
                    },
                ))
                .collect(),
                deleted_grids: Vec::new(),
            },
        );
        assert!(system.grid_exists(&entities, GridId::new(4)));
        assert!(system.map_entity(&entities, MapId::new(1)).is_some());
        assert!(system.grid_entity(&entities, GridId::new(4)).is_some());
        assert_eq!(
            system.grid_world_position(&entities, GridId::new(4)),
            Some(Vector2::ZERO)
        );
        assert_eq!(
            system.try_find_grid_at(&entities, MapId::new(1), Vector2::new(0.5, 0.5)),
            Some(GridId::new(4))
        );
        let bounds = system.grid_world_bounds(&entities, GridId::new(4)).unwrap();
        assert!(bounds.contains(Vector2::new(0.0, 0.0), true));
    }

    #[test]
    fn map_system_can_query_tile_refs_bounds_and_entities() {
        let mut entities = ClientEntityManager::new();
        let system = MapSystem::new();
        system.apply_game_state(
            &mut entities,
            &GameStateMapData {
                grid_data: std::iter::once((
                    GridId::new(4),
                    GridDatum {
                        coordinates: MapCoordinates::new(Vector2::ZERO, MapId::new(1)),
                        angle: Angle::ZERO,
                        chunk_data: vec![ChunkDatum::create_modified(
                            Vector2i::new(0, 0),
                            vec![Tile::new(2, TileRenderFlag(0), 0)],
                        )],
                    },
                ))
                .collect(),
                deleted_grids: Vec::new(),
            },
        );

        let grid_uid = system.grid_entity(&entities, GridId::new(4)).unwrap();
        let uid = entities.create_entity(None, EntityUid::new(77));
        entities.initialize_entity(uid);
        {
            let transform = entities.inner.transforms.get_mut(&uid).unwrap();
            transform.map_id = MapId::new(1);
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
            .insert_fixture(Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ));
        let physics = PhysicsSystem::new();
        let map_owner = system.map_entity(&entities, MapId::new(1)).unwrap();
        entities.inner.ensure_broadphase(map_owner);
        sekai::SharedPhysicsSystem.sync_broadphase(&mut entities.inner, map_owner, &[uid]);

        assert_eq!(
            system
                .tile_ref(&entities, GridId::new(4), Vector2i::new(0, 0))
                .unwrap()
                .tile
                .type_id,
            2
        );
        assert!(
            system
                .tile_world_bounds(&entities, GridId::new(4), Vector2i::new(0, 0))
                .unwrap()
                .contains(Vector2::new(0.5, 0.5), true)
        );
        assert_eq!(
            system.entities_at_tile(&entities, GridId::new(4), Vector2i::new(0, 0)),
            vec![uid]
        );
        assert_eq!(
            system.entities_at_tiles(
                &entities,
                GridId::new(4),
                [Vector2i::new(0, 0), Vector2i::new(0, 0)]
            ),
            vec![uid]
        );
        assert_eq!(
            system.entities_in_grid_aabb(
                &entities,
                GridId::new(4),
                Box2::new(-1.0, -1.0, 1.0, 1.0),
                false
            ),
            vec![uid]
        );
        assert!(
            system
                .entity_world_aabb(&entities, uid)
                .unwrap()
                .contains(Vector2::new(0.5, 0.5), true)
        );
        assert_eq!(
            system.entities_in_map_aabb(
                &entities,
                &physics,
                MapId::new(1),
                Box2::new(-1.0, -1.0, 1.0, 1.0)
            ),
            vec![uid]
        );
        assert_eq!(
            system.intersect_ray(
                &entities,
                &physics,
                MapId::new(1),
                CollisionRay::new(Vector2::new(-5.0, 0.5), Vector2::UNIT_X, -1),
                10.0,
                true,
            )[0]
            .entity,
            uid
        );
    }

    #[test]
    fn map_system_can_query_anchored_entities_after_runtime_rebuild() {
        let mut entities = ClientEntityManager::new();
        let system = MapSystem::new();
        system.apply_game_state(
            &mut entities,
            &GameStateMapData {
                grid_data: std::iter::once((
                    GridId::new(4),
                    GridDatum {
                        coordinates: MapCoordinates::new(Vector2::ZERO, MapId::new(1)),
                        angle: Angle::ZERO,
                        chunk_data: vec![ChunkDatum::create_modified(
                            Vector2i::new(0, 0),
                            vec![Tile::new(2, TileRenderFlag(0), 0)],
                        )],
                    },
                ))
                .collect(),
                deleted_grids: Vec::new(),
            },
        );

        let grid_uid = system.grid_entity(&entities, GridId::new(4)).unwrap();
        let uid = entities.create_entity(None, EntityUid::new(88));
        entities.initialize_entity(uid);
        let _ = entities.inner.apply_transform_state(
            uid,
            sekai::TransformComponentState {
                local_position: Vector2::new(0.5, 0.5),
                rotation: Angle::ZERO,
                parent_id: grid_uid,
                map_id: MapId::new(1),
                grid_id: GridId::new(4),
                no_local_rotation: false,
                anchored: true,
            },
        );
        entities.rebuild_runtime_state();

        assert_eq!(
            system.entities_at_tile(&entities, GridId::new(4), Vector2i::new(0, 0)),
            vec![uid]
        );
        assert_eq!(
            system.entities_in_grid_aabb(
                &entities,
                GridId::new(4),
                Box2::new(-1.0, -1.0, 1.0, 1.0),
                true
            ),
            vec![uid]
        );
    }

    #[test]
    fn map_system_intersect_ray_respects_collision_mask() {
        let mut entities = ClientEntityManager::new();
        let system = MapSystem::new();
        system.apply_game_state(
            &mut entities,
            &GameStateMapData {
                grid_data: std::iter::once((
                    GridId::new(4),
                    GridDatum {
                        coordinates: MapCoordinates::new(Vector2::ZERO, MapId::new(1)),
                        angle: Angle::ZERO,
                        chunk_data: Vec::new(),
                    },
                ))
                .collect(),
                deleted_grids: Vec::new(),
            },
        );

        let first = entities.create_entity(None, EntityUid::new(91));
        entities.initialize_entity(first);
        let _ = entities.inner.apply_transform_state(
            first,
            sekai::TransformComponentState {
                local_position: Vector2::new(5.0, 0.0),
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(1),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(first);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        body.awake = true;
        let mut first_fixture = Fixture::new(
            "first",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        );
        first_fixture.collision_layer = 1 << 0;
        entities
            .inner
            .ensure_fixtures(first)
            .insert_fixture(first_fixture);

        let second = entities.create_entity(None, EntityUid::new(92));
        entities.initialize_entity(second);
        let _ = entities.inner.apply_transform_state(
            second,
            sekai::TransformComponentState {
                local_position: Vector2::new(8.0, 0.0),
                rotation: Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(1),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(second);
        body.can_collide = true;
        body.set_body_type(BodyType::Dynamic);
        body.awake = true;
        let mut second_fixture = Fixture::new(
            "second",
            PhysShape::Aabb(AabbShape::new(Box2::new(-1.0, -1.0, 1.0, 1.0), 0.0)),
        );
        second_fixture.collision_layer = 1 << 1;
        entities
            .inner
            .ensure_fixtures(second)
            .insert_fixture(second_fixture);

        let physics = PhysicsSystem::new();
        entities.sync_map_physics_runtime(MapId::new(1));

        let hits = system.intersect_ray(
            &entities,
            &physics,
            MapId::new(1),
            CollisionRay::new(Vector2::ZERO, Vector2::UNIT_X, 1 << 1),
            10.0,
            false,
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entity, second);
    }
}
