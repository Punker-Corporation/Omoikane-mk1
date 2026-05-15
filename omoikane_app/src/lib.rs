use butsuri::{AabbShape, BodyType, CircleShape, Fixture, PhysShape};
use daikoku::{BoundKeyFunction, BoundKeyState, DaikokuServer, ServerOptions, ServerState};
use hikari::{
    BindGroupLayoutId, BufferUsage, Camera2dExtract, CommandListId, DrawCall, ExtractCameraId,
    ExtractTextureId, FrameContext, FrameId, FrameSubmission, GpuBuffer, GpuBufferDescriptor,
    GpuBufferId, GpuTexture, GpuTextureDescriptor, GpuTextureId, GraphicsDevice, GraphicsDeviceId,
    GraphicsInstance, GraphicsInstanceId, GraphicsResourceCatalog, GraphicsResourceError,
    PipelineShader, PreparedFrame, QueuedFrame, RenderCommandList, RenderExtract,
    RenderExtractError, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    RenderPipelineId, ShaderModule, ShaderModuleDescriptor, ShaderModuleId, ShaderSourceKind,
    ShaderStage, SpriteExtract, SurfaceSize, SurfaceTarget, SurfaceTargetId, TextureFormat,
    TextureSize, TextureUsage, VertexBufferLayout, VertexStepMode,
};
use jikan::GameTick;
use keisan::{Angle, Box2, Color, Vector2};
use sekai::{EntityUid, GridId, MapId, TransformComponentState};
use serde::{Deserialize, Serialize};
use shinobi::{BaseClient, ClientOptions, ClientRunLevel};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq)]
pub struct HeadlessAppOptions {
    pub user_id: String,
    pub username: String,
    pub server: ServerOptions,
    pub client: ClientOptions,
    pub fixed_delta_seconds: f32,
}

pub type AppOptions = HeadlessAppOptions;

impl Default for HeadlessAppOptions {
    fn default() -> Self {
        let username = "local".to_string();
        Self {
            user_id: "local".to_string(),
            username: username.clone(),
            server: ServerOptions {
                server_name: "Omoikane Headless".to_string(),
                ..ServerOptions::default()
            },
            client: ClientOptions { username },
            fixed_delta_seconds: 1.0 / 60.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppState {
    pub started: bool,
    pub ticks_run: u64,
    pub current_tick: GameTick,
    pub server_state: ServerState,
    pub client_run_level: ClientRunLevel,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SandboxEntityOptions {
    pub prototype: Option<String>,
    pub position: Vector2,
    pub rotation: f32,
    pub appearance_name: String,
    pub attach_local_player: bool,
}

impl Default for SandboxEntityOptions {
    fn default() -> Self {
        Self {
            prototype: Some("sandbox_visual".to_string()),
            position: Vector2::ZERO,
            rotation: 0.0,
            appearance_name: "sandbox_visual".to_string(),
            attach_local_player: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SandboxEntity {
    pub entity: EntityUid,
    pub position: Vector2,
    pub rotation: f32,
    pub appearance_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectSceneEntity {
    pub scene_name: String,
    pub id: String,
    pub entity: EntityUid,
    pub appearance_name: String,
    pub attach_local_player: bool,
    pub physics: Option<ProjectScenePhysicsConfig>,
    pub rotation: f32,
    pub texture: ExtractTextureId,
    pub size: Vector2,
    pub tint: Color,
    pub depth: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderFrameOptions {
    pub camera: ExtractCameraId,
    pub sandbox_texture: ExtractTextureId,
    pub world_view: Box2,
    pub viewport_size: Vector2,
    pub sprite_size: Vector2,
    pub sprite_tint: Color,
    pub sprite_depth: f32,
}

impl Default for RenderFrameOptions {
    fn default() -> Self {
        Self {
            camera: ExtractCameraId::new(1),
            sandbox_texture: ExtractTextureId::new(1),
            world_view: Box2::centered_around(Vector2::ZERO, Vector2::new(16.0, 9.0)),
            viewport_size: Vector2::new(1280.0, 720.0),
            sprite_size: Vector2::ONE,
            sprite_tint: Color::WHITE,
            sprite_depth: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuFrameOptions {
    pub instance: GraphicsInstanceId,
    pub device: GraphicsDeviceId,
    pub surface: SurfaceTargetId,
    pub frame: FrameId,
    pub target: GpuTextureId,
    pub vertices: GpuBufferId,
    pub vertex_shader: ShaderModuleId,
    pub fragment_shader: ShaderModuleId,
    pub pipeline: RenderPipelineId,
    pub commands: CommandListId,
    pub surface_size: SurfaceSize,
}

impl Default for CpuFrameOptions {
    fn default() -> Self {
        Self {
            instance: GraphicsInstanceId::new(1),
            device: GraphicsDeviceId::new(2),
            surface: SurfaceTargetId::new(3),
            frame: FrameId::new(4),
            target: GpuTextureId::new(5),
            vertices: GpuBufferId::new(6),
            vertex_shader: ShaderModuleId::new(7),
            fragment_shader: ShaderModuleId::new(8),
            pipeline: RenderPipelineId::new(9),
            commands: CommandListId::new(10),
            surface_size: SurfaceSize::new(1280, 720),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OmoikaneProjectConfig {
    pub name: String,
    #[serde(default)]
    pub resources: ProjectResourceConfig,
    #[serde(default)]
    pub scenes: Vec<ProjectSceneConfig>,
}

impl OmoikaneProjectConfig {
    pub fn from_json_str(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json_string_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn to_cpu_frame_resource_config(
        &self,
        frame: CpuFrameOptions,
    ) -> Result<CpuFrameResourceConfig, ProjectConfigError> {
        let mut texture_ids = BTreeSet::new();
        let mut pipeline_ids = BTreeSet::new();
        let mut resource_config = CpuFrameResourceConfig::new(frame);
        for texture in &self.resources.textures {
            if !texture_ids.insert(texture.id) {
                return Err(ProjectConfigError::DuplicateTextureId(texture.id));
            }
            resource_config = resource_config.with_texture(texture.to_cpu_config()?);
        }
        for pipeline in &self.resources.render_pipelines {
            if !pipeline_ids.insert(pipeline.id) {
                return Err(ProjectConfigError::DuplicateRenderPipelineId(pipeline.id));
            }
            resource_config = resource_config.with_render_pipeline(pipeline.to_cpu_config());
        }
        Ok(resource_config)
    }

    pub fn render_frame_options_for_scene(
        &self,
        name: &str,
    ) -> Result<RenderFrameOptions, ProjectConfigError> {
        self.scene(name).map(ProjectSceneConfig::to_render_options)
    }

    pub fn scene(&self, name: &str) -> Result<&ProjectSceneConfig, ProjectConfigError> {
        let mut matched = None;
        for scene in &self.scenes {
            if scene.name == name {
                if matched.is_some() {
                    return Err(ProjectConfigError::DuplicateSceneName(name.to_string()));
                }
                matched = Some(scene);
            }
        }
        matched.ok_or_else(|| ProjectConfigError::MissingScene(name.to_string()))
    }
}

impl Default for OmoikaneProjectConfig {
    fn default() -> Self {
        Self {
            name: "Omoikane Headless Project".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProjectResourceConfig {
    #[serde(default)]
    pub textures: Vec<ProjectTextureConfig>,
    #[serde(default)]
    pub render_pipelines: Vec<ProjectRenderPipelineConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectSceneConfig {
    pub name: String,
    pub camera: u64,
    pub sandbox_texture: u64,
    pub world_view: Box2,
    pub viewport_size: Vector2,
    #[serde(default)]
    pub controlled_entity: Option<String>,
    #[serde(default)]
    pub input_bindings: Vec<ProjectSceneInputBindingConfig>,
    #[serde(default = "ProjectSceneConfig::default_sprite_size")]
    pub sprite_size: Vector2,
    #[serde(default)]
    pub sprite_tint: ProjectColorConfig,
    #[serde(default)]
    pub sprite_depth: f32,
    #[serde(default)]
    pub sprites: Vec<ProjectSpriteConfig>,
    #[serde(default)]
    pub dynamic_entities: Vec<ProjectSceneEntityConfig>,
}

impl ProjectSceneConfig {
    const fn default_sprite_size() -> Vector2 {
        Vector2::ONE
    }

    pub fn to_render_options(&self) -> RenderFrameOptions {
        RenderFrameOptions {
            camera: ExtractCameraId::new(self.camera),
            sandbox_texture: ExtractTextureId::new(self.sandbox_texture),
            world_view: self.world_view,
            viewport_size: self.viewport_size,
            sprite_size: self.sprite_size,
            sprite_tint: self.sprite_tint.into(),
            sprite_depth: self.sprite_depth,
        }
    }

    pub fn sprite_extracts(&self) -> Vec<SpriteExtract> {
        self.sprites
            .iter()
            .map(|sprite| sprite.to_sprite_extract(ExtractCameraId::new(self.camera)))
            .collect()
    }

    pub fn validate_render_data(&self) -> Result<(), ProjectConfigError> {
        let invalid = |reason: &str| ProjectConfigError::InvalidSceneRenderData {
            scene: self.name.clone(),
            reason: reason.to_string(),
        };
        if !is_finite_box(self.world_view) {
            return Err(invalid("world view must be finite"));
        }
        if self.world_view.right <= self.world_view.left
            || self.world_view.top <= self.world_view.bottom
        {
            return Err(invalid("world view must have positive width and height"));
        }
        if !is_positive_finite_vector(self.viewport_size) {
            return Err(invalid("viewport size must be finite and positive"));
        }
        if !is_positive_finite_vector(self.sprite_size) {
            return Err(invalid("sandbox sprite size must be finite and positive"));
        }
        if !is_finite_color(self.sprite_tint) {
            return Err(invalid("sandbox sprite tint must be finite"));
        }
        if !self.sprite_depth.is_finite() {
            return Err(invalid("sandbox sprite depth must be finite"));
        }
        for (index, sprite) in self.sprites.iter().enumerate() {
            sprite.validate_visuals(&self.name, index)?;
        }
        Ok(())
    }

    pub fn input_function_for_action(
        &self,
        action: &str,
    ) -> Result<BoundKeyFunction, ProjectConfigError> {
        self.validate_input_bindings()?;
        for binding in &self.input_bindings {
            if binding.action == action {
                return Ok(BoundKeyFunction::new(binding.function.clone()));
            }
        }
        Err(ProjectConfigError::MissingSceneInputAction {
            scene: self.name.clone(),
            action: action.to_string(),
        })
    }

    pub fn validate_input_bindings(&self) -> Result<(), ProjectConfigError> {
        let mut actions = BTreeSet::new();
        for binding in &self.input_bindings {
            if binding.action.is_empty() {
                return Err(ProjectConfigError::EmptySceneInputAction {
                    scene: self.name.clone(),
                });
            }
            if binding.function.is_empty() {
                return Err(ProjectConfigError::EmptySceneInputFunction {
                    scene: self.name.clone(),
                    action: binding.action.clone(),
                });
            }
            if !actions.insert(binding.action.clone()) {
                return Err(ProjectConfigError::DuplicateSceneInputAction {
                    scene: self.name.clone(),
                    action: binding.action.clone(),
                });
            }
        }
        Ok(())
    }

    pub fn validate_dynamic_entities(&self) -> Result<(), ProjectConfigError> {
        let mut entity_ids = BTreeSet::new();
        for entity_config in &self.dynamic_entities {
            if entity_config.id.is_empty() {
                return Err(ProjectConfigError::EmptySceneEntityId {
                    scene: self.name.clone(),
                });
            }
            if !entity_ids.insert(entity_config.id.clone()) {
                return Err(ProjectConfigError::DuplicateSceneEntityId {
                    scene: self.name.clone(),
                    id: entity_config.id.clone(),
                });
            }
            entity_config.validate_metadata(&self.name)?;
            entity_config.validate_visuals(&self.name)?;
            if let Some(physics) = &entity_config.physics {
                physics.validate(&self.name, &entity_config.id)?;
            }
        }
        if let Some(controlled_entity) = &self.controlled_entity {
            if controlled_entity.is_empty() {
                return Err(ProjectConfigError::EmptyControlledSceneEntityId {
                    scene: self.name.clone(),
                });
            }
            if !entity_ids.contains(controlled_entity) {
                return Err(ProjectConfigError::MissingSceneEntityId {
                    scene: self.name.clone(),
                    id: controlled_entity.clone(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectSceneInputBindingConfig {
    pub action: String,
    pub function: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectSceneEntityConfig {
    pub id: String,
    pub appearance_name: String,
    pub texture: u64,
    pub position: Vector2,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub prototype: Option<String>,
    #[serde(default)]
    pub attach_local_player: bool,
    #[serde(default)]
    pub physics: Option<ProjectScenePhysicsConfig>,
    #[serde(default = "ProjectSceneEntityConfig::default_size")]
    pub size: Vector2,
    #[serde(default)]
    pub tint: ProjectColorConfig,
    #[serde(default)]
    pub depth: f32,
}

impl ProjectSceneEntityConfig {
    const fn default_size() -> Vector2 {
        Vector2::ONE
    }

    fn validate_metadata(&self, scene: &str) -> Result<(), ProjectConfigError> {
        let invalid = |reason: &str| ProjectConfigError::InvalidSceneEntityMetadata {
            scene: scene.to_string(),
            entity: self.id.clone(),
            reason: reason.to_string(),
        };
        if self.appearance_name.is_empty() {
            return Err(invalid("appearance name must not be empty"));
        }
        if self
            .prototype
            .as_ref()
            .is_some_and(|prototype| prototype.is_empty())
        {
            return Err(invalid("prototype must not be empty when present"));
        }
        Ok(())
    }

    fn validate_visuals(&self, scene: &str) -> Result<(), ProjectConfigError> {
        let invalid = |reason: &str| ProjectConfigError::InvalidSceneEntityVisual {
            scene: scene.to_string(),
            entity: self.id.clone(),
            reason: reason.to_string(),
        };
        if !is_finite_vector(self.position) {
            return Err(invalid("position must be finite"));
        }
        if !self.rotation.is_finite() {
            return Err(invalid("rotation must be finite"));
        }
        if !is_positive_finite_vector(self.size) {
            return Err(invalid("size must be finite and positive"));
        }
        if !is_finite_color(self.tint) {
            return Err(invalid("tint must be finite"));
        }
        if !self.depth.is_finite() {
            return Err(invalid("depth must be finite"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectScenePhysicsConfig {
    #[serde(default)]
    pub body_type: ProjectPhysicsBodyType,
    #[serde(default)]
    pub linear_velocity: Vector2,
    #[serde(default)]
    pub angular_velocity: f32,
    #[serde(default)]
    pub fixtures: Vec<ProjectSceneFixtureConfig>,
    #[serde(default)]
    pub can_collide: bool,
    #[serde(default = "ProjectScenePhysicsConfig::default_awake")]
    pub awake: bool,
    #[serde(default = "ProjectScenePhysicsConfig::default_predict")]
    pub predict: bool,
}

impl ProjectScenePhysicsConfig {
    const fn default_awake() -> bool {
        true
    }

    const fn default_predict() -> bool {
        true
    }

    pub fn validate(&self, scene: &str, entity: &str) -> Result<(), ProjectConfigError> {
        if !is_finite_vector(self.linear_velocity) {
            return Err(ProjectConfigError::InvalidScenePhysicsValue {
                scene: scene.to_string(),
                entity: entity.to_string(),
                reason: "linear velocity must be finite".to_string(),
            });
        }
        if !self.angular_velocity.is_finite() {
            return Err(ProjectConfigError::InvalidScenePhysicsValue {
                scene: scene.to_string(),
                entity: entity.to_string(),
                reason: "angular velocity must be finite".to_string(),
            });
        }
        let mut fixture_ids = BTreeSet::new();
        for fixture in &self.fixtures {
            if fixture.id.is_empty() {
                return Err(ProjectConfigError::EmptySceneFixtureId {
                    scene: scene.to_string(),
                    entity: entity.to_string(),
                });
            }
            if !fixture_ids.insert(fixture.id.clone()) {
                return Err(ProjectConfigError::DuplicateSceneFixtureId {
                    scene: scene.to_string(),
                    entity: entity.to_string(),
                    id: fixture.id.clone(),
                });
            }
            fixture.validate_shape(scene, entity)?;
            fixture.validate_material(scene, entity)?;
        }
        Ok(())
    }
}

impl Default for ProjectScenePhysicsConfig {
    fn default() -> Self {
        Self {
            body_type: ProjectPhysicsBodyType::default(),
            linear_velocity: Vector2::ZERO,
            angular_velocity: 0.0,
            fixtures: Vec::new(),
            can_collide: false,
            awake: Self::default_awake(),
            predict: Self::default_predict(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectSceneFixtureConfig {
    pub id: String,
    pub shape: ProjectSceneFixtureShapeConfig,
    #[serde(default = "ProjectSceneFixtureConfig::default_friction")]
    pub friction: f32,
    #[serde(default)]
    pub restitution: f32,
    #[serde(default = "ProjectSceneFixtureConfig::default_hard")]
    pub hard: bool,
    #[serde(default)]
    pub mass: f32,
    #[serde(default)]
    pub collision_layer: i32,
    #[serde(default)]
    pub collision_mask: i32,
    #[serde(default)]
    pub body_type: ProjectPhysicsBodyType,
}

impl ProjectSceneFixtureConfig {
    const fn default_friction() -> f32 {
        0.4
    }

    const fn default_hard() -> bool {
        true
    }

    pub fn to_fixture(&self) -> Fixture {
        let mut fixture = Fixture::new(self.id.clone(), self.shape.to_phys_shape());
        fixture.friction = self.friction;
        fixture.restitution = self.restitution;
        fixture.hard = self.hard;
        fixture.mass = self.mass;
        fixture.collision_layer = self.collision_layer;
        fixture.collision_mask = self.collision_mask;
        fixture.body_type = self.body_type.into();
        fixture
    }

    fn validate_shape(&self, scene: &str, entity: &str) -> Result<(), ProjectConfigError> {
        let invalid = |reason: &str| ProjectConfigError::InvalidSceneFixtureShape {
            scene: scene.to_string(),
            entity: entity.to_string(),
            fixture: self.id.clone(),
            reason: reason.to_string(),
        };
        match self.shape {
            ProjectSceneFixtureShapeConfig::Aabb {
                local_bounds,
                radius,
            } => {
                if !is_finite_box(local_bounds) {
                    return Err(invalid("aabb bounds must be finite"));
                }
                if local_bounds.right <= local_bounds.left
                    || local_bounds.top <= local_bounds.bottom
                {
                    return Err(invalid("aabb bounds must have positive width and height"));
                }
                if !radius.is_finite() || radius < 0.0 {
                    return Err(invalid("aabb radius must be finite and non-negative"));
                }
            }
            ProjectSceneFixtureShapeConfig::Circle { position, radius } => {
                if !is_finite_vector(position) {
                    return Err(invalid("circle position must be finite"));
                }
                if !radius.is_finite() || radius <= 0.0 {
                    return Err(invalid("circle radius must be finite and positive"));
                }
            }
        }
        Ok(())
    }

    fn validate_material(&self, scene: &str, entity: &str) -> Result<(), ProjectConfigError> {
        let invalid = |reason: &str| ProjectConfigError::InvalidSceneFixtureMaterial {
            scene: scene.to_string(),
            entity: entity.to_string(),
            fixture: self.id.clone(),
            reason: reason.to_string(),
        };
        if !self.friction.is_finite() || self.friction < 0.0 {
            return Err(invalid("friction must be finite and non-negative"));
        }
        if !self.restitution.is_finite() || self.restitution < 0.0 {
            return Err(invalid("restitution must be finite and non-negative"));
        }
        if !self.mass.is_finite() || self.mass < 0.0 {
            return Err(invalid("mass must be finite and non-negative"));
        }
        Ok(())
    }
}

impl Default for ProjectSceneFixtureConfig {
    fn default() -> Self {
        Self {
            id: "main".to_string(),
            shape: ProjectSceneFixtureShapeConfig::Aabb {
                local_bounds: Box2::new(-0.5, -0.5, 0.5, 0.5),
                radius: 0.0,
            },
            friction: Self::default_friction(),
            restitution: 0.0,
            hard: Self::default_hard(),
            mass: 0.0,
            collision_layer: 0,
            collision_mask: 0,
            body_type: ProjectPhysicsBodyType::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ProjectSceneFixtureShapeConfig {
    Aabb { local_bounds: Box2, radius: f32 },
    Circle { position: Vector2, radius: f32 },
}

impl ProjectSceneFixtureShapeConfig {
    fn to_phys_shape(self) -> PhysShape {
        match self {
            Self::Aabb {
                local_bounds,
                radius,
            } => PhysShape::Aabb(AabbShape::new(local_bounds, radius)),
            Self::Circle { position, radius } => {
                PhysShape::Circle(CircleShape::new(position, radius))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectPhysicsBodyType {
    Kinematic,
    KinematicController,
    Static,
    #[default]
    Dynamic,
}

impl From<ProjectPhysicsBodyType> for BodyType {
    fn from(value: ProjectPhysicsBodyType) -> Self {
        match value {
            ProjectPhysicsBodyType::Kinematic => Self::Kinematic,
            ProjectPhysicsBodyType::KinematicController => Self::KinematicController,
            ProjectPhysicsBodyType::Static => Self::Static,
            ProjectPhysicsBodyType::Dynamic => Self::Dynamic,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectSpriteConfig {
    pub texture: u64,
    pub position: Vector2,
    #[serde(default = "ProjectSpriteConfig::default_size")]
    pub size: Vector2,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub tint: ProjectColorConfig,
    #[serde(default)]
    pub depth: f32,
}

impl ProjectSpriteConfig {
    const fn default_size() -> Vector2 {
        Vector2::ONE
    }

    fn validate_visuals(&self, scene: &str, index: usize) -> Result<(), ProjectConfigError> {
        let invalid = |reason: &str| ProjectConfigError::InvalidSceneSpriteVisual {
            scene: scene.to_string(),
            index,
            reason: reason.to_string(),
        };
        if !is_finite_vector(self.position) {
            return Err(invalid("position must be finite"));
        }
        if !is_positive_finite_vector(self.size) {
            return Err(invalid("size must be finite and positive"));
        }
        if !self.rotation.is_finite() {
            return Err(invalid("rotation must be finite"));
        }
        if !is_finite_color(self.tint) {
            return Err(invalid("tint must be finite"));
        }
        if !self.depth.is_finite() {
            return Err(invalid("depth must be finite"));
        }
        Ok(())
    }

    pub fn to_sprite_extract(&self, camera: ExtractCameraId) -> SpriteExtract {
        SpriteExtract::new(
            camera,
            ExtractTextureId::new(self.texture),
            self.position,
            self.size,
        )
        .with_rotation(self.rotation)
        .with_tint(self.tint.into())
        .with_depth(self.depth)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProjectColorConfig {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Default for ProjectColorConfig {
    fn default() -> Self {
        Self {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }
    }
}

impl From<ProjectColorConfig> for Color {
    fn from(value: ProjectColorConfig) -> Self {
        Self::new(value.r, value.g, value.b, value.a)
    }
}

fn is_finite_vector(value: Vector2) -> bool {
    value.x.is_finite() && value.y.is_finite()
}

fn is_positive_finite_vector(value: Vector2) -> bool {
    is_finite_vector(value) && value.x > 0.0 && value.y > 0.0
}

fn is_finite_box(value: Box2) -> bool {
    value.left.is_finite()
        && value.bottom.is_finite()
        && value.right.is_finite()
        && value.top.is_finite()
}

fn is_finite_color(value: ProjectColorConfig) -> bool {
    value.r.is_finite() && value.g.is_finite() && value.b.is_finite() && value.a.is_finite()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectTextureConfig {
    pub id: u64,
    pub label: String,
    pub width: u32,
    pub height: u32,
    #[serde(default = "ProjectTextureConfig::default_depth_or_layers")]
    pub depth_or_layers: u32,
    pub format: ProjectTextureFormat,
    #[serde(default)]
    pub usages: Vec<ProjectTextureUsage>,
}

impl ProjectTextureConfig {
    const fn default_depth_or_layers() -> u32 {
        1
    }

    pub fn to_cpu_config(&self) -> Result<CpuTextureResourceConfig, ProjectConfigError> {
        if self.usages.is_empty() {
            return Err(ProjectConfigError::MissingTextureUsage(self.id));
        }
        Ok(CpuTextureResourceConfig::new(
            GpuTextureId::new(self.id),
            GpuTextureDescriptor::new(
                self.label.clone(),
                TextureSize::new(self.width, self.height, self.depth_or_layers),
                self.format.into(),
                self.usages.iter().copied().map(TextureUsage::from),
            ),
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectTextureFormat {
    Rgba8Unorm,
    Bgra8Unorm,
    Depth24Stencil8,
    Depth32Float,
}

impl From<ProjectTextureFormat> for TextureFormat {
    fn from(value: ProjectTextureFormat) -> Self {
        match value {
            ProjectTextureFormat::Rgba8Unorm => Self::Rgba8Unorm,
            ProjectTextureFormat::Bgra8Unorm => Self::Bgra8Unorm,
            ProjectTextureFormat::Depth24Stencil8 => Self::Depth24Stencil8,
            ProjectTextureFormat::Depth32Float => Self::Depth32Float,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectTextureUsage {
    Sampled,
    RenderTarget,
    DepthStencil,
    Storage,
    TransferSrc,
    TransferDst,
}

impl From<ProjectTextureUsage> for TextureUsage {
    fn from(value: ProjectTextureUsage) -> Self {
        match value {
            ProjectTextureUsage::Sampled => Self::Sampled,
            ProjectTextureUsage::RenderTarget => Self::RenderTarget,
            ProjectTextureUsage::DepthStencil => Self::DepthStencil,
            ProjectTextureUsage::Storage => Self::Storage,
            ProjectTextureUsage::TransferSrc => Self::TransferSrc,
            ProjectTextureUsage::TransferDst => Self::TransferDst,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRenderPipelineConfig {
    pub id: u64,
    pub label: String,
    pub vertex_shader: u64,
    pub fragment_shader: Option<u64>,
    #[serde(default)]
    pub vertex_buffers: Vec<ProjectVertexBufferLayoutConfig>,
    #[serde(default)]
    pub color_targets: Vec<ProjectTextureFormat>,
    pub depth_target: Option<ProjectTextureFormat>,
    #[serde(default)]
    pub bind_group_layouts: Vec<u64>,
}

impl ProjectRenderPipelineConfig {
    pub fn to_cpu_config(&self) -> CpuRenderPipelineResourceConfig {
        let descriptor = RenderPipelineDescriptor::new(
            self.label.clone(),
            PipelineShader::new(ShaderModuleId::new(self.vertex_shader), ShaderStage::Vertex),
            self.fragment_shader.map(|shader| {
                PipelineShader::new(ShaderModuleId::new(shader), ShaderStage::Fragment)
            }),
            self.vertex_buffers.iter().map(VertexBufferLayout::from),
            self.color_targets.iter().copied().map(TextureFormat::from),
            self.depth_target.map(TextureFormat::from),
        )
        .with_bind_group_layouts(
            self.bind_group_layouts
                .iter()
                .copied()
                .map(BindGroupLayoutId::new),
        );
        CpuRenderPipelineResourceConfig::new(RenderPipelineId::new(self.id), descriptor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectVertexBufferLayoutConfig {
    pub slot: u32,
    pub stride_bytes: u64,
    pub step_mode: ProjectVertexStepMode,
}

impl From<&ProjectVertexBufferLayoutConfig> for VertexBufferLayout {
    fn from(value: &ProjectVertexBufferLayoutConfig) -> Self {
        Self::new(value.slot, value.stride_bytes, value.step_mode.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectVertexStepMode {
    Vertex,
    Instance,
}

impl From<ProjectVertexStepMode> for VertexStepMode {
    fn from(value: ProjectVertexStepMode) -> Self {
        match value {
            ProjectVertexStepMode::Vertex => Self::Vertex,
            ProjectVertexStepMode::Instance => Self::Instance,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectConfigError {
    DuplicateTextureId(u64),
    DuplicateRenderPipelineId(u64),
    DuplicateSceneName(String),
    DuplicateSceneEntityId {
        scene: String,
        id: String,
    },
    DuplicateSceneFixtureId {
        scene: String,
        entity: String,
        id: String,
    },
    DuplicateSceneInputAction {
        scene: String,
        action: String,
    },
    EmptyControlledSceneEntityId {
        scene: String,
    },
    EmptySceneEntityId {
        scene: String,
    },
    EmptySceneFixtureId {
        scene: String,
        entity: String,
    },
    EmptySceneInputAction {
        scene: String,
    },
    EmptySceneInputFunction {
        scene: String,
        action: String,
    },
    InvalidSceneFixtureShape {
        scene: String,
        entity: String,
        fixture: String,
        reason: String,
    },
    InvalidSceneFixtureMaterial {
        scene: String,
        entity: String,
        fixture: String,
        reason: String,
    },
    InvalidSceneEntityMetadata {
        scene: String,
        entity: String,
        reason: String,
    },
    InvalidSceneEntityVisual {
        scene: String,
        entity: String,
        reason: String,
    },
    InvalidScenePhysicsValue {
        scene: String,
        entity: String,
        reason: String,
    },
    InvalidSceneRenderData {
        scene: String,
        reason: String,
    },
    InvalidSceneSpriteVisual {
        scene: String,
        index: usize,
        reason: String,
    },
    MissingSceneEntityId {
        scene: String,
        id: String,
    },
    MissingSceneInputAction {
        scene: String,
        action: String,
    },
    MissingSceneTextureResource {
        scene: String,
        texture: u64,
    },
    MissingScene(String),
    MissingTextureUsage(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuTextureResourceConfig {
    pub id: GpuTextureId,
    pub descriptor: GpuTextureDescriptor,
}

impl CpuTextureResourceConfig {
    pub fn new(id: GpuTextureId, descriptor: GpuTextureDescriptor) -> Self {
        Self { id, descriptor }
    }

    fn build(&self) -> Result<GpuTexture, GraphicsResourceError> {
        GpuTexture::from_descriptor(self.id, self.descriptor.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuRenderPipelineResourceConfig {
    pub id: RenderPipelineId,
    pub descriptor: RenderPipelineDescriptor,
}

impl CpuRenderPipelineResourceConfig {
    pub fn new(id: RenderPipelineId, descriptor: RenderPipelineDescriptor) -> Self {
        Self { id, descriptor }
    }

    fn build(&self) -> Result<RenderPipeline, GraphicsResourceError> {
        RenderPipeline::from_descriptor(self.id, self.descriptor.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CpuFrameResourceConfig {
    pub frame: CpuFrameOptions,
    pub textures: Vec<CpuTextureResourceConfig>,
    pub render_pipelines: Vec<CpuRenderPipelineResourceConfig>,
}

impl CpuFrameResourceConfig {
    pub fn new(frame: CpuFrameOptions) -> Self {
        Self {
            frame,
            textures: Vec::new(),
            render_pipelines: Vec::new(),
        }
    }

    pub fn with_texture(mut self, texture: CpuTextureResourceConfig) -> Self {
        self.textures.push(texture);
        self
    }

    pub fn with_render_pipeline(mut self, pipeline: CpuRenderPipelineResourceConfig) -> Self {
        self.render_pipelines.push(pipeline);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameHandleAllocator {
    next: u64,
}

impl FrameHandleAllocator {
    pub const fn new() -> Self {
        Self { next: 1 }
    }

    pub const fn with_next(next: u64) -> Self {
        Self { next }
    }

    pub const fn next_raw(&self) -> u64 {
        self.next
    }

    fn allocate_raw(&mut self) -> u64 {
        let raw = self.next;
        self.next += 1;
        raw
    }

    pub fn allocate_render_options(
        &mut self,
        mut options: RenderFrameOptions,
    ) -> RenderFrameOptions {
        options.camera = ExtractCameraId::new(self.allocate_raw());
        options.sandbox_texture = ExtractTextureId::new(self.allocate_raw());
        options
    }

    pub fn allocate_cpu_options(&mut self, mut options: CpuFrameOptions) -> CpuFrameOptions {
        options.instance = GraphicsInstanceId::new(self.allocate_raw());
        options.device = GraphicsDeviceId::new(self.allocate_raw());
        options.surface = SurfaceTargetId::new(self.allocate_raw());
        options.frame = FrameId::new(self.allocate_raw());
        options.target = GpuTextureId::new(self.allocate_raw());
        options.vertices = GpuBufferId::new(self.allocate_raw());
        options.vertex_shader = ShaderModuleId::new(self.allocate_raw());
        options.fragment_shader = ShaderModuleId::new(self.allocate_raw());
        options.pipeline = RenderPipelineId::new(self.allocate_raw());
        options.commands = CommandListId::new(self.allocate_raw());
        options
    }

    pub fn allocate_transient_cpu_options(
        &mut self,
        mut options: CpuFrameOptions,
    ) -> CpuFrameOptions {
        options.frame = FrameId::new(self.allocate_raw());
        options.commands = CommandListId::new(self.allocate_raw());
        options
    }
}

impl Default for FrameHandleAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CpuFrame {
    extract: RenderExtract,
    prepared: PreparedFrame,
    queued: QueuedFrame,
    submission: FrameSubmission,
}

impl CpuFrame {
    pub const fn extract(&self) -> &RenderExtract {
        &self.extract
    }

    pub const fn prepared(&self) -> &PreparedFrame {
        &self.prepared
    }

    pub const fn queued(&self) -> &QueuedFrame {
        &self.queued
    }

    pub const fn submission(&self) -> &FrameSubmission {
        &self.submission
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CpuFrameError {
    InvalidExtract(Vec<RenderExtractError>),
    Prepare(Vec<hikari::PrepareFrameError>),
    Graphics(GraphicsResourceError),
}

impl From<GraphicsResourceError> for CpuFrameError {
    fn from(value: GraphicsResourceError) -> Self {
        Self::Graphics(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CpuFrameResources {
    instance: GraphicsInstance,
    device: GraphicsDevice,
    surface: SurfaceTarget,
    target: GpuTexture,
    extra_textures: BTreeMap<GpuTextureId, GpuTexture>,
    vertices: GpuBuffer,
    vertex_shader: ShaderModule,
    fragment_shader: ShaderModule,
    pipeline: RenderPipeline,
    extra_render_pipelines: BTreeMap<RenderPipelineId, RenderPipeline>,
    catalog: GraphicsResourceCatalog,
}

impl CpuFrameResources {
    pub fn from_options(options: CpuFrameOptions) -> Result<Self, GraphicsResourceError> {
        let instance = GraphicsInstance::new(options.instance, "headless");
        let device = GraphicsDevice::new(options.device, instance.id(), "cpu-device");
        let surface = SurfaceTarget::new(options.surface, "headless", options.surface_size);
        let target = GpuTexture::from_descriptor(
            options.target,
            GpuTextureDescriptor::new(
                "swapchain",
                TextureSize::new(
                    options.surface_size.width(),
                    options.surface_size.height(),
                    1,
                ),
                TextureFormat::Rgba8Unorm,
                [TextureUsage::RenderTarget],
            ),
        )?;
        let vertices = GpuBuffer::from_descriptor(
            options.vertices,
            GpuBufferDescriptor::new("quad_vertices", 256, [BufferUsage::Vertex]),
        )?;
        let vertex_shader = ShaderModule::from_descriptor(
            options.vertex_shader,
            ShaderModuleDescriptor::new(
                "sprite_vs",
                ShaderStage::Vertex,
                ShaderSourceKind::Wgsl,
                "main",
                "fn main() {}",
            ),
        )?;
        let fragment_shader = ShaderModule::from_descriptor(
            options.fragment_shader,
            ShaderModuleDescriptor::new(
                "sprite_fs",
                ShaderStage::Fragment,
                ShaderSourceKind::Wgsl,
                "main",
                "fn main() {}",
            ),
        )?;
        let pipeline = RenderPipeline::from_descriptor(
            options.pipeline,
            RenderPipelineDescriptor::new(
                "sprite_pipeline",
                PipelineShader::new(vertex_shader.id(), ShaderStage::Vertex),
                Some(PipelineShader::new(
                    fragment_shader.id(),
                    ShaderStage::Fragment,
                )),
                [VertexBufferLayout::new(0, 16, VertexStepMode::Vertex)],
                [TextureFormat::Rgba8Unorm],
                None,
            ),
        )?;

        let mut catalog = GraphicsResourceCatalog::new();
        catalog
            .register_device(&device)
            .register_surface(&surface)
            .register_texture(&target)
            .register_buffer(&vertices)
            .register_render_pipeline(&pipeline);

        Ok(Self {
            instance,
            device,
            surface,
            target,
            extra_textures: BTreeMap::new(),
            vertices,
            vertex_shader,
            fragment_shader,
            pipeline,
            extra_render_pipelines: BTreeMap::new(),
            catalog,
        })
    }

    pub fn from_config(config: CpuFrameResourceConfig) -> Result<Self, GraphicsResourceError> {
        let mut resources = Self::from_options(config.frame)?;
        for texture in config.textures {
            resources.register_texture(texture.build()?);
        }
        for pipeline in config.render_pipelines {
            resources.register_render_pipeline(pipeline.build()?);
        }
        Ok(resources)
    }

    pub const fn device(&self) -> &GraphicsDevice {
        &self.device
    }

    pub const fn instance(&self) -> &GraphicsInstance {
        &self.instance
    }

    pub const fn surface(&self) -> &SurfaceTarget {
        &self.surface
    }

    pub const fn target(&self) -> &GpuTexture {
        &self.target
    }

    pub fn texture(&self, id: GpuTextureId) -> Option<&GpuTexture> {
        if self.target.id() == id {
            Some(&self.target)
        } else {
            self.extra_textures.get(&id)
        }
    }

    pub fn texture_count(&self) -> usize {
        1 + self.extra_textures.len()
    }

    pub fn register_texture(&mut self, texture: GpuTexture) -> Option<GpuTexture> {
        let previous = if texture.id() == self.target.id() {
            Some(std::mem::replace(&mut self.target, texture.clone()))
        } else {
            self.extra_textures.insert(texture.id(), texture.clone())
        };
        self.catalog.register_texture(&texture);
        previous
    }

    pub const fn vertices(&self) -> &GpuBuffer {
        &self.vertices
    }

    pub const fn vertex_shader(&self) -> &ShaderModule {
        &self.vertex_shader
    }

    pub const fn fragment_shader(&self) -> &ShaderModule {
        &self.fragment_shader
    }

    pub const fn pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }

    pub fn render_pipeline(&self, id: RenderPipelineId) -> Option<&RenderPipeline> {
        if self.pipeline.id() == id {
            Some(&self.pipeline)
        } else {
            self.extra_render_pipelines.get(&id)
        }
    }

    pub fn render_pipeline_count(&self) -> usize {
        1 + self.extra_render_pipelines.len()
    }

    pub fn register_render_pipeline(&mut self, pipeline: RenderPipeline) -> Option<RenderPipeline> {
        let previous = if pipeline.id() == self.pipeline.id() {
            Some(std::mem::replace(&mut self.pipeline, pipeline.clone()))
        } else {
            self.extra_render_pipelines
                .insert(pipeline.id(), pipeline.clone())
        };
        self.catalog.register_render_pipeline(&pipeline);
        previous
    }

    pub const fn catalog(&self) -> &GraphicsResourceCatalog {
        &self.catalog
    }

    fn validate_submission(
        &self,
        submission: &FrameSubmission,
    ) -> Result<(), GraphicsResourceError> {
        self.catalog.validate_submission(submission)
    }
}

pub struct HeadlessApp {
    options: HeadlessAppOptions,
    server: DaikokuServer,
    client: BaseClient,
    ticks_run: u64,
    started: bool,
    sandbox_entity: Option<SandboxEntity>,
    project_scene_entities: Vec<ProjectSceneEntity>,
    frame_handles: FrameHandleAllocator,
    cpu_frame_resources: Option<CpuFrameResources>,
}

pub type App = HeadlessApp;

impl HeadlessApp {
    pub fn new(options: HeadlessAppOptions) -> Self {
        Self {
            server: DaikokuServer::new(options.server.clone()),
            client: BaseClient::new(options.client.clone()),
            options,
            ticks_run: 0,
            started: false,
            sandbox_entity: None,
            project_scene_entities: Vec::new(),
            frame_handles: FrameHandleAllocator::new(),
            cpu_frame_resources: None,
        }
    }

    pub fn startup(&mut self) {
        if self.started {
            return;
        }

        self.server.start();
        let _ = self
            .server
            .connect_player(self.options.user_id.clone(), self.options.username.clone());
        let _ = self.server.join_player(&self.options.user_id);
        self.client.startup(self.options.user_id.clone());
        self.client.flush_to_server(&mut self.server);
        self.started = true;
    }

    pub fn shutdown(&mut self) {
        self.client.shutdown();
        self.server
            .shutdown(Some("headless app shutdown".to_string()));
        self.started = false;
        self.sandbox_entity = None;
        self.project_scene_entities.clear();
        self.cpu_frame_resources = None;
    }

    pub const fn frame_handle_allocator(&self) -> &FrameHandleAllocator {
        &self.frame_handles
    }

    pub fn frame_handle_allocator_mut(&mut self) -> &mut FrameHandleAllocator {
        &mut self.frame_handles
    }

    pub fn cpu_frame_resources(&self) -> Option<&CpuFrameResources> {
        self.cpu_frame_resources.as_ref()
    }

    pub fn ensure_cpu_frame_resources(
        &mut self,
        options: CpuFrameOptions,
    ) -> Result<&CpuFrameResources, CpuFrameError> {
        self.ensure_cpu_frame_resources_from_config(CpuFrameResourceConfig::new(options))
    }

    pub fn ensure_cpu_frame_resources_from_config(
        &mut self,
        config: CpuFrameResourceConfig,
    ) -> Result<&CpuFrameResources, CpuFrameError> {
        if self.cpu_frame_resources.is_none() {
            self.cpu_frame_resources = Some(CpuFrameResources::from_config(config)?);
        }
        Ok(self
            .cpu_frame_resources
            .as_ref()
            .expect("resources inserted"))
    }

    pub fn spawn_sandbox_entity(&mut self, options: SandboxEntityOptions) -> SandboxEntity {
        let sandbox = self.create_sandbox_entity(options);
        self.sandbox_entity = Some(sandbox.clone());
        sandbox
    }

    fn create_sandbox_entity(&mut self, options: SandboxEntityOptions) -> SandboxEntity {
        if !self.started {
            self.startup();
        }

        let uid = self
            .server
            .create_entity_uninitialized(options.prototype.as_deref());
        let _ = self.server.apply_transform_state(
            uid,
            TransformComponentState {
                local_position: options.position,
                rotation: Angle::new(options.rotation as f64),
                parent_id: EntityUid::INVALID,
                map_id: MapId::NULLSPACE,
                grid_id: GridId::INVALID,
                no_local_rotation: false,
                anchored: false,
            },
        );
        let _ = self
            .server
            .set_appearance_data(uid, "name", options.appearance_name.clone());
        let _ = self.server.initialize_entity(uid);
        if options.attach_local_player {
            let _ = self.server.attach_player(&self.options.user_id, uid, true);
        }

        SandboxEntity {
            entity: uid,
            position: options.position,
            rotation: options.rotation,
            appearance_name: options.appearance_name,
        }
    }

    pub fn tick(&mut self) {
        if !self.started {
            self.startup();
        }

        self.client.flush_to_server(&mut self.server);
        self.server.tick_update(self.options.fixed_delta_seconds);
        let messages = self.server.take_outbox(&self.options.user_id);
        self.client.receive_server_messages(messages);
        self.client.tick_update();
        self.ticks_run += 1;
    }

    pub fn run_ticks(&mut self, count: u64) {
        for _ in 0..count {
            self.tick();
        }
    }

    pub fn handle_local_input(
        &mut self,
        function: impl Into<BoundKeyFunction>,
        state: BoundKeyState,
    ) -> bool {
        if !self.started {
            self.startup();
        }

        self.client.handle_local_input(function, state)
    }

    pub fn handle_project_scene_input(
        &mut self,
        project: &OmoikaneProjectConfig,
        scene_name: &str,
        action: &str,
        state: BoundKeyState,
    ) -> Result<bool, ProjectConfigError> {
        let function = project
            .scene(scene_name)?
            .input_function_for_action(action)?;
        Ok(self.handle_local_input(function, state))
    }

    pub fn options(&self) -> &HeadlessAppOptions {
        &self.options
    }

    pub fn server(&self) -> &DaikokuServer {
        &self.server
    }

    pub fn client(&self) -> &BaseClient {
        &self.client
    }

    pub fn sandbox_entity(&self) -> Option<&SandboxEntity> {
        self.sandbox_entity.as_ref()
    }

    pub fn project_scene_entities(&self) -> &[ProjectSceneEntity] {
        &self.project_scene_entities
    }

    pub fn project_scene_entity(
        &self,
        scene_name: &str,
        entity_id: &str,
    ) -> Option<&ProjectSceneEntity> {
        self.project_scene_entities
            .iter()
            .find(|entity| entity.scene_name == scene_name && entity.id == entity_id)
    }

    pub fn spawn_project_scene_entities(
        &mut self,
        project: &OmoikaneProjectConfig,
        scene_name: &str,
    ) -> Result<Vec<ProjectSceneEntity>, ProjectConfigError> {
        let scene = project.scene(scene_name)?;
        scene.validate_input_bindings()?;
        scene.validate_dynamic_entities()?;

        let mut spawned = Vec::with_capacity(scene.dynamic_entities.len());
        for entity_config in &scene.dynamic_entities {
            let attach_local_player = entity_config.attach_local_player
                || scene
                    .controlled_entity
                    .as_ref()
                    .is_some_and(|controlled| controlled == &entity_config.id);
            let sandbox = self.create_sandbox_entity(SandboxEntityOptions {
                prototype: entity_config.prototype.clone(),
                position: entity_config.position,
                rotation: entity_config.rotation,
                appearance_name: entity_config.appearance_name.clone(),
                attach_local_player,
            });
            if let Some(physics) = &entity_config.physics {
                let _ = self.server.configure_physics_body(
                    sandbox.entity,
                    Some(physics.body_type.into()),
                    Some(physics.awake),
                    Some(physics.can_collide),
                    Some(physics.predict),
                );
                let _ = self
                    .server
                    .set_body_linear_velocity(sandbox.entity, physics.linear_velocity);
                let _ = self
                    .server
                    .set_body_angular_velocity(sandbox.entity, physics.angular_velocity);
                for fixture in &physics.fixtures {
                    let _ = self
                        .server
                        .insert_fixture(sandbox.entity, fixture.to_fixture());
                }
            }
            let scene_entity = ProjectSceneEntity {
                scene_name: scene.name.clone(),
                id: entity_config.id.clone(),
                entity: sandbox.entity,
                appearance_name: sandbox.appearance_name,
                attach_local_player,
                physics: entity_config.physics.clone(),
                rotation: sandbox.rotation,
                texture: ExtractTextureId::new(entity_config.texture),
                size: entity_config.size,
                tint: entity_config.tint.into(),
                depth: entity_config.depth,
            };
            self.project_scene_entities.push(scene_entity.clone());
            spawned.push(scene_entity);
        }
        Ok(spawned)
    }

    pub fn build_render_extract(
        &self,
        options: RenderFrameOptions,
    ) -> Result<RenderExtract, Vec<RenderExtractError>> {
        let mut extract = RenderExtract::new();
        extract
            .add_camera(Camera2dExtract::new(
                options.camera,
                "main",
                options.world_view,
                options.viewport_size,
            ))
            .map_err(|error| vec![error])?;

        if let Some(sandbox) = self.sandbox_entity()
            && let Some(position) = self.client.entity_local_position(sandbox.entity)
        {
            extract.push_sprite(
                SpriteExtract::new(
                    options.camera,
                    options.sandbox_texture,
                    position,
                    options.sprite_size,
                )
                .with_tint(options.sprite_tint)
                .with_depth(options.sprite_depth),
            );
        }

        extract.validate().map(|_| extract)
    }

    pub fn build_project_scene_render_extract(
        &self,
        project: &OmoikaneProjectConfig,
        scene_name: &str,
    ) -> Result<RenderExtract, ProjectSceneRenderError> {
        let scene = project.scene(scene_name)?;
        scene.validate_render_data()?;
        let mut extract = self
            .build_render_extract(scene.to_render_options())
            .map_err(ProjectSceneRenderError::InvalidExtract)?;
        for sprite in scene.sprite_extracts() {
            extract.push_sprite(sprite);
        }
        for entity in self
            .project_scene_entities
            .iter()
            .filter(|entity| entity.scene_name == scene.name)
        {
            if let Some(position) = self.client.entity_local_position(entity.entity) {
                let rotation = self
                    .client
                    .entity_local_rotation(entity.entity)
                    .unwrap_or(entity.rotation);
                extract.push_sprite(
                    SpriteExtract::new(
                        ExtractCameraId::new(scene.camera),
                        entity.texture,
                        position,
                        entity.size,
                    )
                    .with_rotation(rotation)
                    .with_tint(entity.tint)
                    .with_depth(entity.depth),
                );
            }
        }
        extract
            .validate()
            .map(|_| extract)
            .map_err(ProjectSceneRenderError::InvalidExtract)
    }

    pub fn build_cpu_frame(
        &self,
        render_options: RenderFrameOptions,
        frame_options: CpuFrameOptions,
    ) -> Result<CpuFrame, CpuFrameError> {
        let resources = CpuFrameResources::from_options(frame_options)?;
        self.build_cpu_frame_with_resources(render_options, frame_options, &resources)
    }

    fn build_cpu_frame_with_resources(
        &self,
        render_options: RenderFrameOptions,
        frame_options: CpuFrameOptions,
        resources: &CpuFrameResources,
    ) -> Result<CpuFrame, CpuFrameError> {
        let extract = self
            .build_render_extract(render_options)
            .map_err(CpuFrameError::InvalidExtract)?;
        self.build_cpu_frame_from_extract_with_resources(extract, frame_options, resources)
    }

    fn build_cpu_frame_from_extract_with_resources(
        &self,
        extract: RenderExtract,
        frame_options: CpuFrameOptions,
        resources: &CpuFrameResources,
    ) -> Result<CpuFrame, CpuFrameError> {
        let prepared = PreparedFrame::from_extract(&extract).map_err(CpuFrameError::Prepare)?;
        let queued = prepared.queue();
        let draw_instances = queued
            .draws()
            .iter()
            .map(|draw| draw.instances())
            .sum::<usize>() as u32;

        let mut commands = RenderCommandList::builder(frame_options.commands, "frame_commands");
        commands.render_pass(
            RenderPassDescriptor::new("main", [resources.target().id()], None),
            |pass| {
                pass.set_pipeline(resources.pipeline().id())
                    .set_vertex_buffer(0, resources.vertices().id())
                    .draw(DrawCall::new(6, draw_instances))?;
                Ok(())
            },
        )?;
        let commands = commands.finish()?;
        let submission = FrameSubmission::new(
            FrameContext::new(
                frame_options.frame,
                resources.device().id(),
                resources.surface().id(),
            ),
            [commands],
        )?;
        resources.validate_submission(&submission)?;

        Ok(CpuFrame {
            extract,
            prepared,
            queued,
            submission,
        })
    }

    fn validate_project_scene_texture_resources(
        &self,
        scene_name: &str,
        extract: &RenderExtract,
        resources: &CpuFrameResources,
    ) -> Result<(), ProjectConfigError> {
        let mut textures = BTreeSet::new();
        for sprite in extract.sprites() {
            textures.insert(sprite.texture());
        }
        for batch in extract.tile_batches() {
            textures.insert(batch.texture());
        }
        for texture in textures {
            if resources
                .texture(GpuTextureId::new(texture.raw()))
                .is_none()
            {
                return Err(ProjectConfigError::MissingSceneTextureResource {
                    scene: scene_name.to_string(),
                    texture: texture.raw(),
                });
            }
        }
        Ok(())
    }

    pub fn build_allocated_cpu_frame(
        &mut self,
        render_options: RenderFrameOptions,
        frame_options: CpuFrameOptions,
    ) -> Result<CpuFrame, CpuFrameError> {
        let render_options = self.frame_handles.allocate_render_options(render_options);
        let frame_options = self.frame_handles.allocate_cpu_options(frame_options);
        self.build_cpu_frame(render_options, frame_options)
    }

    pub fn build_registered_cpu_frame(
        &mut self,
        render_options: RenderFrameOptions,
        frame_options: CpuFrameOptions,
    ) -> Result<CpuFrame, CpuFrameError> {
        self.build_registered_cpu_frame_from_config(
            render_options,
            CpuFrameResourceConfig::new(frame_options),
        )
    }

    pub fn build_registered_cpu_frame_from_config(
        &mut self,
        render_options: RenderFrameOptions,
        resource_config: CpuFrameResourceConfig,
    ) -> Result<CpuFrame, CpuFrameError> {
        let frame_options = resource_config.frame;
        self.ensure_cpu_frame_resources_from_config(resource_config)?;
        let render_options = self.frame_handles.allocate_render_options(render_options);
        let frame_options = self
            .frame_handles
            .allocate_transient_cpu_options(frame_options);
        let resources = self
            .cpu_frame_resources
            .as_ref()
            .expect("resources ensured")
            .clone();
        self.build_cpu_frame_with_resources(render_options, frame_options, &resources)
    }

    pub fn build_project_scene_registered_cpu_frame(
        &mut self,
        project: &OmoikaneProjectConfig,
        scene_name: &str,
        resource_config: CpuFrameResourceConfig,
    ) -> Result<CpuFrame, ProjectSceneCpuFrameError> {
        let frame_options = resource_config.frame;
        self.ensure_cpu_frame_resources_from_config(resource_config)
            .map_err(ProjectSceneCpuFrameError::Frame)?;
        let extract = self
            .build_project_scene_render_extract(project, scene_name)
            .map_err(ProjectSceneCpuFrameError::Scene)?;
        let frame_options = self
            .frame_handles
            .allocate_transient_cpu_options(frame_options);
        let resources = self
            .cpu_frame_resources
            .as_ref()
            .expect("resources ensured")
            .clone();
        self.validate_project_scene_texture_resources(scene_name, &extract, &resources)
            .map_err(ProjectSceneRenderError::Project)
            .map_err(ProjectSceneCpuFrameError::Scene)?;
        self.build_cpu_frame_from_extract_with_resources(extract, frame_options, &resources)
            .map_err(ProjectSceneCpuFrameError::Frame)
    }

    pub fn app_state(&self) -> AppState {
        AppState {
            started: self.started,
            ticks_run: self.ticks_run,
            current_tick: self.current_tick(),
            server_state: self.server_state(),
            client_run_level: self.client_run_level(),
        }
    }

    pub fn server_state(&self) -> ServerState {
        self.server.state()
    }

    pub fn client_run_level(&self) -> ClientRunLevel {
        self.client.run_level()
    }

    pub fn current_tick(&self) -> GameTick {
        self.server.current_tick()
    }

    pub fn ticks_run(&self) -> u64 {
        self.ticks_run
    }

    pub fn is_started(&self) -> bool {
        self.started
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectSceneRenderError {
    Project(ProjectConfigError),
    InvalidExtract(Vec<RenderExtractError>),
}

impl From<ProjectConfigError> for ProjectSceneRenderError {
    fn from(value: ProjectConfigError) -> Self {
        Self::Project(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectSceneCpuFrameError {
    Scene(ProjectSceneRenderError),
    Frame(CpuFrameError),
}

impl Default for HeadlessApp {
    fn default() -> Self {
        Self::new(HeadlessAppOptions::default())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CpuFrameOptions, CpuFrameResourceConfig, CpuFrameResources,
        CpuRenderPipelineResourceConfig, CpuTextureResourceConfig, FrameHandleAllocator,
        HeadlessApp, HeadlessAppOptions, OmoikaneProjectConfig, ProjectColorConfig,
        ProjectConfigError, ProjectRenderPipelineConfig, ProjectResourceConfig, ProjectSceneConfig,
        ProjectSceneCpuFrameError, ProjectSceneEntityConfig, ProjectSceneFixtureConfig,
        ProjectSceneFixtureShapeConfig, ProjectSceneInputBindingConfig, ProjectScenePhysicsConfig,
        ProjectSceneRenderError, ProjectSpriteConfig, ProjectTextureConfig, ProjectTextureFormat,
        ProjectTextureUsage, ProjectVertexBufferLayoutConfig, ProjectVertexStepMode,
        RenderFrameOptions, SandboxEntityOptions,
    };
    use daikoku::{BoundKeyState, ServerState};
    use hikari::{
        ExtractCameraId, ExtractTextureId, GpuTexture, GpuTextureDescriptor, GpuTextureId,
        PipelineShader, PreparedFrame, QueuedPrimitive, RenderPipeline, RenderPipelineDescriptor,
        RenderPipelineId, ShaderStage, TextureFormat, TextureSize, TextureUsage,
        VertexBufferLayout, VertexStepMode,
    };
    use jikan::GameTick;
    use keisan::{Box2, Color, Vector2};
    use shinobi::ClientRunLevel;

    #[test]
    fn headless_app_starts_server_and_client() {
        let mut app = HeadlessApp::default();

        app.startup();

        assert!(app.is_started());
        assert_eq!(app.server_state(), ServerState::Running);
        assert_eq!(app.client_run_level(), ClientRunLevel::Connected);
        assert_eq!(app.current_tick(), GameTick::ZERO);
        assert_eq!(app.ticks_run(), 0);
    }

    #[test]
    fn headless_app_runs_local_ticks_and_applies_server_state() {
        let mut app = HeadlessApp::default();

        app.run_ticks(2);

        assert_eq!(
            app.app_state(),
            super::AppState {
                started: true,
                ticks_run: 2,
                current_tick: GameTick::new(2),
                server_state: ServerState::Running,
                client_run_level: ClientRunLevel::InGame,
            }
        );
    }

    #[test]
    fn headless_app_spawns_and_replicates_sandbox_entity() {
        let mut app = HeadlessApp::default();
        let sandbox = app.spawn_sandbox_entity(SandboxEntityOptions {
            position: Vector2::new(2.0, -1.0),
            appearance_name: "test_sprite".to_string(),
            ..SandboxEntityOptions::default()
        });

        app.run_ticks(2);

        assert_eq!(app.client().controlled_entity(), Some(sandbox.entity));
        assert!(app.client().entity_exists(sandbox.entity));
        assert_eq!(
            app.client().entity_local_position(sandbox.entity),
            Some(Vector2::new(2.0, -1.0))
        );
        assert_eq!(
            app.client()
                .entity_appearance_data::<String>(sandbox.entity, "name")
                .as_deref(),
            Some("test_sprite")
        );
        assert_eq!(app.sandbox_entity(), Some(&sandbox));
    }

    #[test]
    fn headless_app_extracts_replicated_sandbox_entity_for_rendering() {
        let mut app = HeadlessApp::default();
        let sandbox = app.spawn_sandbox_entity(SandboxEntityOptions {
            position: Vector2::new(2.0, -1.0),
            ..SandboxEntityOptions::default()
        });
        app.run_ticks(2);

        let extract = app
            .build_render_extract(RenderFrameOptions::default())
            .expect("valid render extract");
        let validation = extract.validate().expect("extract validates");

        assert_eq!(validation.camera_count(), 1);
        assert_eq!(validation.sprite_count(), 1);
        assert_eq!(extract.sprites()[0].position(), sandbox.position);

        let prepared = PreparedFrame::from_extract(&extract).expect("prepared frame");
        assert_eq!(prepared.primitive_count(), 1);
        let queued = prepared.queue();
        assert_eq!(queued.draws().len(), 1);
        assert_eq!(queued.draws()[0].primitive(), QueuedPrimitive::Sprite);
        assert_eq!(queued.draws()[0].instances(), 1);
    }

    #[test]
    fn headless_app_builds_valid_cpu_frame_submission() {
        let mut app = HeadlessApp::default();
        app.spawn_sandbox_entity(SandboxEntityOptions::default());
        app.run_ticks(2);

        let frame = app
            .build_cpu_frame(RenderFrameOptions::default(), CpuFrameOptions::default())
            .expect("valid cpu frame");

        assert_eq!(frame.extract().sprites().len(), 1);
        assert_eq!(frame.prepared().primitive_count(), 1);
        assert_eq!(frame.queued().draws().len(), 1);
        assert_eq!(frame.submission().command_lists().len(), 1);
        assert_eq!(
            frame.submission().command_lists()[0].debug_dump(),
            "command_list id=10 label=\"frame_commands\" commands=5\n  0: begin_render_pass label=\"main\" color_targets=[5]\n  1: set_pipeline pipeline=9\n  2: set_vertex_buffer slot=0 buffer=6\n  3: draw vertices=6 instances=1\n  4: end_render_pass\n"
        );
    }

    #[test]
    fn frame_handle_allocator_assigns_distinct_render_and_cpu_frame_handles() {
        let mut allocator = FrameHandleAllocator::new();

        let render = allocator.allocate_render_options(RenderFrameOptions::default());
        let cpu = allocator.allocate_cpu_options(CpuFrameOptions::default());

        assert_eq!(render.camera.raw(), 1);
        assert_eq!(render.sandbox_texture.raw(), 2);
        assert_eq!(cpu.instance.raw(), 3);
        assert_eq!(cpu.device.raw(), 4);
        assert_eq!(cpu.surface.raw(), 5);
        assert_eq!(cpu.frame.raw(), 6);
        assert_eq!(cpu.target.raw(), 7);
        assert_eq!(cpu.vertices.raw(), 8);
        assert_eq!(cpu.vertex_shader.raw(), 9);
        assert_eq!(cpu.fragment_shader.raw(), 10);
        assert_eq!(cpu.pipeline.raw(), 11);
        assert_eq!(cpu.commands.raw(), 12);
        assert_eq!(allocator.next_raw(), 13);
    }

    #[test]
    fn headless_app_builds_allocated_cpu_frames_with_distinct_handles() {
        let mut app = HeadlessApp::default();
        app.spawn_sandbox_entity(SandboxEntityOptions::default());
        app.run_ticks(2);

        let first = app
            .build_allocated_cpu_frame(RenderFrameOptions::default(), CpuFrameOptions::default())
            .expect("first cpu frame");
        let second = app
            .build_allocated_cpu_frame(RenderFrameOptions::default(), CpuFrameOptions::default())
            .expect("second cpu frame");

        assert_eq!(first.extract().sprites().len(), 1);
        assert_eq!(second.extract().sprites().len(), 1);
        assert_ne!(
            first.submission().frame().frame(),
            second.submission().frame().frame()
        );
        assert_ne!(
            first.submission().command_lists()[0].id(),
            second.submission().command_lists()[0].id()
        );
        assert_eq!(app.frame_handle_allocator().next_raw(), 25);
    }

    #[test]
    fn headless_app_reuses_registered_cpu_resources_between_frames() {
        let mut app = HeadlessApp::default();
        app.spawn_sandbox_entity(SandboxEntityOptions::default());
        app.run_ticks(2);

        let first = app
            .build_registered_cpu_frame(RenderFrameOptions::default(), CpuFrameOptions::default())
            .expect("first registered cpu frame");
        let second = app
            .build_registered_cpu_frame(RenderFrameOptions::default(), CpuFrameOptions::default())
            .expect("second registered cpu frame");
        let resources = app.cpu_frame_resources().expect("registered resources");

        assert_eq!(
            resources.instance().id(),
            CpuFrameOptions::default().instance
        );
        assert_eq!(resources.device().id(), CpuFrameOptions::default().device);
        assert_eq!(resources.surface().id(), CpuFrameOptions::default().surface);
        assert_eq!(resources.target().id(), CpuFrameOptions::default().target);
        assert_eq!(
            resources.vertices().id(),
            CpuFrameOptions::default().vertices
        );
        assert_eq!(
            resources.vertex_shader().id(),
            CpuFrameOptions::default().vertex_shader
        );
        assert_eq!(
            resources.fragment_shader().id(),
            CpuFrameOptions::default().fragment_shader
        );
        assert_eq!(
            resources.pipeline().id(),
            CpuFrameOptions::default().pipeline
        );
        assert_ne!(
            first.submission().frame().frame(),
            second.submission().frame().frame()
        );
        assert_ne!(
            first.submission().command_lists()[0].id(),
            second.submission().command_lists()[0].id()
        );
        assert_eq!(app.frame_handle_allocator().next_raw(), 9);
    }

    #[test]
    fn cpu_frame_resources_can_register_additional_textures_and_pipelines() {
        let mut resources =
            CpuFrameResources::from_options(CpuFrameOptions::default()).expect("resources");
        let texture = GpuTexture::from_descriptor(
            GpuTextureId::new(20),
            GpuTextureDescriptor::new(
                "ui_atlas",
                TextureSize::new(256, 256, 1),
                TextureFormat::Rgba8Unorm,
                [TextureUsage::Sampled],
            ),
        )
        .expect("texture");
        let pipeline = RenderPipeline::from_descriptor(
            RenderPipelineId::new(21),
            RenderPipelineDescriptor::new(
                "ui_pipeline",
                PipelineShader::new(resources.vertex_shader().id(), ShaderStage::Vertex),
                Some(PipelineShader::new(
                    resources.fragment_shader().id(),
                    ShaderStage::Fragment,
                )),
                [VertexBufferLayout::new(0, 16, VertexStepMode::Vertex)],
                [TextureFormat::Rgba8Unorm],
                None,
            ),
        )
        .expect("pipeline");

        assert!(resources.texture(texture.id()).is_none());
        assert!(resources.render_pipeline(pipeline.id()).is_none());

        assert_eq!(resources.register_texture(texture.clone()), None);
        assert_eq!(resources.register_render_pipeline(pipeline.clone()), None);

        assert_eq!(resources.texture(texture.id()), Some(&texture));
        assert_eq!(resources.render_pipeline(pipeline.id()), Some(&pipeline));
        assert_eq!(resources.texture_count(), 2);
        assert_eq!(resources.render_pipeline_count(), 2);
        assert!(resources.catalog().contains_texture(texture.id()));
        assert!(resources.catalog().contains_render_pipeline(pipeline.id()));
    }

    #[test]
    fn cpu_frame_resources_build_from_project_style_config() {
        let frame = CpuFrameOptions::default();
        let config = CpuFrameResourceConfig::new(frame)
            .with_texture(CpuTextureResourceConfig::new(
                GpuTextureId::new(30),
                GpuTextureDescriptor::new(
                    "scene_atlas",
                    TextureSize::new(128, 64, 1),
                    TextureFormat::Rgba8Unorm,
                    [TextureUsage::Sampled],
                ),
            ))
            .with_render_pipeline(CpuRenderPipelineResourceConfig::new(
                RenderPipelineId::new(31),
                RenderPipelineDescriptor::new(
                    "scene_pipeline",
                    PipelineShader::new(frame.vertex_shader, ShaderStage::Vertex),
                    Some(PipelineShader::new(
                        frame.fragment_shader,
                        ShaderStage::Fragment,
                    )),
                    [VertexBufferLayout::new(0, 16, VertexStepMode::Vertex)],
                    [TextureFormat::Rgba8Unorm],
                    None,
                ),
            ));

        let resources = CpuFrameResources::from_config(config).expect("configured resources");

        assert!(resources.texture(GpuTextureId::new(30)).is_some());
        assert!(
            resources
                .render_pipeline(RenderPipelineId::new(31))
                .is_some()
        );
        assert!(resources.catalog().contains_texture(GpuTextureId::new(30)));
        assert!(
            resources
                .catalog()
                .contains_render_pipeline(RenderPipelineId::new(31))
        );
    }

    #[test]
    fn cpu_frame_resources_reject_invalid_project_style_config() {
        let config = CpuFrameResourceConfig::new(CpuFrameOptions::default()).with_texture(
            CpuTextureResourceConfig::new(
                GpuTextureId::new(40),
                GpuTextureDescriptor::new(
                    "empty_scene_atlas",
                    TextureSize::new(0, 64, 1),
                    TextureFormat::Rgba8Unorm,
                    [TextureUsage::Sampled],
                ),
            ),
        );

        assert!(CpuFrameResources::from_config(config).is_err());
    }

    #[test]
    fn headless_app_builds_registered_cpu_frame_from_resource_config() {
        let mut app = HeadlessApp::default();
        app.spawn_sandbox_entity(SandboxEntityOptions::default());
        app.run_ticks(2);
        let frame_options = CpuFrameOptions::default();
        let config =
            CpuFrameResourceConfig::new(frame_options).with_texture(CpuTextureResourceConfig::new(
                GpuTextureId::new(50),
                GpuTextureDescriptor::new(
                    "project_atlas",
                    TextureSize::new(32, 32, 1),
                    TextureFormat::Rgba8Unorm,
                    [TextureUsage::Sampled],
                ),
            ));

        let frame = app
            .build_registered_cpu_frame_from_config(RenderFrameOptions::default(), config)
            .expect("registered frame from config");
        let resources = app.cpu_frame_resources().expect("registered resources");

        assert_eq!(frame.extract().sprites().len(), 1);
        assert!(resources.texture(GpuTextureId::new(50)).is_some());
        assert!(resources.catalog().contains_texture(GpuTextureId::new(50)));
    }

    #[test]
    fn project_config_roundtrips_json_and_builds_cpu_resource_config() {
        let frame = CpuFrameOptions::default();
        let project = OmoikaneProjectConfig {
            name: "sandbox".to_string(),
            resources: ProjectResourceConfig {
                textures: vec![ProjectTextureConfig {
                    id: 60,
                    label: "sandbox_atlas".to_string(),
                    width: 64,
                    height: 32,
                    depth_or_layers: 1,
                    format: ProjectTextureFormat::Rgba8Unorm,
                    usages: vec![ProjectTextureUsage::Sampled],
                }],
                render_pipelines: vec![ProjectRenderPipelineConfig {
                    id: 63,
                    label: "sandbox_pipeline".to_string(),
                    vertex_shader: frame.vertex_shader.raw(),
                    fragment_shader: Some(frame.fragment_shader.raw()),
                    vertex_buffers: vec![ProjectVertexBufferLayoutConfig {
                        slot: 0,
                        stride_bytes: 16,
                        step_mode: ProjectVertexStepMode::Vertex,
                    }],
                    color_targets: vec![ProjectTextureFormat::Rgba8Unorm],
                    depth_target: None,
                    bind_group_layouts: Vec::new(),
                }],
            },
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 70,
                sandbox_texture: 60,
                world_view: Box2::centered_around(Vector2::ZERO, Vector2::new(20.0, 10.0)),
                viewport_size: Vector2::new(1024.0, 512.0),
                controlled_entity: None,
                input_bindings: Vec::new(),
                sprite_size: Vector2::new(2.0, 3.0),
                sprite_tint: ProjectColorConfig {
                    r: 0.25,
                    g: 0.5,
                    b: 0.75,
                    a: 1.0,
                },
                sprite_depth: 4.0,
                sprites: vec![ProjectSpriteConfig {
                    texture: 60,
                    position: Vector2::new(5.0, -2.0),
                    size: Vector2::new(0.5, 0.75),
                    rotation: 0.25,
                    tint: ProjectColorConfig {
                        r: 1.0,
                        g: 0.0,
                        b: 0.5,
                        a: 0.8,
                    },
                    depth: 5.0,
                }],
                dynamic_entities: Vec::new(),
            }],
        };

        let json = project.to_json_string_pretty().expect("json");
        let decoded = OmoikaneProjectConfig::from_json_str(&json).expect("decoded project");
        let resource_config = decoded
            .to_cpu_frame_resource_config(CpuFrameOptions::default())
            .expect("resource config");
        let resources = CpuFrameResources::from_config(resource_config).expect("resources");

        assert_eq!(decoded, project);
        assert!(resources.texture(GpuTextureId::new(60)).is_some());
        assert!(
            resources
                .render_pipeline(RenderPipelineId::new(63))
                .is_some()
        );
        assert!(resources.catalog().contains_texture(GpuTextureId::new(60)));
        assert!(
            resources
                .catalog()
                .contains_render_pipeline(RenderPipelineId::new(63))
        );

        let render_options = decoded
            .render_frame_options_for_scene("main")
            .expect("scene render options");
        assert_eq!(render_options.camera, ExtractCameraId::new(70));
        assert_eq!(render_options.sandbox_texture, ExtractTextureId::new(60));
        assert_eq!(render_options.viewport_size, Vector2::new(1024.0, 512.0));
        assert_eq!(render_options.sprite_size, Vector2::new(2.0, 3.0));
        assert_eq!(render_options.sprite_tint, Color::new(0.25, 0.5, 0.75, 1.0));
        assert_eq!(render_options.sprite_depth, 4.0);
        let sprites = decoded.scene("main").expect("scene").sprite_extracts();
        assert_eq!(sprites.len(), 1);
        assert_eq!(sprites[0].camera(), ExtractCameraId::new(70));
        assert_eq!(sprites[0].texture(), ExtractTextureId::new(60));
        assert_eq!(sprites[0].position(), Vector2::new(5.0, -2.0));
        assert_eq!(sprites[0].size(), Vector2::new(0.5, 0.75));
        assert_eq!(sprites[0].rotation(), 0.25);
        assert_eq!(sprites[0].tint(), Color::new(1.0, 0.0, 0.5, 0.8));
        assert_eq!(sprites[0].depth(), 5.0);
    }

    #[test]
    fn project_config_rejects_duplicate_texture_ids() {
        let project = OmoikaneProjectConfig {
            name: "duplicate-textures".to_string(),
            resources: ProjectResourceConfig {
                textures: vec![
                    ProjectTextureConfig {
                        id: 61,
                        label: "a".to_string(),
                        width: 1,
                        height: 1,
                        depth_or_layers: 1,
                        format: ProjectTextureFormat::Rgba8Unorm,
                        usages: vec![ProjectTextureUsage::Sampled],
                    },
                    ProjectTextureConfig {
                        id: 61,
                        label: "b".to_string(),
                        width: 1,
                        height: 1,
                        depth_or_layers: 1,
                        format: ProjectTextureFormat::Rgba8Unorm,
                        usages: vec![ProjectTextureUsage::Sampled],
                    },
                ],
                render_pipelines: Vec::new(),
            },
            scenes: Vec::new(),
        };

        assert_eq!(
            project.to_cpu_frame_resource_config(CpuFrameOptions::default()),
            Err(ProjectConfigError::DuplicateTextureId(61))
        );
    }

    #[test]
    fn project_config_rejects_duplicate_render_pipeline_ids() {
        let frame = CpuFrameOptions::default();
        let pipeline = ProjectRenderPipelineConfig {
            id: 64,
            label: "duplicate_pipeline".to_string(),
            vertex_shader: frame.vertex_shader.raw(),
            fragment_shader: Some(frame.fragment_shader.raw()),
            vertex_buffers: vec![ProjectVertexBufferLayoutConfig {
                slot: 0,
                stride_bytes: 16,
                step_mode: ProjectVertexStepMode::Vertex,
            }],
            color_targets: vec![ProjectTextureFormat::Rgba8Unorm],
            depth_target: None,
            bind_group_layouts: Vec::new(),
        };
        let project = OmoikaneProjectConfig {
            name: "duplicate-pipelines".to_string(),
            resources: ProjectResourceConfig {
                textures: Vec::new(),
                render_pipelines: vec![pipeline.clone(), pipeline],
            },
            scenes: Vec::new(),
        };

        assert_eq!(
            project.to_cpu_frame_resource_config(frame),
            Err(ProjectConfigError::DuplicateRenderPipelineId(64))
        );
    }

    #[test]
    fn project_config_resolves_named_scene_render_options() {
        let project = OmoikaneProjectConfig {
            name: "scene-project".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 80,
                sandbox_texture: 81,
                world_view: Box2::centered_around(Vector2::ZERO, Vector2::new(16.0, 9.0)),
                viewport_size: Vector2::new(1280.0, 720.0),
                controlled_entity: None,
                input_bindings: Vec::new(),
                sprite_size: Vector2::new(1.5, 1.25),
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: -1.0,
                sprites: Vec::new(),
                dynamic_entities: Vec::new(),
            }],
        };

        let options = project
            .render_frame_options_for_scene("main")
            .expect("scene render options");

        assert_eq!(options.camera, ExtractCameraId::new(80));
        assert_eq!(options.sandbox_texture, ExtractTextureId::new(81));
        assert_eq!(options.sprite_size, Vector2::new(1.5, 1.25));
        assert_eq!(options.sprite_tint, Color::WHITE);
        assert_eq!(options.sprite_depth, -1.0);
    }

    #[test]
    fn project_config_rejects_missing_and_duplicate_scenes() {
        let scene = ProjectSceneConfig {
            name: "main".to_string(),
            camera: 90,
            sandbox_texture: 91,
            world_view: Box2::centered_around(Vector2::ZERO, Vector2::new(16.0, 9.0)),
            viewport_size: Vector2::new(1280.0, 720.0),
            controlled_entity: None,
            input_bindings: Vec::new(),
            sprite_size: Vector2::ONE,
            sprite_tint: ProjectColorConfig::default(),
            sprite_depth: 0.0,
            sprites: Vec::new(),
            dynamic_entities: Vec::new(),
        };
        let project = OmoikaneProjectConfig {
            name: "duplicate-scenes".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![scene.clone(), scene],
        };

        assert_eq!(
            project.render_frame_options_for_scene("main"),
            Err(ProjectConfigError::DuplicateSceneName("main".to_string()))
        );
        assert_eq!(
            project.render_frame_options_for_scene("missing"),
            Err(ProjectConfigError::MissingScene("missing".to_string()))
        );
    }

    #[test]
    fn headless_app_builds_registered_cpu_frame_from_project_config() {
        let mut app = HeadlessApp::default();
        app.spawn_sandbox_entity(SandboxEntityOptions::default());
        app.run_ticks(2);
        let frame_options = CpuFrameOptions::default();
        let project = OmoikaneProjectConfig {
            name: "headless-project".to_string(),
            resources: ProjectResourceConfig {
                textures: vec![ProjectTextureConfig {
                    id: 62,
                    label: "headless_atlas".to_string(),
                    width: 16,
                    height: 16,
                    depth_or_layers: 1,
                    format: ProjectTextureFormat::Rgba8Unorm,
                    usages: vec![ProjectTextureUsage::Sampled],
                }],
                render_pipelines: vec![ProjectRenderPipelineConfig {
                    id: 65,
                    label: "headless_project_pipeline".to_string(),
                    vertex_shader: frame_options.vertex_shader.raw(),
                    fragment_shader: Some(frame_options.fragment_shader.raw()),
                    vertex_buffers: vec![ProjectVertexBufferLayoutConfig {
                        slot: 0,
                        stride_bytes: 16,
                        step_mode: ProjectVertexStepMode::Vertex,
                    }],
                    color_targets: vec![ProjectTextureFormat::Rgba8Unorm],
                    depth_target: None,
                    bind_group_layouts: Vec::new(),
                }],
            },
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 66,
                sandbox_texture: 62,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: Vec::new(),
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: RenderFrameOptions::default().sprite_depth,
                sprites: vec![ProjectSpriteConfig {
                    texture: 62,
                    position: Vector2::new(3.0, 4.0),
                    size: Vector2::new(0.75, 0.75),
                    rotation: 0.0,
                    tint: ProjectColorConfig::default(),
                    depth: 1.0,
                }],
                dynamic_entities: vec![ProjectSceneEntityConfig {
                    id: "scene_frame_actor".to_string(),
                    appearance_name: "scene_frame_actor".to_string(),
                    texture: 120,
                    position: Vector2::new(-2.0, -1.0),
                    rotation: 0.0,
                    prototype: Some("scene_frame_actor".to_string()),
                    attach_local_player: false,
                    physics: None,
                    size: Vector2::new(0.75, 0.5),
                    tint: ProjectColorConfig::default(),
                    depth: 2.0,
                }],
            }],
        };
        app.spawn_project_scene_entities(&project, "main")
            .expect("spawn dynamic scene entities");
        app.run_ticks(2);
        let resource_config = project
            .to_cpu_frame_resource_config(frame_options)
            .expect("project resources");
        let render_options = project
            .render_frame_options_for_scene("main")
            .expect("scene render options");

        let frame = app
            .build_registered_cpu_frame_from_config(render_options, resource_config)
            .expect("frame from project config");

        assert_eq!(frame.extract().sprites().len(), 1);
        assert_eq!(frame.queued().draws().len(), 1);
        assert!(
            app.cpu_frame_resources()
                .expect("resources")
                .catalog()
                .contains_texture(GpuTextureId::new(62))
        );
        assert!(
            app.cpu_frame_resources()
                .expect("resources")
                .catalog()
                .contains_render_pipeline(RenderPipelineId::new(65))
        );
    }

    #[test]
    fn headless_app_builds_project_scene_render_extract_with_static_sprites() {
        let mut app = HeadlessApp::default();
        app.spawn_sandbox_entity(SandboxEntityOptions::default());
        app.run_ticks(2);
        let project = OmoikaneProjectConfig {
            name: "static-sprites".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 101,
                sandbox_texture: 102,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: Vec::new(),
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: vec![
                    ProjectSpriteConfig {
                        texture: 103,
                        position: Vector2::new(1.0, 2.0),
                        size: Vector2::ONE,
                        rotation: 0.0,
                        tint: ProjectColorConfig::default(),
                        depth: 1.0,
                    },
                    ProjectSpriteConfig {
                        texture: 104,
                        position: Vector2::new(-1.0, -2.0),
                        size: Vector2::new(2.0, 1.0),
                        rotation: 0.5,
                        tint: ProjectColorConfig {
                            r: 0.0,
                            g: 1.0,
                            b: 0.0,
                            a: 1.0,
                        },
                        depth: 2.0,
                    },
                ],
                dynamic_entities: Vec::new(),
            }],
        };

        let extract = app
            .build_project_scene_render_extract(&project, "main")
            .expect("project scene extract");

        assert_eq!(extract.sprites().len(), 3);
        assert_eq!(extract.sprites()[1].texture(), ExtractTextureId::new(103));
        assert_eq!(extract.sprites()[2].position(), Vector2::new(-1.0, -2.0));
        assert_eq!(extract.sprites()[2].rotation(), 0.5);
        assert_eq!(extract.validate().expect("valid extract").sprite_count(), 3);
    }

    #[test]
    fn headless_app_rejects_invalid_project_scene_render_data_before_extract() {
        let app = HeadlessApp::default();
        let mut project = OmoikaneProjectConfig {
            name: "invalid-render-data".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 101,
                sandbox_texture: 102,
                world_view: Box2::new(1.0, -1.0, 1.0, 1.0),
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: Vec::new(),
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: Vec::new(),
                dynamic_entities: Vec::new(),
            }],
        };

        assert_eq!(
            app.build_project_scene_render_extract(&project, "main"),
            Err(ProjectSceneRenderError::Project(
                ProjectConfigError::InvalidSceneRenderData {
                    scene: "main".to_string(),
                    reason: "world view must have positive width and height".to_string(),
                }
            ))
        );

        let scene = &mut project.scenes[0];
        scene.world_view = RenderFrameOptions::default().world_view;
        scene.sprite_size = Vector2::new(1.0, 0.0);
        assert_eq!(
            app.build_project_scene_render_extract(&project, "main"),
            Err(ProjectSceneRenderError::Project(
                ProjectConfigError::InvalidSceneRenderData {
                    scene: "main".to_string(),
                    reason: "sandbox sprite size must be finite and positive".to_string(),
                }
            ))
        );

        let scene = &mut project.scenes[0];
        scene.sprite_size = Vector2::ONE;
        scene.sprite_tint.r = f32::NAN;
        assert_eq!(
            app.build_project_scene_render_extract(&project, "main"),
            Err(ProjectSceneRenderError::Project(
                ProjectConfigError::InvalidSceneRenderData {
                    scene: "main".to_string(),
                    reason: "sandbox sprite tint must be finite".to_string(),
                }
            ))
        );
    }

    #[test]
    fn headless_app_rejects_invalid_project_scene_static_sprites_before_extract() {
        let app = HeadlessApp::default();
        let mut project = OmoikaneProjectConfig {
            name: "invalid-static-sprites".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 101,
                sandbox_texture: 102,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: Vec::new(),
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: vec![ProjectSpriteConfig {
                    texture: 103,
                    position: Vector2::ZERO,
                    size: Vector2::new(f32::INFINITY, 1.0),
                    rotation: 0.0,
                    tint: ProjectColorConfig::default(),
                    depth: 1.0,
                }],
                dynamic_entities: Vec::new(),
            }],
        };

        assert_eq!(
            app.build_project_scene_render_extract(&project, "main"),
            Err(ProjectSceneRenderError::Project(
                ProjectConfigError::InvalidSceneSpriteVisual {
                    scene: "main".to_string(),
                    index: 0,
                    reason: "size must be finite and positive".to_string(),
                }
            ))
        );

        let sprite = &mut project.scenes[0].sprites[0];
        sprite.size = Vector2::ONE;
        sprite.depth = f32::NEG_INFINITY;
        assert_eq!(
            app.build_project_scene_render_extract(&project, "main"),
            Err(ProjectSceneRenderError::Project(
                ProjectConfigError::InvalidSceneSpriteVisual {
                    scene: "main".to_string(),
                    index: 0,
                    reason: "depth must be finite".to_string(),
                }
            ))
        );
    }

    #[test]
    fn headless_app_spawns_project_scene_dynamic_entities_for_extract() {
        let mut app = HeadlessApp::default();
        let project = OmoikaneProjectConfig {
            name: "dynamic-scene".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 110,
                sandbox_texture: 111,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: Vec::new(),
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: Vec::new(),
                dynamic_entities: vec![
                    ProjectSceneEntityConfig {
                        id: "actor_a".to_string(),
                        appearance_name: "scene_actor_a".to_string(),
                        texture: 112,
                        position: Vector2::new(2.0, 3.0),
                        rotation: 0.25,
                        prototype: Some("scene_actor".to_string()),
                        attach_local_player: false,
                        physics: None,
                        size: Vector2::new(1.0, 2.0),
                        tint: ProjectColorConfig::default(),
                        depth: 1.0,
                    },
                    ProjectSceneEntityConfig {
                        id: "actor_b".to_string(),
                        appearance_name: "scene_actor_b".to_string(),
                        texture: 113,
                        position: Vector2::new(-2.0, -3.0),
                        rotation: 0.0,
                        prototype: None,
                        attach_local_player: false,
                        physics: None,
                        size: Vector2::new(0.5, 0.5),
                        tint: ProjectColorConfig {
                            r: 0.5,
                            g: 1.0,
                            b: 0.5,
                            a: 1.0,
                        },
                        depth: 2.0,
                    },
                ],
            }],
        };

        let spawned = app
            .spawn_project_scene_entities(&project, "main")
            .expect("spawn scene entities");
        app.run_ticks(2);
        let extract = app
            .build_project_scene_render_extract(&project, "main")
            .expect("project scene extract");

        assert_eq!(spawned.len(), 2);
        assert_eq!(app.project_scene_entities().len(), 2);
        assert_eq!(
            app.project_scene_entity("main", "actor_b")
                .expect("authored scene entity")
                .entity,
            spawned[1].entity
        );
        assert_eq!(extract.sprites().len(), 2);
        assert_eq!(extract.sprites()[0].texture(), ExtractTextureId::new(112));
        assert_eq!(extract.sprites()[0].position(), Vector2::new(2.0, 3.0));
        assert_eq!(extract.sprites()[0].rotation(), 0.25);
        assert_eq!(extract.sprites()[0].size(), Vector2::new(1.0, 2.0));
        assert_eq!(extract.sprites()[1].texture(), ExtractTextureId::new(113));
        assert_eq!(extract.sprites()[1].depth(), 2.0);
    }

    #[test]
    fn headless_app_rejects_duplicate_project_scene_entity_ids() {
        let mut app = HeadlessApp::default();
        let entity = ProjectSceneEntityConfig {
            id: "dupe".to_string(),
            appearance_name: "duplicate_actor".to_string(),
            texture: 112,
            position: Vector2::ZERO,
            rotation: 0.0,
            prototype: None,
            attach_local_player: false,
            physics: None,
            size: Vector2::ONE,
            tint: ProjectColorConfig::default(),
            depth: 0.0,
        };
        let project = OmoikaneProjectConfig {
            name: "duplicate-entity-ids".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 110,
                sandbox_texture: 111,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: Vec::new(),
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: Vec::new(),
                dynamic_entities: vec![entity.clone(), entity],
            }],
        };

        assert_eq!(
            app.spawn_project_scene_entities(&project, "main"),
            Err(ProjectConfigError::DuplicateSceneEntityId {
                scene: "main".to_string(),
                id: "dupe".to_string(),
            })
        );
        assert!(app.project_scene_entities().is_empty());
    }

    #[test]
    fn headless_app_rejects_empty_project_scene_entity_ids() {
        let mut app = HeadlessApp::default();
        let project = OmoikaneProjectConfig {
            name: "empty-entity-id".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 117,
                sandbox_texture: 118,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: Vec::new(),
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: Vec::new(),
                dynamic_entities: vec![ProjectSceneEntityConfig {
                    id: String::new(),
                    appearance_name: "actor".to_string(),
                    texture: 119,
                    position: Vector2::ZERO,
                    rotation: 0.0,
                    prototype: None,
                    attach_local_player: false,
                    physics: None,
                    size: Vector2::ONE,
                    tint: ProjectColorConfig::default(),
                    depth: 1.0,
                }],
            }],
        };

        assert_eq!(
            app.spawn_project_scene_entities(&project, "main"),
            Err(ProjectConfigError::EmptySceneEntityId {
                scene: "main".to_string(),
            })
        );
        assert!(app.project_scene_entities().is_empty());
    }

    #[test]
    fn headless_app_rejects_missing_controlled_project_scene_entity() {
        let mut app = HeadlessApp::default();
        let project = OmoikaneProjectConfig {
            name: "missing-controlled-entity".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 110,
                sandbox_texture: 111,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: Some("missing".to_string()),
                input_bindings: Vec::new(),
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: Vec::new(),
                dynamic_entities: vec![ProjectSceneEntityConfig {
                    id: "present".to_string(),
                    appearance_name: "present_actor".to_string(),
                    texture: 112,
                    position: Vector2::ZERO,
                    rotation: 0.0,
                    prototype: None,
                    attach_local_player: false,
                    physics: None,
                    size: Vector2::ONE,
                    tint: ProjectColorConfig::default(),
                    depth: 0.0,
                }],
            }],
        };

        assert_eq!(
            app.spawn_project_scene_entities(&project, "main"),
            Err(ProjectConfigError::MissingSceneEntityId {
                scene: "main".to_string(),
                id: "missing".to_string(),
            })
        );
        assert!(app.project_scene_entities().is_empty());
    }

    #[test]
    fn headless_app_rejects_empty_controlled_project_scene_entity() {
        let mut app = HeadlessApp::default();
        let project = OmoikaneProjectConfig {
            name: "empty-controlled-entity".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 117,
                sandbox_texture: 118,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: Some(String::new()),
                input_bindings: Vec::new(),
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: Vec::new(),
                dynamic_entities: Vec::new(),
            }],
        };

        assert_eq!(
            app.spawn_project_scene_entities(&project, "main"),
            Err(ProjectConfigError::EmptyControlledSceneEntityId {
                scene: "main".to_string(),
            })
        );
        assert!(app.project_scene_entities().is_empty());
    }

    #[test]
    fn headless_app_rejects_invalid_project_scene_entity_metadata_before_spawn() {
        let mut app = HeadlessApp::default();
        let mut project = OmoikaneProjectConfig {
            name: "invalid-entity-metadata".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 117,
                sandbox_texture: 118,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: Vec::new(),
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: Vec::new(),
                dynamic_entities: vec![ProjectSceneEntityConfig {
                    id: "actor".to_string(),
                    appearance_name: String::new(),
                    texture: 119,
                    position: Vector2::ZERO,
                    rotation: 0.0,
                    prototype: None,
                    attach_local_player: false,
                    physics: None,
                    size: Vector2::ONE,
                    tint: ProjectColorConfig::default(),
                    depth: 1.0,
                }],
            }],
        };

        assert_eq!(
            app.spawn_project_scene_entities(&project, "main"),
            Err(ProjectConfigError::InvalidSceneEntityMetadata {
                scene: "main".to_string(),
                entity: "actor".to_string(),
                reason: "appearance name must not be empty".to_string(),
            })
        );

        let entity = &mut project.scenes[0].dynamic_entities[0];
        entity.appearance_name = "actor".to_string();
        entity.prototype = Some(String::new());
        assert_eq!(
            app.spawn_project_scene_entities(&project, "main"),
            Err(ProjectConfigError::InvalidSceneEntityMetadata {
                scene: "main".to_string(),
                entity: "actor".to_string(),
                reason: "prototype must not be empty when present".to_string(),
            })
        );
        assert!(app.project_scene_entities().is_empty());
    }

    #[test]
    fn headless_app_rejects_invalid_project_scene_entity_visuals_before_spawn() {
        let mut app = HeadlessApp::default();
        let mut project = OmoikaneProjectConfig {
            name: "invalid-entity-visuals".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 117,
                sandbox_texture: 118,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: Vec::new(),
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: Vec::new(),
                dynamic_entities: vec![ProjectSceneEntityConfig {
                    id: "actor".to_string(),
                    appearance_name: "actor".to_string(),
                    texture: 119,
                    position: Vector2::new(f32::NAN, 0.0),
                    rotation: 0.0,
                    prototype: None,
                    attach_local_player: false,
                    physics: None,
                    size: Vector2::ONE,
                    tint: ProjectColorConfig::default(),
                    depth: 1.0,
                }],
            }],
        };

        assert_eq!(
            app.spawn_project_scene_entities(&project, "main"),
            Err(ProjectConfigError::InvalidSceneEntityVisual {
                scene: "main".to_string(),
                entity: "actor".to_string(),
                reason: "position must be finite".to_string(),
            })
        );

        let entity = &mut project.scenes[0].dynamic_entities[0];
        entity.position = Vector2::ZERO;
        entity.rotation = f32::INFINITY;
        assert_eq!(
            app.spawn_project_scene_entities(&project, "main"),
            Err(ProjectConfigError::InvalidSceneEntityVisual {
                scene: "main".to_string(),
                entity: "actor".to_string(),
                reason: "rotation must be finite".to_string(),
            })
        );

        let entity = &mut project.scenes[0].dynamic_entities[0];
        entity.rotation = 0.0;
        entity.size = Vector2::new(0.0, 1.0);
        assert_eq!(
            app.spawn_project_scene_entities(&project, "main"),
            Err(ProjectConfigError::InvalidSceneEntityVisual {
                scene: "main".to_string(),
                entity: "actor".to_string(),
                reason: "size must be finite and positive".to_string(),
            })
        );

        let entity = &mut project.scenes[0].dynamic_entities[0];
        entity.size = Vector2::ONE;
        entity.tint.a = f32::NAN;
        assert_eq!(
            app.spawn_project_scene_entities(&project, "main"),
            Err(ProjectConfigError::InvalidSceneEntityVisual {
                scene: "main".to_string(),
                entity: "actor".to_string(),
                reason: "tint must be finite".to_string(),
            })
        );

        let entity = &mut project.scenes[0].dynamic_entities[0];
        entity.tint = ProjectColorConfig::default();
        entity.depth = f32::NEG_INFINITY;
        assert_eq!(
            app.spawn_project_scene_entities(&project, "main"),
            Err(ProjectConfigError::InvalidSceneEntityVisual {
                scene: "main".to_string(),
                entity: "actor".to_string(),
                reason: "depth must be finite".to_string(),
            })
        );
        assert!(app.project_scene_entities().is_empty());
    }

    #[test]
    fn headless_app_reports_project_scene_input_binding_errors() {
        let mut app = HeadlessApp::default();
        let missing_project = OmoikaneProjectConfig {
            name: "missing-input-binding".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 110,
                sandbox_texture: 111,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: vec![ProjectSceneInputBindingConfig {
                    action: "move_right".to_string(),
                    function: "MoveRight".to_string(),
                }],
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: Vec::new(),
                dynamic_entities: Vec::new(),
            }],
        };
        let duplicate_project = OmoikaneProjectConfig {
            name: "input-bindings".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 110,
                sandbox_texture: 111,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: vec![
                    ProjectSceneInputBindingConfig {
                        action: "move_right".to_string(),
                        function: "MoveRight".to_string(),
                    },
                    ProjectSceneInputBindingConfig {
                        action: "move_right".to_string(),
                        function: "MoveLeft".to_string(),
                    },
                ],
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: Vec::new(),
                dynamic_entities: Vec::new(),
            }],
        };

        assert_eq!(
            app.handle_project_scene_input(&missing_project, "main", "jump", BoundKeyState::Down),
            Err(ProjectConfigError::MissingSceneInputAction {
                scene: "main".to_string(),
                action: "jump".to_string(),
            })
        );
        assert_eq!(
            app.handle_project_scene_input(
                &duplicate_project,
                "main",
                "move_right",
                BoundKeyState::Down
            ),
            Err(ProjectConfigError::DuplicateSceneInputAction {
                scene: "main".to_string(),
                action: "move_right".to_string(),
            })
        );
    }

    #[test]
    fn headless_app_rejects_duplicate_project_scene_input_bindings_before_lookup() {
        let mut app = HeadlessApp::default();
        let project = OmoikaneProjectConfig {
            name: "hidden-duplicate-input-bindings".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 110,
                sandbox_texture: 111,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: vec![
                    ProjectSceneInputBindingConfig {
                        action: "move_right".to_string(),
                        function: "MoveRight".to_string(),
                    },
                    ProjectSceneInputBindingConfig {
                        action: "jump".to_string(),
                        function: "Jump".to_string(),
                    },
                    ProjectSceneInputBindingConfig {
                        action: "jump".to_string(),
                        function: "AltJump".to_string(),
                    },
                ],
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: Vec::new(),
                dynamic_entities: Vec::new(),
            }],
        };

        assert_eq!(
            app.handle_project_scene_input(&project, "main", "move_right", BoundKeyState::Down),
            Err(ProjectConfigError::DuplicateSceneInputAction {
                scene: "main".to_string(),
                action: "jump".to_string(),
            })
        );
    }

    #[test]
    fn headless_app_rejects_invalid_input_bindings_before_scene_spawn() {
        let mut app = HeadlessApp::default();
        let project = OmoikaneProjectConfig {
            name: "invalid-input-binding-spawn".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 110,
                sandbox_texture: 111,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: vec![ProjectSceneInputBindingConfig {
                    action: "move_right".to_string(),
                    function: String::new(),
                }],
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: Vec::new(),
                dynamic_entities: vec![ProjectSceneEntityConfig {
                    id: "actor".to_string(),
                    appearance_name: "actor".to_string(),
                    texture: 112,
                    position: Vector2::ZERO,
                    rotation: 0.0,
                    prototype: None,
                    attach_local_player: false,
                    physics: None,
                    size: Vector2::ONE,
                    tint: ProjectColorConfig::default(),
                    depth: 0.0,
                }],
            }],
        };

        assert_eq!(
            app.spawn_project_scene_entities(&project, "main"),
            Err(ProjectConfigError::EmptySceneInputFunction {
                scene: "main".to_string(),
                action: "move_right".to_string(),
            })
        );
        assert!(app.project_scene_entities().is_empty());
    }

    #[test]
    fn headless_app_rejects_empty_project_scene_input_bindings() {
        let mut app = HeadlessApp::default();
        let mut project = OmoikaneProjectConfig {
            name: "empty-input-bindings".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 110,
                sandbox_texture: 111,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: vec![ProjectSceneInputBindingConfig {
                    action: String::new(),
                    function: "MoveRight".to_string(),
                }],
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: Vec::new(),
                dynamic_entities: Vec::new(),
            }],
        };

        assert_eq!(
            app.handle_project_scene_input(&project, "main", "move_right", BoundKeyState::Down),
            Err(ProjectConfigError::EmptySceneInputAction {
                scene: "main".to_string(),
            })
        );

        project.scenes[0].input_bindings[0] = ProjectSceneInputBindingConfig {
            action: "move_right".to_string(),
            function: String::new(),
        };

        assert_eq!(
            app.handle_project_scene_input(&project, "main", "move_right", BoundKeyState::Down),
            Err(ProjectConfigError::EmptySceneInputFunction {
                scene: "main".to_string(),
                action: "move_right".to_string(),
            })
        );
    }

    #[test]
    fn headless_app_routes_project_scene_controlled_entity_input_through_server() {
        let mut app = HeadlessApp::default();
        let project = OmoikaneProjectConfig {
            name: "controlled-scene".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 114,
                sandbox_texture: 115,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: Some("controlled".to_string()),
                input_bindings: vec![ProjectSceneInputBindingConfig {
                    action: "move_right".to_string(),
                    function: "MoveRight".to_string(),
                }],
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: Vec::new(),
                dynamic_entities: vec![ProjectSceneEntityConfig {
                    id: "controlled".to_string(),
                    appearance_name: "controlled_actor".to_string(),
                    texture: 116,
                    position: Vector2::ZERO,
                    rotation: 0.0,
                    prototype: Some("controlled_actor".to_string()),
                    attach_local_player: false,
                    physics: None,
                    size: Vector2::ONE,
                    tint: ProjectColorConfig::default(),
                    depth: 1.0,
                }],
            }],
        };

        let spawned = app
            .spawn_project_scene_entities(&project, "main")
            .expect("spawn controlled scene entity");
        let controlled = spawned[0].entity;
        assert!(spawned[0].attach_local_player);
        assert_eq!(
            app.server()
                .player_controlled_entity(&app.options().user_id),
            Some(controlled)
        );

        app.run_ticks(2);
        assert_eq!(app.client().controlled_entity(), Some(controlled));
        let _ = app
            .handle_project_scene_input(&project, "main", "move_right", BoundKeyState::Down)
            .expect("project scene input");
        app.run_ticks(3);

        let extract = app
            .build_project_scene_render_extract(&project, "main")
            .expect("project scene extract");
        assert_eq!(extract.sprites().len(), 1);
        assert_eq!(extract.sprites()[0].texture(), ExtractTextureId::new(116));
        assert!(extract.sprites()[0].position().x > 0.0);
    }

    #[test]
    fn headless_app_applies_project_scene_entity_initial_physics() {
        let mut app = HeadlessApp::default();
        let project = OmoikaneProjectConfig {
            name: "physics-scene".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 117,
                sandbox_texture: 118,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: Vec::new(),
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: Vec::new(),
                dynamic_entities: vec![ProjectSceneEntityConfig {
                    id: "drifter".to_string(),
                    appearance_name: "drifting_actor".to_string(),
                    texture: 119,
                    position: Vector2::ZERO,
                    rotation: 0.5,
                    prototype: Some("drifting_actor".to_string()),
                    attach_local_player: false,
                    physics: Some(ProjectScenePhysicsConfig {
                        linear_velocity: Vector2::new(30.0, 0.0),
                        angular_velocity: 0.0,
                        fixtures: vec![
                            ProjectSceneFixtureConfig {
                                id: "body".to_string(),
                                shape: ProjectSceneFixtureShapeConfig::Aabb {
                                    local_bounds: Box2::new(-0.5, -0.5, 0.5, 0.5),
                                    radius: 0.0,
                                },
                                ..ProjectSceneFixtureConfig::default()
                            },
                            ProjectSceneFixtureConfig {
                                id: "sensor".to_string(),
                                shape: ProjectSceneFixtureShapeConfig::Circle {
                                    position: Vector2::ZERO,
                                    radius: 0.25,
                                },
                                hard: false,
                                ..ProjectSceneFixtureConfig::default()
                            },
                        ],
                        ..ProjectScenePhysicsConfig::default()
                    }),
                    size: Vector2::ONE,
                    tint: ProjectColorConfig::default(),
                    depth: 1.0,
                }],
            }],
        };

        let spawned = app
            .spawn_project_scene_entities(&project, "main")
            .expect("spawn physics scene entity");
        let entity = spawned[0].entity;
        assert_eq!(
            app.server().body_linear_velocity(entity),
            Some(Vector2::new(30.0, 0.0))
        );
        assert_eq!(app.server().fixture_count(entity), 2);

        app.run_ticks(3);
        let extract = app
            .build_project_scene_render_extract(&project, "main")
            .expect("project scene extract");

        assert_eq!(extract.sprites().len(), 1);
        assert_eq!(extract.sprites()[0].texture(), ExtractTextureId::new(119));
        assert_eq!(extract.sprites()[0].rotation(), 0.5);
        assert!(extract.sprites()[0].position().x > 0.0);
    }

    #[test]
    fn headless_app_rejects_invalid_project_scene_fixture_ids_before_spawn() {
        let mut app = HeadlessApp::default();
        let mut project = OmoikaneProjectConfig {
            name: "invalid-fixture-ids".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 117,
                sandbox_texture: 118,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: Vec::new(),
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: Vec::new(),
                dynamic_entities: vec![ProjectSceneEntityConfig {
                    id: "actor".to_string(),
                    appearance_name: "actor".to_string(),
                    texture: 119,
                    position: Vector2::ZERO,
                    rotation: 0.0,
                    prototype: None,
                    attach_local_player: false,
                    physics: Some(ProjectScenePhysicsConfig {
                        fixtures: vec![ProjectSceneFixtureConfig {
                            id: String::new(),
                            ..ProjectSceneFixtureConfig::default()
                        }],
                        ..ProjectScenePhysicsConfig::default()
                    }),
                    size: Vector2::ONE,
                    tint: ProjectColorConfig::default(),
                    depth: 1.0,
                }],
            }],
        };

        assert_eq!(
            app.spawn_project_scene_entities(&project, "main"),
            Err(ProjectConfigError::EmptySceneFixtureId {
                scene: "main".to_string(),
                entity: "actor".to_string(),
            })
        );
        assert!(app.project_scene_entities().is_empty());

        let fixtures = &mut project.scenes[0].dynamic_entities[0]
            .physics
            .as_mut()
            .expect("physics")
            .fixtures;
        *fixtures = vec![
            ProjectSceneFixtureConfig {
                id: "body".to_string(),
                ..ProjectSceneFixtureConfig::default()
            },
            ProjectSceneFixtureConfig {
                id: "body".to_string(),
                ..ProjectSceneFixtureConfig::default()
            },
        ];

        assert_eq!(
            app.spawn_project_scene_entities(&project, "main"),
            Err(ProjectConfigError::DuplicateSceneFixtureId {
                scene: "main".to_string(),
                entity: "actor".to_string(),
                id: "body".to_string(),
            })
        );
        assert!(app.project_scene_entities().is_empty());
    }

    #[test]
    fn headless_app_rejects_invalid_project_scene_fixture_shapes_before_spawn() {
        let mut app = HeadlessApp::default();
        let mut project = OmoikaneProjectConfig {
            name: "invalid-fixture-shapes".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 117,
                sandbox_texture: 118,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: Vec::new(),
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: Vec::new(),
                dynamic_entities: vec![ProjectSceneEntityConfig {
                    id: "actor".to_string(),
                    appearance_name: "actor".to_string(),
                    texture: 119,
                    position: Vector2::ZERO,
                    rotation: 0.0,
                    prototype: None,
                    attach_local_player: false,
                    physics: Some(ProjectScenePhysicsConfig {
                        fixtures: vec![ProjectSceneFixtureConfig {
                            id: "body".to_string(),
                            shape: ProjectSceneFixtureShapeConfig::Aabb {
                                local_bounds: Box2::new(1.0, -1.0, 1.0, 1.0),
                                radius: 0.0,
                            },
                            ..ProjectSceneFixtureConfig::default()
                        }],
                        ..ProjectScenePhysicsConfig::default()
                    }),
                    size: Vector2::ONE,
                    tint: ProjectColorConfig::default(),
                    depth: 1.0,
                }],
            }],
        };

        assert_eq!(
            app.spawn_project_scene_entities(&project, "main"),
            Err(ProjectConfigError::InvalidSceneFixtureShape {
                scene: "main".to_string(),
                entity: "actor".to_string(),
                fixture: "body".to_string(),
                reason: "aabb bounds must have positive width and height".to_string(),
            })
        );

        project.scenes[0].dynamic_entities[0]
            .physics
            .as_mut()
            .expect("physics")
            .fixtures[0]
            .shape = ProjectSceneFixtureShapeConfig::Circle {
            position: Vector2::ZERO,
            radius: 0.0,
        };

        assert_eq!(
            app.spawn_project_scene_entities(&project, "main"),
            Err(ProjectConfigError::InvalidSceneFixtureShape {
                scene: "main".to_string(),
                entity: "actor".to_string(),
                fixture: "body".to_string(),
                reason: "circle radius must be finite and positive".to_string(),
            })
        );
        assert!(app.project_scene_entities().is_empty());
    }

    #[test]
    fn headless_app_rejects_invalid_project_scene_physics_values_before_spawn() {
        let mut app = HeadlessApp::default();
        let mut project = OmoikaneProjectConfig {
            name: "invalid-physics-values".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 117,
                sandbox_texture: 118,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: Vec::new(),
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: Vec::new(),
                dynamic_entities: vec![ProjectSceneEntityConfig {
                    id: "actor".to_string(),
                    appearance_name: "actor".to_string(),
                    texture: 119,
                    position: Vector2::ZERO,
                    rotation: 0.0,
                    prototype: None,
                    attach_local_player: false,
                    physics: Some(ProjectScenePhysicsConfig {
                        linear_velocity: Vector2::new(f32::NAN, 0.0),
                        fixtures: vec![ProjectSceneFixtureConfig {
                            id: "body".to_string(),
                            ..ProjectSceneFixtureConfig::default()
                        }],
                        ..ProjectScenePhysicsConfig::default()
                    }),
                    size: Vector2::ONE,
                    tint: ProjectColorConfig::default(),
                    depth: 1.0,
                }],
            }],
        };

        assert_eq!(
            app.spawn_project_scene_entities(&project, "main"),
            Err(ProjectConfigError::InvalidScenePhysicsValue {
                scene: "main".to_string(),
                entity: "actor".to_string(),
                reason: "linear velocity must be finite".to_string(),
            })
        );

        let physics = project.scenes[0].dynamic_entities[0]
            .physics
            .as_mut()
            .expect("physics");
        physics.linear_velocity = Vector2::ZERO;
        physics.angular_velocity = f32::INFINITY;

        assert_eq!(
            app.spawn_project_scene_entities(&project, "main"),
            Err(ProjectConfigError::InvalidScenePhysicsValue {
                scene: "main".to_string(),
                entity: "actor".to_string(),
                reason: "angular velocity must be finite".to_string(),
            })
        );
        assert!(app.project_scene_entities().is_empty());
    }

    #[test]
    fn headless_app_rejects_invalid_project_scene_fixture_material_before_spawn() {
        let mut app = HeadlessApp::default();
        let mut project = OmoikaneProjectConfig {
            name: "invalid-fixture-material".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 117,
                sandbox_texture: 118,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: Vec::new(),
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: Vec::new(),
                dynamic_entities: vec![ProjectSceneEntityConfig {
                    id: "actor".to_string(),
                    appearance_name: "actor".to_string(),
                    texture: 119,
                    position: Vector2::ZERO,
                    rotation: 0.0,
                    prototype: None,
                    attach_local_player: false,
                    physics: Some(ProjectScenePhysicsConfig {
                        fixtures: vec![ProjectSceneFixtureConfig {
                            id: "body".to_string(),
                            friction: -0.1,
                            ..ProjectSceneFixtureConfig::default()
                        }],
                        ..ProjectScenePhysicsConfig::default()
                    }),
                    size: Vector2::ONE,
                    tint: ProjectColorConfig::default(),
                    depth: 1.0,
                }],
            }],
        };

        assert_eq!(
            app.spawn_project_scene_entities(&project, "main"),
            Err(ProjectConfigError::InvalidSceneFixtureMaterial {
                scene: "main".to_string(),
                entity: "actor".to_string(),
                fixture: "body".to_string(),
                reason: "friction must be finite and non-negative".to_string(),
            })
        );

        let fixture = &mut project.scenes[0].dynamic_entities[0]
            .physics
            .as_mut()
            .expect("physics")
            .fixtures[0];
        fixture.friction = 0.0;
        fixture.mass = f32::NAN;

        assert_eq!(
            app.spawn_project_scene_entities(&project, "main"),
            Err(ProjectConfigError::InvalidSceneFixtureMaterial {
                scene: "main".to_string(),
                entity: "actor".to_string(),
                fixture: "body".to_string(),
                reason: "mass must be finite and non-negative".to_string(),
            })
        );
        assert!(app.project_scene_entities().is_empty());
    }

    #[test]
    fn headless_app_builds_registered_cpu_frame_from_project_scene() {
        let mut app = HeadlessApp::default();
        let frame_options = CpuFrameOptions::default();
        let project = OmoikaneProjectConfig {
            name: "scene-frame".to_string(),
            resources: ProjectResourceConfig {
                textures: vec![ProjectTextureConfig {
                    id: 120,
                    label: "scene_frame_atlas".to_string(),
                    width: 32,
                    height: 32,
                    depth_or_layers: 1,
                    format: ProjectTextureFormat::Rgba8Unorm,
                    usages: vec![ProjectTextureUsage::Sampled],
                }],
                render_pipelines: vec![ProjectRenderPipelineConfig {
                    id: 121,
                    label: "scene_frame_pipeline".to_string(),
                    vertex_shader: frame_options.vertex_shader.raw(),
                    fragment_shader: Some(frame_options.fragment_shader.raw()),
                    vertex_buffers: vec![ProjectVertexBufferLayoutConfig {
                        slot: 0,
                        stride_bytes: 16,
                        step_mode: ProjectVertexStepMode::Vertex,
                    }],
                    color_targets: vec![ProjectTextureFormat::Rgba8Unorm],
                    depth_target: None,
                    bind_group_layouts: Vec::new(),
                }],
            },
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 122,
                sandbox_texture: 120,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: Vec::new(),
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: vec![ProjectSpriteConfig {
                    texture: 120,
                    position: Vector2::new(2.0, 1.0),
                    size: Vector2::new(0.5, 0.5),
                    rotation: 0.0,
                    tint: ProjectColorConfig::default(),
                    depth: 1.0,
                }],
                dynamic_entities: vec![ProjectSceneEntityConfig {
                    id: "scene_frame_actor".to_string(),
                    appearance_name: "scene_frame_actor".to_string(),
                    texture: 120,
                    position: Vector2::new(-2.0, -1.0),
                    rotation: 0.0,
                    prototype: Some("scene_frame_actor".to_string()),
                    attach_local_player: false,
                    physics: None,
                    size: Vector2::new(0.75, 0.5),
                    tint: ProjectColorConfig::default(),
                    depth: 2.0,
                }],
            }],
        };
        app.spawn_project_scene_entities(&project, "main")
            .expect("spawn dynamic scene entities");
        app.run_ticks(2);
        let resource_config = project
            .to_cpu_frame_resource_config(frame_options)
            .expect("project resources");

        let frame = app
            .build_project_scene_registered_cpu_frame(&project, "main", resource_config)
            .expect("project scene frame");

        assert_eq!(frame.extract().sprites().len(), 2);
        assert!(
            frame
                .extract()
                .sprites()
                .iter()
                .any(|sprite| sprite.position() == Vector2::new(-2.0, -1.0))
        );
        assert_eq!(frame.queued().draws().len(), 2);
        assert_eq!(
            frame.submission().command_lists()[0].debug_dump(),
            "command_list id=2 label=\"frame_commands\" commands=5\n  0: begin_render_pass label=\"main\" color_targets=[5]\n  1: set_pipeline pipeline=9\n  2: set_vertex_buffer slot=0 buffer=6\n  3: draw vertices=6 instances=2\n  4: end_render_pass\n"
        );
        assert_eq!(app.frame_handle_allocator().next_raw(), 3);
        assert!(
            app.cpu_frame_resources()
                .expect("resources")
                .catalog()
                .contains_render_pipeline(RenderPipelineId::new(121))
        );
    }

    #[test]
    fn headless_app_reports_missing_project_scene_texture_resource_for_cpu_frame() {
        let mut app = HeadlessApp::default();
        let project = OmoikaneProjectConfig {
            name: "missing-scene-texture".to_string(),
            resources: ProjectResourceConfig::default(),
            scenes: vec![ProjectSceneConfig {
                name: "main".to_string(),
                camera: 122,
                sandbox_texture: 120,
                world_view: RenderFrameOptions::default().world_view,
                viewport_size: RenderFrameOptions::default().viewport_size,
                controlled_entity: None,
                input_bindings: Vec::new(),
                sprite_size: RenderFrameOptions::default().sprite_size,
                sprite_tint: ProjectColorConfig::default(),
                sprite_depth: 0.0,
                sprites: vec![ProjectSpriteConfig {
                    texture: 999,
                    position: Vector2::ZERO,
                    size: Vector2::ONE,
                    rotation: 0.0,
                    tint: ProjectColorConfig::default(),
                    depth: 1.0,
                }],
                dynamic_entities: Vec::new(),
            }],
        };

        assert_eq!(
            app.build_project_scene_registered_cpu_frame(
                &project,
                "main",
                CpuFrameResourceConfig::new(CpuFrameOptions::default()),
            ),
            Err(ProjectSceneCpuFrameError::Scene(
                ProjectSceneRenderError::Project(ProjectConfigError::MissingSceneTextureResource {
                    scene: "main".to_string(),
                    texture: 999,
                })
            ))
        );
    }

    #[test]
    fn headless_app_reports_missing_project_scene_for_extract() {
        let app = HeadlessApp::default();
        let project = OmoikaneProjectConfig::default();

        assert_eq!(
            app.build_project_scene_render_extract(&project, "missing"),
            Err(ProjectSceneRenderError::Project(
                ProjectConfigError::MissingScene("missing".to_string())
            ))
        );
    }

    #[test]
    fn headless_app_reports_missing_project_scene_for_cpu_frame() {
        let mut app = HeadlessApp::default();
        let project = OmoikaneProjectConfig::default();

        assert_eq!(
            app.build_project_scene_registered_cpu_frame(
                &project,
                "missing",
                CpuFrameResourceConfig::new(CpuFrameOptions::default()),
            ),
            Err(ProjectSceneCpuFrameError::Scene(
                ProjectSceneRenderError::Project(ProjectConfigError::MissingScene(
                    "missing".to_string()
                ))
            ))
        );
    }

    #[test]
    fn headless_app_shutdown_stops_both_sides() {
        let mut app = HeadlessApp::new(HeadlessAppOptions {
            user_id: "u1".to_string(),
            username: "pedel".to_string(),
            ..HeadlessAppOptions::default()
        });

        app.tick();
        app.shutdown();

        assert!(!app.is_started());
        assert_eq!(app.server_state(), ServerState::Stopped);
        assert_eq!(app.client_run_level(), ClientRunLevel::Initialize);
        assert_eq!(app.ticks_run(), 1);
    }
}
