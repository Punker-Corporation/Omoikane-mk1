use std::cmp::Ordering;
use std::fmt;

use keisan::{Box2, Color, Vector2, Vector2i};

use crate::{
    Camera2dExtract, DebugLineExtract, ExtractCameraId, ExtractTextureId, ExtractTileId,
    RenderExtract, RenderExtractError, SpriteExtract, TileBatchExtract,
};

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedFrame {
    cameras: Vec<PreparedCamera2d>,
    sprite_batches: Vec<PreparedSpriteBatch>,
    tile_batches: Vec<PreparedTileBatch>,
    debug_line_batches: Vec<PreparedDebugLineBatch>,
}

impl PreparedFrame {
    pub fn from_extract(extract: &RenderExtract) -> Result<Self, Vec<PrepareFrameError>> {
        if let Err(errors) = extract.validate() {
            return Err(errors
                .into_iter()
                .map(PrepareFrameError::InvalidExtract)
                .collect());
        }

        let cameras = extract
            .cameras()
            .values()
            .map(PreparedCamera2d::from_extract)
            .collect();
        let sprite_batches = prepare_sprite_batches(extract.sprites());
        let tile_batches = extract
            .tile_batches()
            .iter()
            .map(PreparedTileBatch::from_extract)
            .collect();
        let debug_line_batches = prepare_debug_line_batches(extract.debug_lines());

        Ok(Self {
            cameras,
            sprite_batches,
            tile_batches,
            debug_line_batches,
        })
    }

    pub fn queue(&self) -> QueuedFrame {
        QueuedFrame::from_prepared(self)
    }

    pub fn cameras(&self) -> &[PreparedCamera2d] {
        &self.cameras
    }

    pub fn sprite_batches(&self) -> &[PreparedSpriteBatch] {
        &self.sprite_batches
    }

    pub fn tile_batches(&self) -> &[PreparedTileBatch] {
        &self.tile_batches
    }

    pub fn debug_line_batches(&self) -> &[PreparedDebugLineBatch] {
        &self.debug_line_batches
    }

    pub fn primitive_count(&self) -> usize {
        self.sprite_batches
            .iter()
            .map(|batch| batch.instances.len())
            .sum::<usize>()
            + self
                .tile_batches
                .iter()
                .map(|batch| batch.tiles.len())
                .sum::<usize>()
            + self
                .debug_line_batches
                .iter()
                .map(|batch| batch.lines.len())
                .sum::<usize>()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedCamera2d {
    id: ExtractCameraId,
    label: String,
    world_view: Box2,
    viewport_size: Vector2,
    clear_color: Option<Color>,
}

impl PreparedCamera2d {
    fn from_extract(camera: &Camera2dExtract) -> Self {
        Self {
            id: camera.id(),
            label: camera.label().to_string(),
            world_view: camera.world_view(),
            viewport_size: camera.viewport_size(),
            clear_color: camera.clear_color(),
        }
    }

    pub const fn id(&self) -> ExtractCameraId {
        self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn world_view(&self) -> Box2 {
        self.world_view
    }

    pub const fn viewport_size(&self) -> Vector2 {
        self.viewport_size
    }

    pub const fn clear_color(&self) -> Option<Color> {
        self.clear_color
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedSpriteBatch {
    camera: ExtractCameraId,
    texture: ExtractTextureId,
    depth: f32,
    instances: Vec<PreparedSpriteInstance>,
}

impl PreparedSpriteBatch {
    pub const fn camera(&self) -> ExtractCameraId {
        self.camera
    }

    pub const fn texture(&self) -> ExtractTextureId {
        self.texture
    }

    pub const fn depth(&self) -> f32 {
        self.depth
    }

    pub fn instances(&self) -> &[PreparedSpriteInstance] {
        &self.instances
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedSpriteInstance {
    position: Vector2,
    size: Vector2,
    rotation: f32,
    tint: Color,
}

impl PreparedSpriteInstance {
    fn from_extract(sprite: &SpriteExtract) -> Self {
        Self {
            position: sprite.position(),
            size: sprite.size(),
            rotation: sprite.rotation(),
            tint: sprite.tint(),
        }
    }

    pub const fn position(self) -> Vector2 {
        self.position
    }

    pub const fn size(self) -> Vector2 {
        self.size
    }

    pub const fn rotation(self) -> f32 {
        self.rotation
    }

    pub const fn tint(self) -> Color {
        self.tint
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedTileBatch {
    camera: ExtractCameraId,
    texture: ExtractTextureId,
    origin: Vector2,
    tile_size: Vector2,
    depth: f32,
    tiles: Vec<PreparedTile>,
}

impl PreparedTileBatch {
    fn from_extract(batch: &TileBatchExtract) -> Self {
        Self {
            camera: batch.camera(),
            texture: batch.texture(),
            origin: batch.origin(),
            tile_size: batch.tile_size(),
            depth: batch.depth(),
            tiles: batch
                .tiles()
                .iter()
                .map(|tile| PreparedTile {
                    position: tile.position(),
                    tile: tile.tile(),
                    tint: tile.tint(),
                })
                .collect(),
        }
    }

    pub const fn camera(&self) -> ExtractCameraId {
        self.camera
    }

    pub const fn texture(&self) -> ExtractTextureId {
        self.texture
    }

    pub const fn origin(&self) -> Vector2 {
        self.origin
    }

    pub const fn tile_size(&self) -> Vector2 {
        self.tile_size
    }

    pub const fn depth(&self) -> f32 {
        self.depth
    }

    pub fn tiles(&self) -> &[PreparedTile] {
        &self.tiles
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedTile {
    position: Vector2i,
    tile: ExtractTileId,
    tint: Color,
}

impl PreparedTile {
    pub const fn position(self) -> Vector2i {
        self.position
    }

    pub const fn tile(self) -> ExtractTileId {
        self.tile
    }

    pub const fn tint(self) -> Color {
        self.tint
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedDebugLineBatch {
    camera: ExtractCameraId,
    depth: f32,
    color: Color,
    thickness: f32,
    lines: Vec<PreparedDebugLine>,
}

impl PreparedDebugLineBatch {
    pub const fn camera(&self) -> ExtractCameraId {
        self.camera
    }

    pub const fn depth(&self) -> f32 {
        self.depth
    }

    pub const fn color(&self) -> Color {
        self.color
    }

    pub const fn thickness(&self) -> f32 {
        self.thickness
    }

    pub fn lines(&self) -> &[PreparedDebugLine] {
        &self.lines
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedDebugLine {
    start: Vector2,
    end: Vector2,
}

impl PreparedDebugLine {
    fn from_extract(line: &DebugLineExtract) -> Self {
        Self {
            start: line.start(),
            end: line.end(),
        }
    }

    pub const fn start(self) -> Vector2 {
        self.start
    }

    pub const fn end(self) -> Vector2 {
        self.end
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueuedFrame {
    draws: Vec<QueuedDraw>,
}

impl QueuedFrame {
    pub fn from_prepared(frame: &PreparedFrame) -> Self {
        let mut draws = Vec::new();

        for batch in frame.sprite_batches() {
            draws.push(QueuedDraw {
                camera: batch.camera(),
                primitive: QueuedPrimitive::Sprite,
                texture: Some(batch.texture()),
                depth: batch.depth(),
                instances: batch.instances().len(),
            });
        }
        for batch in frame.tile_batches() {
            draws.push(QueuedDraw {
                camera: batch.camera(),
                primitive: QueuedPrimitive::Tile,
                texture: Some(batch.texture()),
                depth: batch.depth(),
                instances: batch.tiles().len(),
            });
        }
        for batch in frame.debug_line_batches() {
            draws.push(QueuedDraw {
                camera: batch.camera(),
                primitive: QueuedPrimitive::DebugLine,
                texture: None,
                depth: batch.depth(),
                instances: batch.lines().len(),
            });
        }

        draws.sort_by(compare_queued_draws);

        Self { draws }
    }

    pub fn draws(&self) -> &[QueuedDraw] {
        &self.draws
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QueuedDraw {
    camera: ExtractCameraId,
    primitive: QueuedPrimitive,
    texture: Option<ExtractTextureId>,
    depth: f32,
    instances: usize,
}

impl QueuedDraw {
    pub const fn camera(self) -> ExtractCameraId {
        self.camera
    }

    pub const fn primitive(self) -> QueuedPrimitive {
        self.primitive
    }

    pub const fn texture(self) -> Option<ExtractTextureId> {
        self.texture
    }

    pub const fn depth(self) -> f32 {
        self.depth
    }

    pub const fn instances(self) -> usize {
        self.instances
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueuedPrimitive {
    Sprite,
    Tile,
    DebugLine,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepareFrameError {
    InvalidExtract(RenderExtractError),
}

impl fmt::Display for PrepareFrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidExtract(err) => write!(f, "invalid render extract: {err}"),
        }
    }
}

impl std::error::Error for PrepareFrameError {}

fn prepare_sprite_batches(sprites: &[SpriteExtract]) -> Vec<PreparedSpriteBatch> {
    let mut batches = Vec::<PreparedSpriteBatch>::new();

    for sprite in sprites {
        if let Some(batch) = batches.iter_mut().find(|batch| {
            batch.camera == sprite.camera()
                && batch.texture == sprite.texture()
                && batch.depth.to_bits() == sprite.depth().to_bits()
        }) {
            batch
                .instances
                .push(PreparedSpriteInstance::from_extract(sprite));
            continue;
        }

        batches.push(PreparedSpriteBatch {
            camera: sprite.camera(),
            texture: sprite.texture(),
            depth: sprite.depth(),
            instances: vec![PreparedSpriteInstance::from_extract(sprite)],
        });
    }

    batches
}

fn prepare_debug_line_batches(lines: &[DebugLineExtract]) -> Vec<PreparedDebugLineBatch> {
    let mut batches = Vec::<PreparedDebugLineBatch>::new();

    for line in lines {
        if let Some(batch) = batches.iter_mut().find(|batch| {
            batch.camera == line.camera()
                && batch.depth.to_bits() == line.depth().to_bits()
                && batch.thickness.to_bits() == line.thickness().to_bits()
                && batch.color == line.color()
        }) {
            batch.lines.push(PreparedDebugLine::from_extract(line));
            continue;
        }

        batches.push(PreparedDebugLineBatch {
            camera: line.camera(),
            depth: line.depth(),
            color: line.color(),
            thickness: line.thickness(),
            lines: vec![PreparedDebugLine::from_extract(line)],
        });
    }

    batches
}

fn compare_queued_draws(left: &QueuedDraw, right: &QueuedDraw) -> Ordering {
    left.camera
        .cmp(&right.camera)
        .then_with(|| {
            left.depth
                .partial_cmp(&right.depth)
                .unwrap_or(Ordering::Equal)
        })
        .then_with(|| left.primitive.cmp(&right.primitive))
        .then_with(|| left.texture.cmp(&right.texture))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExtractTileId, TileExtract};

    fn camera_id() -> ExtractCameraId {
        ExtractCameraId::new(1)
    }

    fn second_camera_id() -> ExtractCameraId {
        ExtractCameraId::new(2)
    }

    fn texture_id() -> ExtractTextureId {
        ExtractTextureId::new(10)
    }

    fn second_texture_id() -> ExtractTextureId {
        ExtractTextureId::new(11)
    }

    fn camera(id: ExtractCameraId) -> Camera2dExtract {
        Camera2dExtract::new(
            id,
            "camera",
            Box2::centered_around(Vector2::ZERO, Vector2::new(16.0, 9.0)),
            Vector2::new(1280.0, 720.0),
        )
    }

    #[test]
    fn prepared_frame_batches_extract_primitives() {
        let mut extract = RenderExtract::new();
        extract.add_camera(camera(camera_id())).expect("camera");
        extract
            .add_camera(camera(second_camera_id()))
            .expect("second camera");
        extract.push_sprite(SpriteExtract::new(
            camera_id(),
            texture_id(),
            Vector2::ZERO,
            Vector2::ONE,
        ));
        extract.push_sprite(SpriteExtract::new(
            camera_id(),
            texture_id(),
            Vector2::ONE,
            Vector2::ONE,
        ));
        extract.push_sprite(SpriteExtract::new(
            camera_id(),
            second_texture_id(),
            Vector2::ZERO,
            Vector2::ONE,
        ));

        let mut tiles = TileBatchExtract::new(
            second_camera_id(),
            texture_id(),
            Vector2::ZERO,
            Vector2::ONE,
        );
        tiles.push_tile(TileExtract::new(Vector2i::new(0, 0), ExtractTileId::new(1)));
        extract.push_tile_batch(tiles);
        extract.push_debug_line(DebugLineExtract::new(
            camera_id(),
            Vector2::ZERO,
            Vector2::ONE,
            Color::GREEN,
        ));

        let prepared = PreparedFrame::from_extract(&extract).expect("prepared frame");

        assert_eq!(prepared.cameras().len(), 2);
        assert_eq!(prepared.sprite_batches().len(), 2);
        assert_eq!(prepared.sprite_batches()[0].instances().len(), 2);
        assert_eq!(prepared.tile_batches().len(), 1);
        assert_eq!(prepared.debug_line_batches().len(), 1);
        assert_eq!(prepared.primitive_count(), 5);
    }

    #[test]
    fn prepared_frame_reports_extract_errors() {
        let mut extract = RenderExtract::new();
        extract.push_sprite(SpriteExtract::new(
            camera_id(),
            texture_id(),
            Vector2::ZERO,
            Vector2::ONE,
        ));

        let errors = PreparedFrame::from_extract(&extract).expect_err("invalid extract");

        assert!(errors.contains(&PrepareFrameError::InvalidExtract(
            RenderExtractError::NoCameras,
        )));
    }

    #[test]
    fn queued_frame_orders_draws_by_camera_depth_and_primitive() {
        let mut extract = RenderExtract::new();
        extract.add_camera(camera(camera_id())).expect("camera");
        extract.push_debug_line(
            DebugLineExtract::new(camera_id(), Vector2::ZERO, Vector2::ONE, Color::GREEN)
                .with_depth(3.0),
        );
        extract.push_sprite(
            SpriteExtract::new(camera_id(), texture_id(), Vector2::ZERO, Vector2::ONE)
                .with_depth(2.0),
        );
        let mut tiles =
            TileBatchExtract::new(camera_id(), texture_id(), Vector2::ZERO, Vector2::ONE)
                .with_depth(2.0);
        tiles.push_tile(TileExtract::new(Vector2i::new(0, 0), ExtractTileId::new(1)));
        extract.push_tile_batch(tiles);

        let queued = PreparedFrame::from_extract(&extract)
            .expect("prepared")
            .queue();

        let primitives = queued
            .draws()
            .iter()
            .map(|draw| (draw.depth(), draw.primitive(), draw.instances()))
            .collect::<Vec<_>>();
        assert_eq!(
            primitives,
            vec![
                (2.0, QueuedPrimitive::Sprite, 1),
                (2.0, QueuedPrimitive::Tile, 1),
                (3.0, QueuedPrimitive::DebugLine, 1),
            ],
        );
    }
}
