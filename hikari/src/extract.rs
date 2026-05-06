use std::collections::BTreeMap;
use std::fmt;

use keisan::{Box2, Color, Vector2, Vector2i};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtractCameraId(u64);

impl ExtractCameraId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtractTextureId(u64);

impl ExtractTextureId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtractTileId(u32);

impl ExtractTileId {
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Camera2dExtract {
    id: ExtractCameraId,
    label: String,
    world_view: Box2,
    viewport_size: Vector2,
    clear_color: Option<Color>,
}

impl Camera2dExtract {
    pub fn new(
        id: ExtractCameraId,
        label: impl Into<String>,
        world_view: Box2,
        viewport_size: Vector2,
    ) -> Self {
        Self {
            id,
            label: label.into(),
            world_view,
            viewport_size,
            clear_color: None,
        }
    }

    pub fn with_clear_color(mut self, color: Color) -> Self {
        self.clear_color = Some(color);
        self
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
pub struct SpriteExtract {
    camera: ExtractCameraId,
    texture: ExtractTextureId,
    position: Vector2,
    size: Vector2,
    rotation: f32,
    tint: Color,
    depth: f32,
}

impl SpriteExtract {
    pub fn new(
        camera: ExtractCameraId,
        texture: ExtractTextureId,
        position: Vector2,
        size: Vector2,
    ) -> Self {
        Self {
            camera,
            texture,
            position,
            size,
            rotation: 0.0,
            tint: Color::WHITE,
            depth: 0.0,
        }
    }

    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_tint(mut self, tint: Color) -> Self {
        self.tint = tint;
        self
    }

    pub fn with_depth(mut self, depth: f32) -> Self {
        self.depth = depth;
        self
    }

    pub const fn camera(&self) -> ExtractCameraId {
        self.camera
    }

    pub const fn texture(&self) -> ExtractTextureId {
        self.texture
    }

    pub const fn position(&self) -> Vector2 {
        self.position
    }

    pub const fn size(&self) -> Vector2 {
        self.size
    }

    pub const fn rotation(&self) -> f32 {
        self.rotation
    }

    pub const fn tint(&self) -> Color {
        self.tint
    }

    pub const fn depth(&self) -> f32 {
        self.depth
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TileExtract {
    position: Vector2i,
    tile: ExtractTileId,
    tint: Color,
}

impl TileExtract {
    pub fn new(position: Vector2i, tile: ExtractTileId) -> Self {
        Self {
            position,
            tile,
            tint: Color::WHITE,
        }
    }

    pub fn with_tint(mut self, tint: Color) -> Self {
        self.tint = tint;
        self
    }

    pub const fn position(&self) -> Vector2i {
        self.position
    }

    pub const fn tile(&self) -> ExtractTileId {
        self.tile
    }

    pub const fn tint(&self) -> Color {
        self.tint
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TileBatchExtract {
    camera: ExtractCameraId,
    texture: ExtractTextureId,
    origin: Vector2,
    tile_size: Vector2,
    depth: f32,
    tiles: Vec<TileExtract>,
}

impl TileBatchExtract {
    pub fn new(
        camera: ExtractCameraId,
        texture: ExtractTextureId,
        origin: Vector2,
        tile_size: Vector2,
    ) -> Self {
        Self {
            camera,
            texture,
            origin,
            tile_size,
            depth: 0.0,
            tiles: Vec::new(),
        }
    }

    pub fn with_depth(mut self, depth: f32) -> Self {
        self.depth = depth;
        self
    }

    pub fn push_tile(&mut self, tile: TileExtract) -> &mut Self {
        self.tiles.push(tile);
        self
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

    pub fn tiles(&self) -> &[TileExtract] {
        &self.tiles
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugLineExtract {
    camera: ExtractCameraId,
    start: Vector2,
    end: Vector2,
    color: Color,
    thickness: f32,
    depth: f32,
}

impl DebugLineExtract {
    pub fn new(camera: ExtractCameraId, start: Vector2, end: Vector2, color: Color) -> Self {
        Self {
            camera,
            start,
            end,
            color,
            thickness: 1.0,
            depth: 0.0,
        }
    }

    pub fn with_thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }

    pub fn with_depth(mut self, depth: f32) -> Self {
        self.depth = depth;
        self
    }

    pub const fn camera(&self) -> ExtractCameraId {
        self.camera
    }

    pub const fn start(&self) -> Vector2 {
        self.start
    }

    pub const fn end(&self) -> Vector2 {
        self.end
    }

    pub const fn color(&self) -> Color {
        self.color
    }

    pub const fn thickness(&self) -> f32 {
        self.thickness
    }

    pub const fn depth(&self) -> f32 {
        self.depth
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RenderExtract {
    cameras: BTreeMap<ExtractCameraId, Camera2dExtract>,
    sprites: Vec<SpriteExtract>,
    tile_batches: Vec<TileBatchExtract>,
    debug_lines: Vec<DebugLineExtract>,
}

impl RenderExtract {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_camera(&mut self, camera: Camera2dExtract) -> Result<&mut Self, RenderExtractError> {
        let id = camera.id();
        if self.cameras.contains_key(&id) {
            return Err(RenderExtractError::DuplicateCamera { camera: id });
        }
        self.cameras.insert(id, camera);
        Ok(self)
    }

    pub fn push_sprite(&mut self, sprite: SpriteExtract) -> &mut Self {
        self.sprites.push(sprite);
        self
    }

    pub fn push_tile_batch(&mut self, batch: TileBatchExtract) -> &mut Self {
        self.tile_batches.push(batch);
        self
    }

    pub fn push_debug_line(&mut self, line: DebugLineExtract) -> &mut Self {
        self.debug_lines.push(line);
        self
    }

    pub fn validate(&self) -> Result<RenderExtractValidation, Vec<RenderExtractError>> {
        let mut errors = Vec::new();

        if self.cameras.is_empty() {
            errors.push(RenderExtractError::NoCameras);
        }

        for camera in self.cameras.values() {
            validate_camera(camera, &mut errors);
        }
        for (index, sprite) in self.sprites.iter().enumerate() {
            validate_sprite(index, sprite, &self.cameras, &mut errors);
        }
        for (index, batch) in self.tile_batches.iter().enumerate() {
            validate_tile_batch(index, batch, &self.cameras, &mut errors);
        }
        for (index, line) in self.debug_lines.iter().enumerate() {
            validate_debug_line(index, line, &self.cameras, &mut errors);
        }

        if errors.is_empty() {
            Ok(RenderExtractValidation {
                camera_count: self.cameras.len(),
                sprite_count: self.sprites.len(),
                tile_batch_count: self.tile_batches.len(),
                tile_count: self
                    .tile_batches
                    .iter()
                    .map(|batch| batch.tiles.len())
                    .sum(),
                debug_line_count: self.debug_lines.len(),
            })
        } else {
            Err(errors)
        }
    }

    pub fn cameras(&self) -> &BTreeMap<ExtractCameraId, Camera2dExtract> {
        &self.cameras
    }

    pub fn sprites(&self) -> &[SpriteExtract] {
        &self.sprites
    }

    pub fn tile_batches(&self) -> &[TileBatchExtract] {
        &self.tile_batches
    }

    pub fn debug_lines(&self) -> &[DebugLineExtract] {
        &self.debug_lines
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderExtractValidation {
    camera_count: usize,
    sprite_count: usize,
    tile_batch_count: usize,
    tile_count: usize,
    debug_line_count: usize,
}

impl RenderExtractValidation {
    pub const fn camera_count(self) -> usize {
        self.camera_count
    }

    pub const fn sprite_count(self) -> usize {
        self.sprite_count
    }

    pub const fn tile_batch_count(self) -> usize {
        self.tile_batch_count
    }

    pub const fn tile_count(self) -> usize {
        self.tile_count
    }

    pub const fn debug_line_count(self) -> usize {
        self.debug_line_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderExtractError {
    NoCameras,
    DuplicateCamera {
        camera: ExtractCameraId,
    },
    InvalidCameraWorldView {
        camera: ExtractCameraId,
    },
    InvalidCameraViewport {
        camera: ExtractCameraId,
    },
    MissingCamera {
        primitive: &'static str,
        index: usize,
        camera: ExtractCameraId,
    },
    InvalidVector {
        primitive: &'static str,
        index: usize,
        field: &'static str,
    },
    InvalidColor {
        primitive: &'static str,
        index: usize,
        field: &'static str,
    },
    EmptyTileBatch {
        index: usize,
    },
}

impl fmt::Display for RenderExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCameras => f.write_str("render extract has no cameras"),
            Self::DuplicateCamera { camera } => write!(f, "duplicate camera {:?}", camera),
            Self::InvalidCameraWorldView { camera } => {
                write!(f, "camera {:?} has an invalid world view", camera)
            }
            Self::InvalidCameraViewport { camera } => {
                write!(f, "camera {:?} has an invalid viewport", camera)
            }
            Self::MissingCamera {
                primitive,
                index,
                camera,
            } => write!(
                f,
                "{primitive} {index} references missing camera {:?}",
                camera
            ),
            Self::InvalidVector {
                primitive,
                index,
                field,
            } => write!(f, "{primitive} {index} has invalid vector field {field}"),
            Self::InvalidColor {
                primitive,
                index,
                field,
            } => write!(f, "{primitive} {index} has invalid color field {field}"),
            Self::EmptyTileBatch { index } => write!(f, "tile batch {index} is empty"),
        }
    }
}

impl std::error::Error for RenderExtractError {}

fn validate_camera(camera: &Camera2dExtract, errors: &mut Vec<RenderExtractError>) {
    let view = camera.world_view();
    if !view.left.is_finite()
        || !view.bottom.is_finite()
        || !view.right.is_finite()
        || !view.top.is_finite()
        || view.right <= view.left
        || view.top <= view.bottom
    {
        errors.push(RenderExtractError::InvalidCameraWorldView {
            camera: camera.id(),
        });
    }

    if !is_positive_vector(camera.viewport_size()) {
        errors.push(RenderExtractError::InvalidCameraViewport {
            camera: camera.id(),
        });
    }

    if let Some(color) = camera.clear_color()
        && !is_finite_color(color)
    {
        errors.push(RenderExtractError::InvalidColor {
            primitive: "camera",
            index: camera.id().raw() as usize,
            field: "clear_color",
        });
    }
}

fn validate_sprite(
    index: usize,
    sprite: &SpriteExtract,
    cameras: &BTreeMap<ExtractCameraId, Camera2dExtract>,
    errors: &mut Vec<RenderExtractError>,
) {
    validate_camera_reference("sprite", index, sprite.camera(), cameras, errors);
    validate_vector("sprite", index, "position", sprite.position(), errors);
    validate_positive_vector("sprite", index, "size", sprite.size(), errors);
    validate_finite_scalar("sprite", index, "rotation", sprite.rotation(), errors);
    validate_finite_scalar("sprite", index, "depth", sprite.depth(), errors);
    validate_color("sprite", index, "tint", sprite.tint(), errors);
}

fn validate_tile_batch(
    index: usize,
    batch: &TileBatchExtract,
    cameras: &BTreeMap<ExtractCameraId, Camera2dExtract>,
    errors: &mut Vec<RenderExtractError>,
) {
    validate_camera_reference("tile batch", index, batch.camera(), cameras, errors);
    validate_vector("tile batch", index, "origin", batch.origin(), errors);
    validate_positive_vector("tile batch", index, "tile_size", batch.tile_size(), errors);
    validate_finite_scalar("tile batch", index, "depth", batch.depth(), errors);
    if batch.tiles().is_empty() {
        errors.push(RenderExtractError::EmptyTileBatch { index });
    }
    for tile in batch.tiles() {
        validate_color(
            "tile",
            tile.tile().raw() as usize,
            "tint",
            tile.tint(),
            errors,
        );
    }
}

fn validate_debug_line(
    index: usize,
    line: &DebugLineExtract,
    cameras: &BTreeMap<ExtractCameraId, Camera2dExtract>,
    errors: &mut Vec<RenderExtractError>,
) {
    validate_camera_reference("debug line", index, line.camera(), cameras, errors);
    validate_vector("debug line", index, "start", line.start(), errors);
    validate_vector("debug line", index, "end", line.end(), errors);
    validate_positive_scalar("debug line", index, "thickness", line.thickness(), errors);
    validate_finite_scalar("debug line", index, "depth", line.depth(), errors);
    validate_color("debug line", index, "color", line.color(), errors);
}

fn validate_camera_reference(
    primitive: &'static str,
    index: usize,
    camera: ExtractCameraId,
    cameras: &BTreeMap<ExtractCameraId, Camera2dExtract>,
    errors: &mut Vec<RenderExtractError>,
) {
    if !cameras.contains_key(&camera) {
        errors.push(RenderExtractError::MissingCamera {
            primitive,
            index,
            camera,
        });
    }
}

fn validate_vector(
    primitive: &'static str,
    index: usize,
    field: &'static str,
    vector: Vector2,
    errors: &mut Vec<RenderExtractError>,
) {
    if !is_finite_vector(vector) {
        errors.push(RenderExtractError::InvalidVector {
            primitive,
            index,
            field,
        });
    }
}

fn validate_positive_vector(
    primitive: &'static str,
    index: usize,
    field: &'static str,
    vector: Vector2,
    errors: &mut Vec<RenderExtractError>,
) {
    if !is_positive_vector(vector) {
        errors.push(RenderExtractError::InvalidVector {
            primitive,
            index,
            field,
        });
    }
}

fn validate_finite_scalar(
    primitive: &'static str,
    index: usize,
    field: &'static str,
    value: f32,
    errors: &mut Vec<RenderExtractError>,
) {
    if !value.is_finite() {
        errors.push(RenderExtractError::InvalidVector {
            primitive,
            index,
            field,
        });
    }
}

fn validate_positive_scalar(
    primitive: &'static str,
    index: usize,
    field: &'static str,
    value: f32,
    errors: &mut Vec<RenderExtractError>,
) {
    if !value.is_finite() || value <= 0.0 {
        errors.push(RenderExtractError::InvalidVector {
            primitive,
            index,
            field,
        });
    }
}

fn validate_color(
    primitive: &'static str,
    index: usize,
    field: &'static str,
    color: Color,
    errors: &mut Vec<RenderExtractError>,
) {
    if !is_finite_color(color) {
        errors.push(RenderExtractError::InvalidColor {
            primitive,
            index,
            field,
        });
    }
}

fn is_finite_vector(vector: Vector2) -> bool {
    vector.x.is_finite() && vector.y.is_finite()
}

fn is_positive_vector(vector: Vector2) -> bool {
    is_finite_vector(vector) && vector.x > 0.0 && vector.y > 0.0
}

fn is_finite_color(color: Color) -> bool {
    color.r.is_finite() && color.g.is_finite() && color.b.is_finite() && color.a.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera_id() -> ExtractCameraId {
        ExtractCameraId::new(1)
    }

    fn texture_id() -> ExtractTextureId {
        ExtractTextureId::new(7)
    }

    fn camera() -> Camera2dExtract {
        Camera2dExtract::new(
            camera_id(),
            "main",
            Box2::centered_around(Vector2::ZERO, Vector2::new(16.0, 9.0)),
            Vector2::new(1280.0, 720.0),
        )
    }

    #[test]
    fn render_extract_validates_basic_2d_frame() {
        let mut extract = RenderExtract::new();
        extract.add_camera(camera()).expect("camera added");
        extract.push_sprite(
            SpriteExtract::new(
                camera_id(),
                texture_id(),
                Vector2::new(1.0, 2.0),
                Vector2::new(0.5, 0.5),
            )
            .with_depth(3.0),
        );

        let mut tiles = TileBatchExtract::new(
            camera_id(),
            texture_id(),
            Vector2::ZERO,
            Vector2::new(1.0, 1.0),
        );
        tiles.push_tile(TileExtract::new(
            Vector2i::new(2, 4),
            ExtractTileId::new(12),
        ));
        extract.push_tile_batch(tiles);
        extract.push_debug_line(DebugLineExtract::new(
            camera_id(),
            Vector2::ZERO,
            Vector2::ONE,
            Color::GREEN,
        ));

        let validation = extract.validate().expect("valid extract");

        assert_eq!(validation.camera_count(), 1);
        assert_eq!(validation.sprite_count(), 1);
        assert_eq!(validation.tile_batch_count(), 1);
        assert_eq!(validation.tile_count(), 1);
        assert_eq!(validation.debug_line_count(), 1);
    }

    #[test]
    fn render_extract_rejects_duplicate_camera_ids() {
        let mut extract = RenderExtract::new();
        extract.add_camera(camera()).expect("camera added");

        let err = extract
            .add_camera(camera())
            .expect_err("duplicate rejected");

        assert_eq!(
            err,
            RenderExtractError::DuplicateCamera {
                camera: camera_id()
            }
        );
    }

    #[test]
    fn render_extract_rejects_missing_cameras_for_primitives() {
        let mut extract = RenderExtract::new();
        extract.push_sprite(SpriteExtract::new(
            camera_id(),
            texture_id(),
            Vector2::ZERO,
            Vector2::ONE,
        ));

        let errors = extract.validate().expect_err("extract should fail");

        assert!(errors.contains(&RenderExtractError::NoCameras));
        assert!(errors.contains(&RenderExtractError::MissingCamera {
            primitive: "sprite",
            index: 0,
            camera: camera_id(),
        }));
    }

    #[test]
    fn render_extract_rejects_invalid_camera_and_primitives() {
        let mut extract = RenderExtract::new();
        extract
            .add_camera(Camera2dExtract::new(
                camera_id(),
                "bad",
                Box2::new(1.0, 1.0, 1.0, 2.0),
                Vector2::new(0.0, 720.0),
            ))
            .expect("camera inserted");
        extract.push_sprite(SpriteExtract::new(
            camera_id(),
            texture_id(),
            Vector2::new(f32::NAN, 0.0),
            Vector2::new(-1.0, 1.0),
        ));
        extract.push_debug_line(
            DebugLineExtract::new(camera_id(), Vector2::ZERO, Vector2::ONE, Color::WHITE)
                .with_thickness(0.0),
        );
        extract.push_tile_batch(TileBatchExtract::new(
            camera_id(),
            texture_id(),
            Vector2::ZERO,
            Vector2::new(1.0, 1.0),
        ));

        let errors = extract.validate().expect_err("extract should fail");

        assert!(
            errors.contains(&RenderExtractError::InvalidCameraWorldView {
                camera: camera_id(),
            })
        );
        assert!(errors.contains(&RenderExtractError::InvalidCameraViewport {
            camera: camera_id(),
        }));
        assert!(errors.contains(&RenderExtractError::InvalidVector {
            primitive: "sprite",
            index: 0,
            field: "position",
        }));
        assert!(errors.contains(&RenderExtractError::InvalidVector {
            primitive: "sprite",
            index: 0,
            field: "size",
        }));
        assert!(errors.contains(&RenderExtractError::InvalidVector {
            primitive: "debug line",
            index: 0,
            field: "thickness",
        }));
        assert!(errors.contains(&RenderExtractError::EmptyTileBatch { index: 0 }));
    }
}
