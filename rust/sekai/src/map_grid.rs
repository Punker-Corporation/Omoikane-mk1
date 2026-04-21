use crate::{EntityCoordinates, EntityUid, GridId, MapChunk, MapCoordinates, MapId, Tile, TileRef};
use keisan::{Angle, Box2, Matrix3, Vector2, Vector2i};
use std::collections::{HashMap, HashSet};

pub trait MapGridLike {
    fn parent_map_id(&self) -> MapId;
    fn grid_entity_id(&self) -> EntityUid;
    fn index(&self) -> GridId;
    fn tile_size(&self) -> u16;
    fn chunk_size(&self) -> u16;
    fn world_position(&self) -> Vector2;
    fn world_rotation(&self) -> Angle;
    fn world_matrix(&self) -> Matrix3;
    fn inv_world_matrix(&self) -> Matrix3;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapGridBounds {
    pub world_bounds: Box2,
    pub local_bounds: Box2,
}

#[derive(Debug, Clone)]
pub struct GridChunkIterator {
    indices: Vec<Vector2i>,
    cursor: usize,
}

impl Iterator for GridChunkIterator {
    type Item = Vector2i;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.indices.get(self.cursor).copied();
        if item.is_some() {
            self.cursor += 1;
        }
        item
    }
}

#[derive(Debug, Clone)]
pub struct MapGrid {
    pub parent_map_id: MapId,
    pub grid_entity_id: EntityUid,
    pub index: GridId,
    pub tile_size: u16,
    pub chunk_size: u16,
    pub world_position: Vector2,
    pub world_rotation: Angle,
    chunks: HashMap<Vector2i, MapChunk>,
    pub local_bounds: Box2,
}

impl MapGrid {
    pub fn new(parent_map_id: MapId, grid_entity_id: EntityUid, index: GridId, chunk_size: u16) -> Self {
        Self {
            parent_map_id,
            grid_entity_id,
            index,
            tile_size: 1,
            chunk_size,
            world_position: Vector2::ZERO,
            world_rotation: Angle::ZERO,
            chunks: HashMap::new(),
            local_bounds: Box2::default(),
        }
    }

    pub fn world_bounds(&self) -> Box2 {
        let bounds = self.local_bounds;
        let points = [
            self.local_to_world(bounds.bottom_left()),
            self.local_to_world(bounds.bottom_right()),
            self.local_to_world(bounds.top_left()),
            self.local_to_world(bounds.top_right()),
        ];

        points
            .into_iter()
            .fold(Box2::from_corners(points[0], points[0]), |acc, point| acc.extend_to_contain(point))
    }

    pub fn bounds(&self) -> MapGridBounds {
        MapGridBounds {
            world_bounds: self.world_bounds(),
            local_bounds: self.local_bounds,
        }
    }

    pub fn get_tile_ref_from_map(&self, coords: MapCoordinates) -> TileRef {
        self.get_tile_ref(self.coordinates_to_tile_map(coords))
    }

    pub fn get_tile_ref_from_entity(&self, coords: EntityCoordinates) -> TileRef {
        self.get_tile_ref(self.coordinates_to_tile_entity(coords))
    }

    pub fn get_tile_ref(&self, tile_coordinates: Vector2i) -> TileRef {
        let chunk_indices = self.grid_tile_to_chunk_indices(tile_coordinates);
        if let Some(chunk) = self.chunks.get(&chunk_indices) {
            let chunk_tile = chunk.grid_tile_to_chunk_tile(tile_coordinates);
            let tile = chunk.get_tile(chunk_tile.x as u16, chunk_tile.y as u16);
            TileRef::new(self.parent_map_id, self.index, tile_coordinates, tile)
        } else {
            TileRef::new(self.parent_map_id, self.index, tile_coordinates, Tile::EMPTY)
        }
    }

    pub fn get_all_tiles(&self, ignore_space: bool) -> Vec<TileRef> {
        let mut refs = Vec::new();
        for chunk in self.chunks.values() {
            for x in 0..self.chunk_size {
                for y in 0..self.chunk_size {
                    let tile = chunk.get_tile(x, y);
                    if ignore_space && tile.is_empty() {
                        continue;
                    }
                    let grid = Vector2i::new(x as i32, y as i32) + chunk.indices() * self.chunk_size as i32;
                    refs.push(TileRef::new(self.parent_map_id, self.index, grid, tile));
                }
            }
        }
        refs
    }

    pub fn set_tile(&mut self, grid_indices: Vector2i, tile: Tile) -> bool {
        let (chunk, chunk_tile) = self.chunk_and_offset_for_tile(grid_indices);
        let changed = chunk
            .set_tile(chunk_tile.x as u16, chunk_tile.y as u16, tile)
            .is_some();
        if changed {
            self.recalculate_local_bounds();
        }
        changed
    }

    pub fn set_tiles(&mut self, tiles: &[(Vector2i, Tile)]) -> Vec<Vector2i> {
        let mut touched = HashSet::new();
        for (indices, tile) in tiles {
            let (chunk, chunk_tile) = self.chunk_and_offset_for_tile(*indices);
            if chunk.set_tile(chunk_tile.x as u16, chunk_tile.y as u16, *tile).is_some() {
                touched.insert(chunk.indices());
            }
        }
        if !touched.is_empty() {
            self.recalculate_local_bounds();
        }
        let mut touched = touched.into_iter().collect::<Vec<_>>();
        touched.sort_by(|a, b| a.x.cmp(&b.x).then(a.y.cmp(&b.y)));
        touched
    }

    pub fn get_chunk(&mut self, chunk_indices: Vector2i) -> &mut MapChunk {
        self.chunks
            .entry(chunk_indices)
            .or_insert_with(|| MapChunk::new(chunk_indices.x, chunk_indices.y, self.chunk_size))
    }

    pub fn try_get_chunk(&self, chunk_indices: Vector2i) -> Option<&MapChunk> {
        self.chunks.get(&chunk_indices)
    }

    pub fn remove_chunk(&mut self, chunk_indices: Vector2i) -> bool {
        let removed = self.chunks.remove(&chunk_indices).is_some();
        if removed {
            self.recalculate_local_bounds();
        }
        removed
    }

    pub fn chunk_indices(&self) -> Vec<Vector2i> {
        let mut indices = self.chunks.keys().copied().collect::<Vec<_>>();
        indices.sort_by(|a, b| a.x.cmp(&b.x).then(a.y.cmp(&b.y)));
        indices
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn get_map_chunks_intersecting(&self, world_aabb: Box2) -> GridChunkIterator {
        let matrix = self.inv_world_matrix();
        let points = [
            matrix * world_aabb.bottom_left(),
            matrix * world_aabb.bottom_right(),
            matrix * world_aabb.top_left(),
            matrix * world_aabb.top_right(),
        ];
        let local_area = points
            .into_iter()
            .fold(Box2::from_corners(points[0], points[0]), |acc, point| acc.extend_to_contain(point));
        let chunk_lb = Vector2i::new(
            (local_area.left / self.chunk_size as f32).floor() as i32,
            (local_area.bottom / self.chunk_size as f32).floor() as i32,
        );
        let chunk_rt = Vector2i::new(
            (local_area.right / self.chunk_size as f32).floor() as i32,
            (local_area.top / self.chunk_size as f32).floor() as i32,
        );
        let mut indices = Vec::new();
        for x in chunk_lb.x..=chunk_rt.x {
            for y in chunk_lb.y..=chunk_rt.y {
                let idx = Vector2i::new(x, y);
                if self.chunks.contains_key(&idx) {
                    indices.push(idx);
                }
            }
        }
        GridChunkIterator { indices, cursor: 0 }
    }

    pub fn get_anchored_entities(&self, pos: Vector2i) -> Vec<EntityUid> {
        let chunk_indices = self.grid_tile_to_chunk_indices(pos);
        let Some(chunk) = self.chunks.get(&chunk_indices) else {
            return Vec::new();
        };
        let chunk_tile = chunk.grid_tile_to_chunk_tile(pos);
        chunk.get_snap_grid_cell(chunk_tile.x as u16, chunk_tile.y as u16).to_vec()
    }

    pub fn add_to_snap_grid_cell(&mut self, pos: Vector2i, entity: EntityUid) -> bool {
        let (chunk, chunk_tile) = self.chunk_and_offset_for_tile(pos);
        if chunk.get_tile(chunk_tile.x as u16, chunk_tile.y as u16).is_empty() {
            return false;
        }
        chunk.add_to_snap_grid_cell(chunk_tile.x as u16, chunk_tile.y as u16, entity);
        true
    }

    pub fn remove_from_snap_grid_cell(&mut self, pos: Vector2i, entity: EntityUid) {
        let (chunk, chunk_tile) = self.chunk_and_offset_for_tile(pos);
        chunk.remove_from_snap_grid_cell(chunk_tile.x as u16, chunk_tile.y as u16, entity);
    }

    pub fn world_to_local(&self, pos_world: Vector2) -> Vector2 {
        self.inv_world_matrix() * pos_world
    }

    pub fn map_to_grid(&self, pos_world: MapCoordinates) -> EntityCoordinates {
        assert!(pos_world.map_id == self.parent_map_id, "map mismatch");
        EntityCoordinates::new(self.grid_entity_id, self.world_to_local(pos_world.position))
    }

    pub fn local_to_world(&self, pos_local: Vector2) -> Vector2 {
        self.world_matrix() * pos_local
    }

    pub fn world_to_tile(&self, pos_world: Vector2) -> Vector2i {
        let local = self.world_to_local(pos_world);
        Vector2i::new(
            (local.x / self.tile_size as f32).floor() as i32,
            (local.y / self.tile_size as f32).floor() as i32,
        )
    }

    pub fn coordinates_to_tile_map(&self, coords: MapCoordinates) -> Vector2i {
        assert!(coords.map_id == self.parent_map_id, "map mismatch");
        self.world_to_tile(coords.position)
    }

    pub fn coordinates_to_tile_entity(&self, coords: EntityCoordinates) -> Vector2i {
        let local = self.local_to_grid(coords);
        Vector2i::new(
            (local.x / self.tile_size as f32).floor() as i32,
            (local.y / self.tile_size as f32).floor() as i32,
        )
    }

    pub fn local_to_chunk_indices(&self, grid_pos: EntityCoordinates) -> Vector2i {
        let local = self.local_to_grid(grid_pos);
        Vector2i::new(
            (local.x / (self.tile_size * self.chunk_size) as f32).floor() as i32,
            (local.y / (self.tile_size * self.chunk_size) as f32).floor() as i32,
        )
    }

    pub fn local_to_grid(&self, position: EntityCoordinates) -> Vector2 {
        if position.entity_id == self.grid_entity_id {
            position.position
        } else {
            self.world_to_local(position.position)
        }
    }

    pub fn collides_with_grid(&self, indices: Vector2i) -> bool {
        let chunk_indices = self.grid_tile_to_chunk_indices(indices);
        let Some(chunk) = self.chunks.get(&chunk_indices) else {
            return false;
        };
        let chunk_tile = chunk.grid_tile_to_chunk_tile(indices);
        !chunk.get_tile(chunk_tile.x as u16, chunk_tile.y as u16).is_empty()
    }

    pub fn grid_tile_to_chunk_indices(&self, grid_tile: Vector2i) -> Vector2i {
        Vector2i::new(
            (grid_tile.x as f32 / self.chunk_size as f32).floor() as i32,
            (grid_tile.y as f32 / self.chunk_size as f32).floor() as i32,
        )
    }

    pub fn grid_tile_to_local(&self, grid_tile: Vector2i) -> EntityCoordinates {
        EntityCoordinates::new(
            self.grid_entity_id,
            Vector2::new(
                grid_tile.x as f32 * self.tile_size as f32 + self.tile_size as f32 / 2.0,
                grid_tile.y as f32 * self.tile_size as f32 + self.tile_size as f32 / 2.0,
            ),
        )
    }

    pub fn grid_tile_to_world_pos(&self, grid_tile: Vector2i) -> Vector2 {
        self.local_to_world(self.grid_tile_to_local(grid_tile).position)
    }

    pub fn grid_tile_to_world(&self, grid_tile: Vector2i) -> MapCoordinates {
        MapCoordinates::new(self.grid_tile_to_world_pos(grid_tile), self.parent_map_id)
    }

    pub fn try_get_tile_ref(&self, indices: Vector2i) -> Option<TileRef> {
        let chunk_indices = self.grid_tile_to_chunk_indices(indices);
        let chunk = self.chunks.get(&chunk_indices)?;
        let chunk_tile = chunk.grid_tile_to_chunk_tile(indices);
        Some(TileRef::new(
            self.parent_map_id,
            self.index,
            indices,
            chunk.get_tile(chunk_tile.x as u16, chunk_tile.y as u16),
        ))
    }

    fn chunk_and_offset_for_tile(&mut self, pos: Vector2i) -> (&mut MapChunk, Vector2i) {
        let chunk_indices = self.grid_tile_to_chunk_indices(pos);
        let chunk = self.get_chunk(chunk_indices);
        let chunk_tile = chunk.grid_tile_to_chunk_tile(pos);
        (chunk, chunk_tile)
    }

    fn recalculate_local_bounds(&mut self) {
        let mut result: Option<Box2> = None;
        for chunk in self.chunks.values() {
            if chunk.filled_tiles <= 0 {
                continue;
            }
            let tile_origin = chunk.indices() * self.chunk_size as i32;
            let chunk_bounds = Box2::from(chunk.cached_bounds.translated(tile_origin));
            result = Some(match result {
                Some(existing) => existing.union(chunk_bounds),
                None => chunk_bounds,
            });
        }
        self.local_bounds = result.unwrap_or_default();
    }
}

impl MapGridLike for MapGrid {
    fn parent_map_id(&self) -> MapId { self.parent_map_id }
    fn grid_entity_id(&self) -> EntityUid { self.grid_entity_id }
    fn index(&self) -> GridId { self.index }
    fn tile_size(&self) -> u16 { self.tile_size }
    fn chunk_size(&self) -> u16 { self.chunk_size }
    fn world_position(&self) -> Vector2 { self.world_position }
    fn world_rotation(&self) -> Angle { self.world_rotation }
    fn world_matrix(&self) -> Matrix3 {
        Matrix3::create_transform(
            self.world_position.x,
            self.world_position.y,
            self.world_rotation.theta as f32,
            1.0,
            1.0,
        )
    }
    fn inv_world_matrix(&self) -> Matrix3 {
        Matrix3::create_inverse_transform(
            self.world_position.x,
            self.world_position.y,
            self.world_rotation.theta as f32,
            1.0,
            1.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::MapGrid;
    use crate::{EntityUid, GridId, MapCoordinates, MapId, Tile, TileRenderFlag};
    use keisan::{Angle, ApproxEq, Vector2, Vector2i};

    #[test]
    fn map_grid_tile_and_snapgrid_flow_works() {
        let mut grid = MapGrid::new(MapId::new(1), EntityUid::new(10), GridId::new(2), 4);
        grid.tile_size = 2;
        grid.set_tile(Vector2i::new(1, 1), Tile::new(3, TileRenderFlag(0), 0));
        let tile = grid.get_tile_ref(Vector2i::new(1, 1));
        assert_eq!(tile.tile.type_id, 3);
        assert!(grid.add_to_snap_grid_cell(Vector2i::new(1, 1), EntityUid::new(30)));
        assert_eq!(grid.get_anchored_entities(Vector2i::new(1, 1)), vec![EntityUid::new(30)]);
        assert!(grid.collides_with_grid(Vector2i::new(1, 1)));
    }

    #[test]
    fn map_grid_transforms_match_expected_shape() {
        let mut grid = MapGrid::new(MapId::new(1), EntityUid::new(10), GridId::new(2), 4);
        grid.world_position = Vector2::new(10.0, 20.0);
        grid.world_rotation = Angle::from_degrees(90.0);
        let world = grid.grid_tile_to_world(Vector2i::new(0, 0));
        assert_eq!(world.map_id, MapId::new(1));
        let back = grid.map_to_grid(MapCoordinates::new(world.position, world.map_id));
        assert!(back.position.approx_eq(Vector2::new(0.5, 0.5)));
    }

    #[test]
    fn map_grid_returns_sorted_chunk_indices() {
        let mut grid = MapGrid::new(MapId::new(1), EntityUid::new(10), GridId::new(2), 4);
        grid.set_tile(Vector2i::new(8, 0), Tile::new(1, TileRenderFlag(0), 0));
        grid.set_tile(Vector2i::new(-1, 0), Tile::new(2, TileRenderFlag(0), 0));
        grid.set_tile(Vector2i::new(0, 0), Tile::new(3, TileRenderFlag(0), 0));
        assert_eq!(
            grid.chunk_indices(),
            vec![Vector2i::new(-1, 0), Vector2i::new(0, 0), Vector2i::new(2, 0)]
        );
    }
}
