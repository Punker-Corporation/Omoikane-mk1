use crate::ServerEntityManager;
use keisan::{Angle, Vector2, Vector2i};
use sekai::{EntityUid, GridId, MoveEvent};

#[derive(Debug, Default, Clone)]
pub struct TransformSystem;

impl TransformSystem {
    pub fn new() -> Self {
        Self
    }

    pub fn process_deferred_moves(
        &mut self,
        entities: &mut ServerEntityManager,
        maps: &mut sekai::MapManager,
    ) -> Vec<MoveEvent> {
        let mut processed = entities.inner.process_deferred_move_events();
        for move_event in &mut processed {
            if let Some(grid_component) = entities.inner.map_grid_components.get(&move_event.sender)
            {
                if let Some(transform) = entities.inner.transforms.get(&move_event.sender) {
                    maps.mark_grid_moved(transform.map_id, grid_component.grid_index);
                }
                let world_state = entities
                    .inner
                    .transforms
                    .get(&move_event.sender)
                    .map(|transform| transform.get_world_position_rotation_matrix(&entities.inner));
                if let Some(grid) = entities.inner.map_grids.get_mut(&move_event.sender) {
                    if let Some((world_pos, world_rot, _)) = world_state {
                        grid.world_position = world_pos;
                        grid.world_rotation = world_rot;
                    }
                }
            }
            let previous_map = entities
                .inner
                .transforms
                .get(&move_event.sender)
                .map(|transform| transform.map_id)
                .unwrap_or(sekai::MapId::NULLSPACE);
            entities.inner.apply_transform_move_event_and_reconcile(
                move_event.sender,
                previous_map,
                move_event,
            );
        }
        processed
    }

    pub fn set_local_transform(
        &mut self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        position: Vector2,
        rotation: Angle,
    ) -> bool {
        let Some(event) = entities
            .inner
            .set_local_transform_deferred(uid, position, rotation)
        else {
            return false;
        };
        entities.inner.defer_move_event(event);
        true
    }

    pub fn offset_local_transform(
        &mut self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        delta: Vector2,
        angular_delta: Angle,
    ) -> bool {
        let Some(event) = entities
            .inner
            .offset_local_transform_deferred(uid, delta, angular_delta)
        else {
            return false;
        };
        entities.inner.defer_move_event(event);
        true
    }

    pub fn anchor_entity(
        &mut self,
        entities: &mut ServerEntityManager,
        uid: EntityUid,
        grid_uid: EntityUid,
        grid_id: GridId,
        tile_indices: Vector2i,
    ) -> bool {
        let Some((map_id, tile_center)) = entities.inner.map_grids.get(&grid_uid).map(|grid| {
            (
                grid.parent_map_id,
                grid.grid_tile_to_local(tile_indices).position,
            )
        }) else {
            return false;
        };

        let Some(existing) = entities.inner.transforms.get(&uid).cloned() else {
            return false;
        };
        let Some(event) = entities.inner.apply_transform_state_with_move_event(
            uid,
            sekai::TransformComponentState {
                local_position: tile_center,
                rotation: existing.local_rotation,
                parent_id: grid_uid,
                map_id,
                grid_id,
                no_local_rotation: existing.no_local_rotation,
                anchored: true,
            },
        ) else {
            return false;
        };
        entities.inner.defer_move_event(event);
        true
    }

    pub fn unanchor_entity(&mut self, entities: &mut ServerEntityManager, uid: EntityUid) -> bool {
        let Some(event) = entities.inner.set_anchored_with_move_event(uid, false) else {
            return false;
        };
        entities.inner.defer_move_event(event);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::TransformSystem;
    use crate::ServerEntityManager;
    use butsuri::{AabbShape, BodyType, Fixture, PhysShape};
    use keisan::{ApproxEq, Box2, Vector2, Vector2i};
    use sekai::{EntityUid, GridId, MapId, MapManager, Tile, TileRenderFlag};

    #[test]
    fn transform_system_reads_world_position_for_entities() {
        let entities = ServerEntityManager::new();
        let mut entities = entities;
        let uid = entities.inner.create_entity_uninitialized(None);
        assert_eq!(
            entities.inner.world_position(uid),
            Some(keisan::Vector2::ZERO)
        );
    }

    #[test]
    fn transform_system_moves_entities_and_marks_grids() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(2)));
        let grid_id = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(7)), 8);
        let grid_uid = maps.get_grid_euid(grid_id).unwrap();
        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        assert!(
            entities
                .inner
                .mutate_transform_and_reconcile(uid, |transform| {
                    transform.parent = grid_uid;
                })
        );

        let mut system = TransformSystem::new();
        assert!(system.set_local_transform(
            &mut entities,
            uid,
            keisan::Vector2::new(1.0, 0.0),
            keisan::Angle::ZERO
        ));
        let processed = system.process_deferred_moves(&mut entities, &mut maps);
        assert_eq!(processed.len(), 1);

        assert!(system.set_local_transform(
            &mut entities,
            grid_uid,
            keisan::Vector2::new(2.0, 0.0),
            keisan::Angle::ZERO
        ));
        let processed = system.process_deferred_moves(&mut entities, &mut maps);
        assert_eq!(processed.len(), 1);
        assert!(maps.get_moved_grids(map_id).contains(&grid_id));
    }

    #[test]
    fn transform_system_deferred_moves_refresh_map_physics_runtime() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(90)));

        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        let _ = entities.inner.apply_transform_state(
            uid,
            sekai::TransformComponentState {
                local_position: Vector2::ZERO,
                rotation: keisan::Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id,
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let body = entities.inner.ensure_physics(uid);
        body.set_body_type(BodyType::Dynamic);
        body.can_collide = true;
        body.awake = true;
        let _ = entities.inner.insert_fixture_and_reconcile(
            uid,
            Fixture::new(
                "main",
                PhysShape::Aabb(AabbShape::new(Box2::new(-0.5, -0.5, 0.5, 0.5), 0.0)),
            ),
        );
        entities.inner.refresh_map_physics_runtime(map_id);
        assert_eq!(
            entities
                .inner
                .entities_in_map_aabb(map_id, Box2::new(-1.0, -1.0, 1.0, 1.0)),
            vec![uid]
        );

        let mut system = TransformSystem::new();
        assert!(system.set_local_transform(
            &mut entities,
            uid,
            Vector2::new(3.0, 0.0),
            keisan::Angle::ZERO
        ));
        let processed = system.process_deferred_moves(&mut entities, &mut maps);
        assert_eq!(processed.len(), 1);
        assert!(
            entities
                .inner
                .entities_in_map_aabb(map_id, Box2::new(-1.0, -1.0, 1.0, 1.0))
                .is_empty()
        );
        assert_eq!(
            entities
                .inner
                .entities_in_map_aabb(map_id, Box2::new(2.0, -1.0, 4.0, 1.0)),
            vec![uid]
        );
    }

    #[test]
    fn transform_system_sets_position_and_rotation_atomically() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);

        let mut system = TransformSystem::new();
        assert!(system.set_local_transform(
            &mut entities,
            uid,
            keisan::Vector2::new(2.0, -1.0),
            keisan::Angle::from_degrees(30.0),
        ));
        let processed = system.process_deferred_moves(&mut entities, &mut sekai::MapManager::new());
        assert_eq!(processed.len(), 1);
        let transform = entities.inner.transforms.get(&uid).unwrap();
        assert_eq!(transform.local_position, keisan::Vector2::new(2.0, -1.0));
        assert_eq!(transform.local_rotation, keisan::Angle::from_degrees(30.0));
    }

    #[test]
    fn transform_system_can_anchor_and_unanchor_entities_to_grid_tiles() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(2)));
        let grid_id = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(7)), 8);
        let grid_uid = maps.get_grid_euid(grid_id).unwrap();
        entities
            .inner
            .map_grids
            .get_mut(&grid_uid)
            .unwrap()
            .set_tile(Vector2i::new(0, 0), Tile::new(1, TileRenderFlag(0), 0));

        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        let mut system = TransformSystem::new();
        assert!(system.anchor_entity(&mut entities, uid, grid_uid, grid_id, Vector2i::new(0, 0)));
        let _ = system.process_deferred_moves(&mut entities, &mut maps);
        let transform = entities.inner.transforms.get(&uid).unwrap();
        assert!(transform.anchored);
        assert_eq!(transform.parent, grid_uid);
        assert_eq!(transform.local_position, keisan::Vector2::new(0.5, 0.5));
        assert_eq!(
            entities
                .inner
                .entities_at_tile(grid_id, Vector2i::new(0, 0)),
            vec![uid]
        );

        assert!(system.unanchor_entity(&mut entities, uid));
        let _ = system.process_deferred_moves(&mut entities, &mut maps);
        assert!(!entities.inner.transforms.get(&uid).unwrap().anchored);
    }

    #[test]
    fn transform_system_anchor_entity_preserves_map_velocities_for_dynamic_bodies() {
        let mut entities = ServerEntityManager::new();
        let mut maps = MapManager::new();
        maps.startup(&mut entities.inner);
        let map_id = maps.create_map(&mut entities.inner, Some(MapId::new(6)));
        let grid_id = maps.create_grid(&mut entities.inner, map_id, Some(GridId::new(9)), 8);
        let grid_uid = maps.get_grid_euid(grid_id).unwrap();
        entities
            .inner
            .map_grids
            .get_mut(&grid_uid)
            .unwrap()
            .set_tile(Vector2i::new(0, 0), Tile::new(1, TileRenderFlag(0), 0));

        let grid_body = entities.inner.ensure_physics(grid_uid);
        grid_body.set_body_type(sekai::BodyType::Dynamic);
        grid_body.linear_velocity = Vector2::new(2.0, -1.0);
        grid_body.angular_velocity = 0.75;

        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        let body = entities.inner.ensure_physics(uid);
        body.set_body_type(sekai::BodyType::Dynamic);
        body.linear_velocity = Vector2::new(-0.5, 1.25);
        body.angular_velocity = 1.0;

        let old_linear = entities.inner.entity_map_linear_velocity(uid).unwrap();
        let old_angular = entities.inner.entity_map_angular_velocity(uid).unwrap();

        let mut system = TransformSystem::new();
        assert!(system.anchor_entity(&mut entities, uid, grid_uid, grid_id, Vector2i::new(0, 0)));

        let new_linear = entities.inner.entity_map_linear_velocity(uid).unwrap();
        let new_angular = entities.inner.entity_map_angular_velocity(uid).unwrap();
        assert!(new_linear.approx_eq_with_tolerance(old_linear, 0.0001));
        assert!((new_angular - old_angular).abs() <= 0.0001);
    }

    #[test]
    fn transform_system_unanchor_entity_uses_shared_anchor_path() {
        let mut entities = ServerEntityManager::new();
        let uid = entities.inner.create_entity_uninitialized(None);
        entities.inner.initialize_entity(uid);
        let _ = entities.inner.apply_transform_state(
            uid,
            sekai::TransformComponentState {
                local_position: keisan::Vector2::new(1.0, 1.0),
                rotation: keisan::Angle::ZERO,
                parent_id: EntityUid::INVALID,
                map_id: MapId::new(7),
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: true,
            },
        );

        let mut system = TransformSystem::new();
        assert!(system.unanchor_entity(&mut entities, uid));
        assert!(!entities.inner.transforms.get(&uid).unwrap().anchored);
    }
}
