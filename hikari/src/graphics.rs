use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphicsInstanceId(u64);

impl GraphicsInstanceId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphicsDeviceId(u64);

impl GraphicsDeviceId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SurfaceTargetId(u64);

impl SurfaceTargetId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameId(u64);

impl FrameId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GpuBufferId(u64);

impl GpuBufferId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GpuTextureId(u64);

impl GpuTextureId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShaderModuleId(u64);

impl ShaderModuleId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderPipelineId(u64);

impl RenderPipelineId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComputePipelineId(u64);

impl ComputePipelineId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BindGroupLayoutId(u64);

impl BindGroupLayoutId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BindGroupId(u64);

impl BindGroupId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandListId(u64);

impl CommandListId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GpuSamplerId(u64);

impl GpuSamplerId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphicsInstance {
    id: GraphicsInstanceId,
    label: String,
}

impl GraphicsInstance {
    pub fn new(id: GraphicsInstanceId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
        }
    }

    pub const fn id(&self) -> GraphicsInstanceId {
        self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphicsDevice {
    id: GraphicsDeviceId,
    instance: GraphicsInstanceId,
    label: String,
}

impl GraphicsDevice {
    pub fn new(
        id: GraphicsDeviceId,
        instance: GraphicsInstanceId,
        label: impl Into<String>,
    ) -> Self {
        Self {
            id,
            instance,
            label: label.into(),
        }
    }

    pub const fn id(&self) -> GraphicsDeviceId {
        self.id
    }

    pub const fn instance(&self) -> GraphicsInstanceId {
        self.instance
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceTarget {
    id: SurfaceTargetId,
    label: String,
    size: SurfaceSize,
}

impl SurfaceTarget {
    pub fn new(id: SurfaceTargetId, label: impl Into<String>, size: SurfaceSize) -> Self {
        Self {
            id,
            label: label.into(),
            size,
        }
    }

    pub const fn id(&self) -> SurfaceTargetId {
        self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn size(&self) -> SurfaceSize {
        self.size
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceSize {
    width: u32,
    height: u32,
}

impl SurfaceSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub const fn width(self) -> u32 {
        self.width
    }

    pub const fn height(self) -> u32 {
        self.height
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameContext {
    frame: FrameId,
    device: GraphicsDeviceId,
    target: SurfaceTargetId,
}

impl FrameContext {
    pub const fn new(frame: FrameId, device: GraphicsDeviceId, target: SurfaceTargetId) -> Self {
        Self {
            frame,
            device,
            target,
        }
    }

    pub const fn frame(&self) -> FrameId {
        self.frame
    }

    pub const fn device(&self) -> GraphicsDeviceId {
        self.device
    }

    pub const fn target(&self) -> SurfaceTargetId {
        self.target
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameSubmission {
    frame: FrameContext,
    command_lists: Vec<RenderCommandList>,
}

impl FrameSubmission {
    pub fn new(
        frame: FrameContext,
        command_lists: impl IntoIterator<Item = RenderCommandList>,
    ) -> Result<Self, GraphicsResourceError> {
        let submission = Self {
            frame,
            command_lists: command_lists.into_iter().collect(),
        };
        submission.validate()?;
        Ok(submission)
    }

    pub const fn frame(&self) -> &FrameContext {
        &self.frame
    }

    pub fn command_lists(&self) -> &[RenderCommandList] {
        &self.command_lists
    }

    pub fn debug_dump(&self) -> String {
        let mut output = String::new();
        writeln!(
            output,
            "frame_submission frame={} device={} target={} command_lists={}",
            self.frame.frame().raw(),
            self.frame.device().raw(),
            self.frame.target().raw(),
            self.command_lists.len()
        )
        .expect("writing to a String cannot fail");

        for command_list in &self.command_lists {
            writeln!(
                output,
                "  command_list id={} label=\"{}\" commands={}",
                command_list.id().raw(),
                command_list.label(),
                command_list.commands().len()
            )
            .expect("writing to a String cannot fail");
        }

        output
    }

    pub fn validate(&self) -> Result<(), GraphicsResourceError> {
        if self.command_lists.is_empty() {
            return Err(GraphicsResourceError::EmptyFrameSubmission {
                frame: self.frame.frame,
            });
        }

        let mut command_lists = BTreeSet::new();
        for command_list in &self.command_lists {
            command_list.validate()?;
            if !command_lists.insert(command_list.id()) {
                return Err(GraphicsResourceError::DuplicateSubmittedCommandList {
                    frame: self.frame.frame,
                    command_list: command_list.id(),
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraphicsResourceCatalog {
    devices: BTreeSet<GraphicsDeviceId>,
    surfaces: BTreeSet<SurfaceTargetId>,
    buffers: BTreeMap<GpuBufferId, BTreeSet<BufferUsage>>,
    textures: BTreeMap<GpuTextureId, BTreeSet<TextureUsage>>,
    samplers: BTreeSet<GpuSamplerId>,
    render_pipelines: BTreeMap<RenderPipelineId, RenderPipelineDescriptor>,
    compute_pipelines: BTreeMap<ComputePipelineId, ComputePipelineDescriptor>,
    bind_groups: BTreeMap<BindGroupId, BindGroupDescriptor>,
}

impl GraphicsResourceCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_device(&mut self, device: &GraphicsDevice) -> &mut Self {
        self.devices.insert(device.id());
        self
    }

    pub fn register_surface(&mut self, surface: &SurfaceTarget) -> &mut Self {
        self.surfaces.insert(surface.id());
        self
    }

    pub fn register_buffer(&mut self, buffer: &GpuBuffer) -> &mut Self {
        self.buffers
            .insert(buffer.id(), buffer.descriptor().usages().clone());
        self
    }

    pub fn register_texture(&mut self, texture: &GpuTexture) -> &mut Self {
        self.textures
            .insert(texture.id(), texture.descriptor().usages().clone());
        self
    }

    pub fn register_sampler(&mut self, sampler: GpuSamplerId) -> &mut Self {
        self.samplers.insert(sampler);
        self
    }

    pub fn register_render_pipeline(&mut self, pipeline: &RenderPipeline) -> &mut Self {
        self.render_pipelines
            .insert(pipeline.id(), pipeline.descriptor().clone());
        self
    }

    pub fn register_compute_pipeline(&mut self, pipeline: &ComputePipeline) -> &mut Self {
        self.compute_pipelines
            .insert(pipeline.id(), pipeline.descriptor().clone());
        self
    }

    pub fn register_bind_group(&mut self, bind_group: &BindGroup) -> &mut Self {
        self.bind_groups
            .insert(bind_group.id(), bind_group.descriptor().clone());
        self
    }

    pub fn contains_device(&self, device: GraphicsDeviceId) -> bool {
        self.devices.contains(&device)
    }

    pub fn contains_surface(&self, surface: SurfaceTargetId) -> bool {
        self.surfaces.contains(&surface)
    }

    pub fn contains_buffer(&self, buffer: GpuBufferId) -> bool {
        self.buffers.contains_key(&buffer)
    }

    pub fn contains_texture(&self, texture: GpuTextureId) -> bool {
        self.textures.contains_key(&texture)
    }

    pub fn contains_sampler(&self, sampler: GpuSamplerId) -> bool {
        self.samplers.contains(&sampler)
    }

    pub fn contains_render_pipeline(&self, pipeline: RenderPipelineId) -> bool {
        self.render_pipelines.contains_key(&pipeline)
    }

    pub fn contains_compute_pipeline(&self, pipeline: ComputePipelineId) -> bool {
        self.compute_pipelines.contains_key(&pipeline)
    }

    pub fn contains_bind_group(&self, bind_group: BindGroupId) -> bool {
        self.bind_groups.contains_key(&bind_group)
    }

    pub fn validate_submission(
        &self,
        submission: &FrameSubmission,
    ) -> Result<(), GraphicsResourceError> {
        submission.validate()?;

        if !self.contains_device(submission.frame().device()) {
            return Err(GraphicsResourceError::UnknownSubmittedResource {
                frame: submission.frame().frame(),
                command_list: None,
                command: "frame",
                resource: SubmittedResource::Device(submission.frame().device()),
            });
        }
        if !self.contains_surface(submission.frame().target()) {
            return Err(GraphicsResourceError::UnknownSubmittedResource {
                frame: submission.frame().frame(),
                command_list: None,
                command: "frame",
                resource: SubmittedResource::Surface(submission.frame().target()),
            });
        }

        for command_list in submission.command_lists() {
            self.validate_command_list(submission.frame().frame(), command_list)?;
        }

        Ok(())
    }

    fn validate_command_list(
        &self,
        frame: FrameId,
        command_list: &RenderCommandList,
    ) -> Result<(), GraphicsResourceError> {
        let mut active_render_pipeline = None;
        let mut active_compute_pipeline = None;
        let mut bound_vertex_slots = BTreeSet::new();

        for command in command_list.commands() {
            self.validate_command(
                frame,
                command_list.id(),
                command,
                active_render_pipeline,
                active_compute_pipeline,
                &bound_vertex_slots,
            )?;

            match command {
                RenderCommand::SetPipeline(pipeline) => {
                    active_render_pipeline = Some(*pipeline);
                }
                RenderCommand::SetVertexBuffer { slot, .. } => {
                    bound_vertex_slots.insert(*slot);
                }
                RenderCommand::EndRenderPass => {
                    active_render_pipeline = None;
                    bound_vertex_slots.clear();
                }
                RenderCommand::SetComputePipeline(pipeline) => {
                    active_compute_pipeline = Some(*pipeline);
                }
                RenderCommand::EndComputePass => {
                    active_compute_pipeline = None;
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn validate_command(
        &self,
        frame: FrameId,
        command_list: CommandListId,
        command: &RenderCommand,
        active_render_pipeline: Option<RenderPipelineId>,
        active_compute_pipeline: Option<ComputePipelineId>,
        bound_vertex_slots: &BTreeSet<u32>,
    ) -> Result<(), GraphicsResourceError> {
        match command {
            RenderCommand::BeginRenderPass(pass) => {
                for target in pass.color_targets() {
                    self.require_texture_usage(
                        frame,
                        command_list,
                        command.name(),
                        *target,
                        TextureUsage::RenderTarget,
                    )?;
                }
                if let Some(target) = pass.depth_target() {
                    self.require_texture_usage(
                        frame,
                        command_list,
                        command.name(),
                        target,
                        TextureUsage::DepthStencil,
                    )?;
                }
            }
            RenderCommand::SetPipeline(pipeline) => {
                if !self.contains_render_pipeline(*pipeline) {
                    return Err(GraphicsResourceError::UnknownSubmittedResource {
                        frame,
                        command_list: Some(command_list),
                        command: command.name(),
                        resource: SubmittedResource::RenderPipeline(*pipeline),
                    });
                }
            }
            RenderCommand::SetBindGroup { slot, group } => {
                self.require_bind_group(frame, command_list, command.name(), *group)?;
                if let Some(pipeline) = active_render_pipeline {
                    self.require_render_bind_group_layout(
                        frame,
                        command_list,
                        command.name(),
                        pipeline,
                        *slot,
                        *group,
                    )?;
                }
            }
            RenderCommand::SetComputeBindGroup { slot, group } => {
                self.require_bind_group(frame, command_list, command.name(), *group)?;
                if let Some(pipeline) = active_compute_pipeline {
                    self.require_compute_bind_group_layout(
                        frame,
                        command_list,
                        command.name(),
                        pipeline,
                        *slot,
                        *group,
                    )?;
                }
            }
            RenderCommand::SetVertexBuffer { buffer, .. } => {
                self.require_buffer_usage(
                    frame,
                    command_list,
                    command.name(),
                    *buffer,
                    BufferUsage::Vertex,
                )?;
            }
            RenderCommand::SetIndexBuffer(buffer) => {
                self.require_buffer_usage(
                    frame,
                    command_list,
                    command.name(),
                    *buffer,
                    BufferUsage::Index,
                )?;
            }
            RenderCommand::SetComputePipeline(pipeline) => {
                if !self.contains_compute_pipeline(*pipeline) {
                    return Err(GraphicsResourceError::UnknownSubmittedResource {
                        frame,
                        command_list: Some(command_list),
                        command: command.name(),
                        resource: SubmittedResource::ComputePipeline(*pipeline),
                    });
                }
            }
            RenderCommand::EndRenderPass
            | RenderCommand::BeginComputePass(_)
            | RenderCommand::EndComputePass
            | RenderCommand::Dispatch(_) => {}
            RenderCommand::Draw(_) | RenderCommand::DrawIndexed(_) => {
                if let Some(pipeline) = active_render_pipeline {
                    self.require_render_vertex_buffers(
                        frame,
                        command_list,
                        command.name(),
                        pipeline,
                        bound_vertex_slots,
                    )?;
                }
            }
        }

        Ok(())
    }

    fn require_buffer_usage(
        &self,
        frame: FrameId,
        command_list: CommandListId,
        command: &'static str,
        buffer: GpuBufferId,
        usage: BufferUsage,
    ) -> Result<(), GraphicsResourceError> {
        let Some(usages) = self.buffers.get(&buffer) else {
            return Err(GraphicsResourceError::UnknownSubmittedResource {
                frame,
                command_list: Some(command_list),
                command,
                resource: SubmittedResource::Buffer(buffer),
            });
        };
        if !usages.contains(&usage) {
            return Err(GraphicsResourceError::SubmittedResourceUsageMismatch {
                frame,
                command_list: Some(command_list),
                command,
                resource: SubmittedResource::Buffer(buffer),
                required: SubmittedResourceUsage::Buffer(usage),
            });
        }
        Ok(())
    }

    fn require_texture_usage(
        &self,
        frame: FrameId,
        command_list: CommandListId,
        command: &'static str,
        texture: GpuTextureId,
        usage: TextureUsage,
    ) -> Result<(), GraphicsResourceError> {
        let Some(usages) = self.textures.get(&texture) else {
            return Err(GraphicsResourceError::UnknownSubmittedResource {
                frame,
                command_list: Some(command_list),
                command,
                resource: SubmittedResource::Texture(texture),
            });
        };
        if !usages.contains(&usage) {
            return Err(GraphicsResourceError::SubmittedResourceUsageMismatch {
                frame,
                command_list: Some(command_list),
                command,
                resource: SubmittedResource::Texture(texture),
                required: SubmittedResourceUsage::Texture(usage),
            });
        }
        Ok(())
    }

    fn require_bind_group(
        &self,
        frame: FrameId,
        command_list: CommandListId,
        command: &'static str,
        bind_group: BindGroupId,
    ) -> Result<(), GraphicsResourceError> {
        let Some(descriptor) = self.bind_groups.get(&bind_group) else {
            return Err(GraphicsResourceError::UnknownSubmittedResource {
                frame,
                command_list: Some(command_list),
                command,
                resource: SubmittedResource::BindGroup(bind_group),
            });
        };

        for entry in descriptor.entries() {
            match entry.resource() {
                BoundResource::UniformBuffer(buffer) => self.require_buffer_usage(
                    frame,
                    command_list,
                    command,
                    buffer,
                    BufferUsage::Uniform,
                )?,
                BoundResource::StorageBuffer(buffer) => self.require_buffer_usage(
                    frame,
                    command_list,
                    command,
                    buffer,
                    BufferUsage::Storage,
                )?,
                BoundResource::SampledTexture(texture) => self.require_texture_usage(
                    frame,
                    command_list,
                    command,
                    texture,
                    TextureUsage::Sampled,
                )?,
                BoundResource::StorageTexture(texture) => self.require_texture_usage(
                    frame,
                    command_list,
                    command,
                    texture,
                    TextureUsage::Storage,
                )?,
                BoundResource::Sampler(sampler) => {
                    if !self.contains_sampler(sampler) {
                        return Err(GraphicsResourceError::UnknownSubmittedResource {
                            frame,
                            command_list: Some(command_list),
                            command,
                            resource: SubmittedResource::Sampler(sampler),
                        });
                    }
                }
            }
        }

        Ok(())
    }

    fn require_render_bind_group_layout(
        &self,
        frame: FrameId,
        command_list: CommandListId,
        command: &'static str,
        pipeline: RenderPipelineId,
        slot: u32,
        bind_group: BindGroupId,
    ) -> Result<(), GraphicsResourceError> {
        let pipeline_id = pipeline;
        let Some(pipeline) = self.render_pipelines.get(&pipeline_id) else {
            return Ok(());
        };
        self.require_pipeline_bind_group_layout(
            SubmittedBindGroupLayoutCheck {
                frame,
                command_list,
                command,
                pipeline: SubmittedResource::RenderPipeline(pipeline_id),
            },
            pipeline.bind_group_layouts(),
            slot,
            bind_group,
        )
    }

    fn require_compute_bind_group_layout(
        &self,
        frame: FrameId,
        command_list: CommandListId,
        command: &'static str,
        pipeline: ComputePipelineId,
        slot: u32,
        bind_group: BindGroupId,
    ) -> Result<(), GraphicsResourceError> {
        let pipeline_id = pipeline;
        let Some(pipeline) = self.compute_pipelines.get(&pipeline_id) else {
            return Ok(());
        };
        self.require_pipeline_bind_group_layout(
            SubmittedBindGroupLayoutCheck {
                frame,
                command_list,
                command,
                pipeline: SubmittedResource::ComputePipeline(pipeline_id),
            },
            pipeline.bind_group_layouts(),
            slot,
            bind_group,
        )
    }

    fn require_pipeline_bind_group_layout(
        &self,
        context: SubmittedBindGroupLayoutCheck,
        layouts: &[BindGroupLayoutId],
        slot: u32,
        bind_group: BindGroupId,
    ) -> Result<(), GraphicsResourceError> {
        let Some(expected) = layouts.get(slot as usize) else {
            return Err(GraphicsResourceError::SubmittedBindGroupSlotOutOfRange {
                frame: context.frame,
                command_list: context.command_list,
                command: context.command,
                pipeline: context.pipeline,
                slot,
                bind_group,
            });
        };
        let actual = self
            .bind_groups
            .get(&bind_group)
            .expect("bind group existence is validated before layout compatibility")
            .layout();
        if actual != *expected {
            return Err(GraphicsResourceError::SubmittedBindGroupLayoutMismatch {
                frame: context.frame,
                command_list: context.command_list,
                command: context.command,
                pipeline: context.pipeline,
                slot,
                bind_group,
                expected: *expected,
                actual,
            });
        }
        Ok(())
    }

    fn require_render_vertex_buffers(
        &self,
        frame: FrameId,
        command_list: CommandListId,
        command: &'static str,
        pipeline: RenderPipelineId,
        bound_vertex_slots: &BTreeSet<u32>,
    ) -> Result<(), GraphicsResourceError> {
        let Some(pipeline_descriptor) = self.render_pipelines.get(&pipeline) else {
            return Ok(());
        };
        for layout in pipeline_descriptor.vertex_buffers() {
            if !bound_vertex_slots.contains(&layout.slot()) {
                return Err(GraphicsResourceError::SubmittedVertexBufferSlotMissing {
                    frame,
                    command_list,
                    command,
                    pipeline,
                    slot: layout.slot(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SubmittedBindGroupLayoutCheck {
    frame: FrameId,
    command_list: CommandListId,
    command: &'static str,
    pipeline: SubmittedResource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuBuffer {
    id: GpuBufferId,
    label: String,
    descriptor: GpuBufferDescriptor,
}

impl GpuBuffer {
    pub fn new(id: GpuBufferId, label: impl Into<String>, size_bytes: u64) -> Self {
        let label = label.into();
        Self {
            id,
            label: label.clone(),
            descriptor: GpuBufferDescriptor::new(label, size_bytes, [BufferUsage::Storage]),
        }
    }

    pub fn from_descriptor(
        id: GpuBufferId,
        descriptor: GpuBufferDescriptor,
    ) -> Result<Self, GraphicsResourceError> {
        descriptor.validate()?;
        Ok(Self {
            id,
            label: descriptor.label.clone(),
            descriptor,
        })
    }

    pub const fn id(&self) -> GpuBufferId {
        self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn size_bytes(&self) -> u64 {
        self.descriptor.size_bytes
    }

    pub fn descriptor(&self) -> &GpuBufferDescriptor {
        &self.descriptor
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuBufferDescriptor {
    label: String,
    size_bytes: u64,
    usages: BTreeSet<BufferUsage>,
}

impl GpuBufferDescriptor {
    pub fn new(
        label: impl Into<String>,
        size_bytes: u64,
        usages: impl IntoIterator<Item = BufferUsage>,
    ) -> Self {
        Self {
            label: label.into(),
            size_bytes,
            usages: usages.into_iter().collect(),
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    pub fn usages(&self) -> &BTreeSet<BufferUsage> {
        &self.usages
    }

    pub fn validate(&self) -> Result<(), GraphicsResourceError> {
        if self.size_bytes == 0 {
            return Err(GraphicsResourceError::EmptyBuffer {
                label: self.label.clone(),
            });
        }
        if self.usages.is_empty() {
            return Err(GraphicsResourceError::MissingBufferUsage {
                label: self.label.clone(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BufferUsage {
    Vertex,
    Index,
    Uniform,
    Storage,
    TransferSrc,
    TransferDst,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuTexture {
    id: GpuTextureId,
    label: String,
    descriptor: GpuTextureDescriptor,
}

impl GpuTexture {
    pub fn new(id: GpuTextureId, label: impl Into<String>, size: TextureSize) -> Self {
        let label = label.into();
        Self {
            id,
            label: label.clone(),
            descriptor: GpuTextureDescriptor::new(
                label,
                size,
                TextureFormat::Rgba8Unorm,
                [TextureUsage::Sampled],
            ),
        }
    }

    pub fn from_descriptor(
        id: GpuTextureId,
        descriptor: GpuTextureDescriptor,
    ) -> Result<Self, GraphicsResourceError> {
        descriptor.validate()?;
        Ok(Self {
            id,
            label: descriptor.label.clone(),
            descriptor,
        })
    }

    pub const fn id(&self) -> GpuTextureId {
        self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn size(&self) -> TextureSize {
        self.descriptor.size
    }

    pub fn descriptor(&self) -> &GpuTextureDescriptor {
        &self.descriptor
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuTextureDescriptor {
    label: String,
    size: TextureSize,
    format: TextureFormat,
    usages: BTreeSet<TextureUsage>,
}

impl GpuTextureDescriptor {
    pub fn new(
        label: impl Into<String>,
        size: TextureSize,
        format: TextureFormat,
        usages: impl IntoIterator<Item = TextureUsage>,
    ) -> Self {
        Self {
            label: label.into(),
            size,
            format,
            usages: usages.into_iter().collect(),
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn size(&self) -> TextureSize {
        self.size
    }

    pub const fn format(&self) -> TextureFormat {
        self.format
    }

    pub fn usages(&self) -> &BTreeSet<TextureUsage> {
        &self.usages
    }

    pub fn validate(&self) -> Result<(), GraphicsResourceError> {
        if self.size.width == 0 || self.size.height == 0 || self.size.depth_or_layers == 0 {
            return Err(GraphicsResourceError::EmptyTexture {
                label: self.label.clone(),
            });
        }
        if self.usages.is_empty() {
            return Err(GraphicsResourceError::MissingTextureUsage {
                label: self.label.clone(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    Rgba8Unorm,
    Bgra8Unorm,
    Depth24Stencil8,
    Depth32Float,
}

impl TextureFormat {
    pub const fn is_depth(self) -> bool {
        matches!(self, Self::Depth24Stencil8 | Self::Depth32Float)
    }

    pub const fn is_color(self) -> bool {
        !self.is_depth()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextureUsage {
    Sampled,
    RenderTarget,
    DepthStencil,
    Storage,
    TransferSrc,
    TransferDst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureSize {
    width: u32,
    height: u32,
    depth_or_layers: u32,
}

impl TextureSize {
    pub const fn new(width: u32, height: u32, depth_or_layers: u32) -> Self {
        Self {
            width,
            height,
            depth_or_layers,
        }
    }

    pub const fn width(self) -> u32 {
        self.width
    }

    pub const fn height(self) -> u32 {
        self.height
    }

    pub const fn depth_or_layers(self) -> u32 {
        self.depth_or_layers
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderModule {
    id: ShaderModuleId,
    label: String,
    descriptor: ShaderModuleDescriptor,
}

impl ShaderModule {
    pub fn new(id: ShaderModuleId, label: impl Into<String>, stage: ShaderStage) -> Self {
        let label = label.into();
        Self {
            id,
            label: label.clone(),
            descriptor: ShaderModuleDescriptor::new(
                label,
                stage,
                ShaderSourceKind::Wgsl,
                "main",
                "",
            ),
        }
    }

    pub fn from_descriptor(
        id: ShaderModuleId,
        descriptor: ShaderModuleDescriptor,
    ) -> Result<Self, GraphicsResourceError> {
        descriptor.validate()?;
        Ok(Self {
            id,
            label: descriptor.label.clone(),
            descriptor,
        })
    }

    pub const fn id(&self) -> ShaderModuleId {
        self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn stage(&self) -> ShaderStage {
        self.descriptor.stage
    }

    pub fn descriptor(&self) -> &ShaderModuleDescriptor {
        &self.descriptor
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderModuleDescriptor {
    label: String,
    stage: ShaderStage,
    source_kind: ShaderSourceKind,
    entry_point: String,
    source: String,
}

impl ShaderModuleDescriptor {
    pub fn new(
        label: impl Into<String>,
        stage: ShaderStage,
        source_kind: ShaderSourceKind,
        entry_point: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            stage,
            source_kind,
            entry_point: entry_point.into(),
            source: source.into(),
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn stage(&self) -> ShaderStage {
        self.stage
    }

    pub const fn source_kind(&self) -> ShaderSourceKind {
        self.source_kind
    }

    pub fn entry_point(&self) -> &str {
        &self.entry_point
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn validate(&self) -> Result<(), GraphicsResourceError> {
        if self.entry_point.trim().is_empty() {
            return Err(GraphicsResourceError::MissingShaderEntryPoint {
                label: self.label.clone(),
            });
        }
        if self.source.trim().is_empty() {
            return Err(GraphicsResourceError::EmptyShaderSource {
                label: self.label.clone(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderSourceKind {
    Wgsl,
    SpirV,
    RustGpu,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderPipeline {
    id: RenderPipelineId,
    label: String,
    descriptor: RenderPipelineDescriptor,
}

impl RenderPipeline {
    pub fn from_descriptor(
        id: RenderPipelineId,
        descriptor: RenderPipelineDescriptor,
    ) -> Result<Self, GraphicsResourceError> {
        descriptor.validate()?;
        Ok(Self {
            id,
            label: descriptor.label.clone(),
            descriptor,
        })
    }

    pub const fn id(&self) -> RenderPipelineId {
        self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn descriptor(&self) -> &RenderPipelineDescriptor {
        &self.descriptor
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderPipelineDescriptor {
    label: String,
    vertex_shader: PipelineShader,
    fragment_shader: Option<PipelineShader>,
    vertex_buffers: Vec<VertexBufferLayout>,
    bind_group_layouts: Vec<BindGroupLayoutId>,
    color_targets: Vec<TextureFormat>,
    depth_target: Option<TextureFormat>,
}

impl RenderPipelineDescriptor {
    pub fn new(
        label: impl Into<String>,
        vertex_shader: PipelineShader,
        fragment_shader: Option<PipelineShader>,
        vertex_buffers: impl IntoIterator<Item = VertexBufferLayout>,
        color_targets: impl IntoIterator<Item = TextureFormat>,
        depth_target: Option<TextureFormat>,
    ) -> Self {
        Self {
            label: label.into(),
            vertex_shader,
            fragment_shader,
            vertex_buffers: vertex_buffers.into_iter().collect(),
            bind_group_layouts: Vec::new(),
            color_targets: color_targets.into_iter().collect(),
            depth_target,
        }
    }

    pub fn with_bind_group_layouts(
        mut self,
        bind_group_layouts: impl IntoIterator<Item = BindGroupLayoutId>,
    ) -> Self {
        self.bind_group_layouts = bind_group_layouts.into_iter().collect();
        self
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn vertex_shader(&self) -> PipelineShader {
        self.vertex_shader
    }

    pub const fn fragment_shader(&self) -> Option<PipelineShader> {
        self.fragment_shader
    }

    pub fn vertex_buffers(&self) -> &[VertexBufferLayout] {
        &self.vertex_buffers
    }

    pub fn color_targets(&self) -> &[TextureFormat] {
        &self.color_targets
    }

    pub fn bind_group_layouts(&self) -> &[BindGroupLayoutId] {
        &self.bind_group_layouts
    }

    pub const fn depth_target(&self) -> Option<TextureFormat> {
        self.depth_target
    }

    pub fn validate(&self) -> Result<(), GraphicsResourceError> {
        if self.vertex_shader.stage != ShaderStage::Vertex {
            return Err(GraphicsResourceError::InvalidPipelineShaderStage {
                label: self.label.clone(),
                expected: ShaderStage::Vertex,
                actual: self.vertex_shader.stage,
            });
        }
        if let Some(fragment_shader) = self.fragment_shader
            && fragment_shader.stage != ShaderStage::Fragment
        {
            return Err(GraphicsResourceError::InvalidPipelineShaderStage {
                label: self.label.clone(),
                expected: ShaderStage::Fragment,
                actual: fragment_shader.stage,
            });
        }
        if self.color_targets.is_empty() && self.depth_target.is_none() {
            return Err(GraphicsResourceError::MissingPipelineTarget {
                label: self.label.clone(),
            });
        }
        for format in &self.color_targets {
            if !format.is_color() {
                return Err(GraphicsResourceError::InvalidColorTargetFormat {
                    label: self.label.clone(),
                    format: *format,
                });
            }
        }
        if let Some(format) = self.depth_target
            && !format.is_depth()
        {
            return Err(GraphicsResourceError::InvalidDepthTargetFormat {
                label: self.label.clone(),
                format,
            });
        }
        for layout in &self.vertex_buffers {
            if layout.stride_bytes == 0 {
                return Err(GraphicsResourceError::EmptyVertexBufferLayout {
                    label: self.label.clone(),
                    slot: layout.slot,
                });
            }
        }
        let mut bind_group_layouts = BTreeSet::new();
        for layout in &self.bind_group_layouts {
            if !bind_group_layouts.insert(*layout) {
                return Err(GraphicsResourceError::DuplicatePipelineBindGroupLayout {
                    label: self.label.clone(),
                    layout: *layout,
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputePipeline {
    id: ComputePipelineId,
    label: String,
    descriptor: ComputePipelineDescriptor,
}

impl ComputePipeline {
    pub fn from_descriptor(
        id: ComputePipelineId,
        descriptor: ComputePipelineDescriptor,
    ) -> Result<Self, GraphicsResourceError> {
        descriptor.validate()?;
        Ok(Self {
            id,
            label: descriptor.label.clone(),
            descriptor,
        })
    }

    pub const fn id(&self) -> ComputePipelineId {
        self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn descriptor(&self) -> &ComputePipelineDescriptor {
        &self.descriptor
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputePipelineDescriptor {
    label: String,
    compute_shader: PipelineShader,
    bind_group_layouts: Vec<BindGroupLayoutId>,
}

impl ComputePipelineDescriptor {
    pub fn new(label: impl Into<String>, compute_shader: PipelineShader) -> Self {
        Self {
            label: label.into(),
            compute_shader,
            bind_group_layouts: Vec::new(),
        }
    }

    pub fn with_bind_group_layouts(
        mut self,
        bind_group_layouts: impl IntoIterator<Item = BindGroupLayoutId>,
    ) -> Self {
        self.bind_group_layouts = bind_group_layouts.into_iter().collect();
        self
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn compute_shader(&self) -> PipelineShader {
        self.compute_shader
    }

    pub fn bind_group_layouts(&self) -> &[BindGroupLayoutId] {
        &self.bind_group_layouts
    }

    pub fn validate(&self) -> Result<(), GraphicsResourceError> {
        if self.compute_shader.stage != ShaderStage::Compute {
            return Err(GraphicsResourceError::InvalidPipelineShaderStage {
                label: self.label.clone(),
                expected: ShaderStage::Compute,
                actual: self.compute_shader.stage,
            });
        }

        let mut bind_group_layouts = BTreeSet::new();
        for layout in &self.bind_group_layouts {
            if !bind_group_layouts.insert(*layout) {
                return Err(GraphicsResourceError::DuplicatePipelineBindGroupLayout {
                    label: self.label.clone(),
                    layout: *layout,
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipelineShader {
    module: ShaderModuleId,
    stage: ShaderStage,
}

impl PipelineShader {
    pub const fn new(module: ShaderModuleId, stage: ShaderStage) -> Self {
        Self { module, stage }
    }

    pub const fn module(self) -> ShaderModuleId {
        self.module
    }

    pub const fn stage(self) -> ShaderStage {
        self.stage
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VertexBufferLayout {
    slot: u32,
    stride_bytes: u64,
    step_mode: VertexStepMode,
}

impl VertexBufferLayout {
    pub const fn new(slot: u32, stride_bytes: u64, step_mode: VertexStepMode) -> Self {
        Self {
            slot,
            stride_bytes,
            step_mode,
        }
    }

    pub const fn slot(self) -> u32 {
        self.slot
    }

    pub const fn stride_bytes(self) -> u64 {
        self.stride_bytes
    }

    pub const fn step_mode(self) -> VertexStepMode {
        self.step_mode
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexStepMode {
    Vertex,
    Instance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindGroupLayout {
    id: BindGroupLayoutId,
    label: String,
    descriptor: BindGroupLayoutDescriptor,
}

impl BindGroupLayout {
    pub fn from_descriptor(
        id: BindGroupLayoutId,
        descriptor: BindGroupLayoutDescriptor,
    ) -> Result<Self, GraphicsResourceError> {
        descriptor.validate()?;
        Ok(Self {
            id,
            label: descriptor.label.clone(),
            descriptor,
        })
    }

    pub const fn id(&self) -> BindGroupLayoutId {
        self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn descriptor(&self) -> &BindGroupLayoutDescriptor {
        &self.descriptor
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindGroupLayoutDescriptor {
    label: String,
    entries: Vec<BindGroupLayoutEntry>,
}

impl BindGroupLayoutDescriptor {
    pub fn new(
        label: impl Into<String>,
        entries: impl IntoIterator<Item = BindGroupLayoutEntry>,
    ) -> Self {
        Self {
            label: label.into(),
            entries: entries.into_iter().collect(),
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn entries(&self) -> &[BindGroupLayoutEntry] {
        &self.entries
    }

    pub fn validate(&self) -> Result<(), GraphicsResourceError> {
        if self.entries.is_empty() {
            return Err(GraphicsResourceError::EmptyBindGroupLayout {
                label: self.label.clone(),
            });
        }

        let mut bindings = BTreeSet::new();
        for entry in &self.entries {
            if !bindings.insert(entry.binding) {
                return Err(GraphicsResourceError::DuplicateBindGroupBinding {
                    label: self.label.clone(),
                    binding: entry.binding,
                });
            }
            if entry.visibility.is_empty() {
                return Err(GraphicsResourceError::MissingBindGroupVisibility {
                    label: self.label.clone(),
                    binding: entry.binding,
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindGroupLayoutEntry {
    binding: u32,
    resource: BindingResourceKind,
    visibility: BTreeSet<ShaderStage>,
}

impl BindGroupLayoutEntry {
    pub fn new(
        binding: u32,
        resource: BindingResourceKind,
        visibility: impl IntoIterator<Item = ShaderStage>,
    ) -> Self {
        Self {
            binding,
            resource,
            visibility: visibility.into_iter().collect(),
        }
    }

    pub const fn binding(&self) -> u32 {
        self.binding
    }

    pub const fn resource(&self) -> BindingResourceKind {
        self.resource
    }

    pub fn visibility(&self) -> &BTreeSet<ShaderStage> {
        &self.visibility
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingResourceKind {
    UniformBuffer,
    StorageBuffer,
    SampledTexture,
    StorageTexture,
    Sampler,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindGroup {
    id: BindGroupId,
    label: String,
    descriptor: BindGroupDescriptor,
}

impl BindGroup {
    pub fn from_descriptor(
        id: BindGroupId,
        descriptor: BindGroupDescriptor,
    ) -> Result<Self, GraphicsResourceError> {
        descriptor.validate()?;
        Ok(Self {
            id,
            label: descriptor.label.clone(),
            descriptor,
        })
    }

    pub const fn id(&self) -> BindGroupId {
        self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn descriptor(&self) -> &BindGroupDescriptor {
        &self.descriptor
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindGroupDescriptor {
    label: String,
    layout: BindGroupLayoutId,
    entries: Vec<BindGroupEntry>,
}

impl BindGroupDescriptor {
    pub fn new(
        label: impl Into<String>,
        layout: BindGroupLayoutId,
        entries: impl IntoIterator<Item = BindGroupEntry>,
    ) -> Self {
        Self {
            label: label.into(),
            layout,
            entries: entries.into_iter().collect(),
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn layout(&self) -> BindGroupLayoutId {
        self.layout
    }

    pub fn entries(&self) -> &[BindGroupEntry] {
        &self.entries
    }

    pub fn validate(&self) -> Result<(), GraphicsResourceError> {
        if self.entries.is_empty() {
            return Err(GraphicsResourceError::EmptyBindGroup {
                label: self.label.clone(),
            });
        }

        let mut bindings = BTreeSet::new();
        for entry in &self.entries {
            if !bindings.insert(entry.binding) {
                return Err(GraphicsResourceError::DuplicateBindGroupResourceBinding {
                    label: self.label.clone(),
                    binding: entry.binding,
                });
            }
        }

        Ok(())
    }

    pub fn validate_against_layout(
        &self,
        layout: &BindGroupLayoutDescriptor,
    ) -> Result<(), GraphicsResourceError> {
        self.validate()?;
        layout.validate()?;

        for layout_entry in layout.entries() {
            if !self
                .entries
                .iter()
                .any(|entry| entry.binding == layout_entry.binding)
            {
                return Err(GraphicsResourceError::MissingBindGroupResource {
                    label: self.label.clone(),
                    binding: layout_entry.binding,
                });
            }
        }

        for entry in &self.entries {
            let Some(layout_entry) = layout
                .entries()
                .iter()
                .find(|layout_entry| layout_entry.binding == entry.binding)
            else {
                return Err(GraphicsResourceError::UnknownBindGroupResource {
                    label: self.label.clone(),
                    binding: entry.binding,
                });
            };
            let actual = entry.resource.kind();
            if actual != layout_entry.resource {
                return Err(GraphicsResourceError::MismatchedBindGroupResource {
                    label: self.label.clone(),
                    binding: entry.binding,
                    expected: layout_entry.resource,
                    actual,
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindGroupEntry {
    binding: u32,
    resource: BoundResource,
}

impl BindGroupEntry {
    pub const fn new(binding: u32, resource: BoundResource) -> Self {
        Self { binding, resource }
    }

    pub const fn binding(self) -> u32 {
        self.binding
    }

    pub const fn resource(self) -> BoundResource {
        self.resource
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundResource {
    UniformBuffer(GpuBufferId),
    StorageBuffer(GpuBufferId),
    SampledTexture(GpuTextureId),
    StorageTexture(GpuTextureId),
    Sampler(GpuSamplerId),
}

impl BoundResource {
    pub const fn kind(self) -> BindingResourceKind {
        match self {
            Self::UniformBuffer(_) => BindingResourceKind::UniformBuffer,
            Self::StorageBuffer(_) => BindingResourceKind::StorageBuffer,
            Self::SampledTexture(_) => BindingResourceKind::SampledTexture,
            Self::StorageTexture(_) => BindingResourceKind::StorageTexture,
            Self::Sampler(_) => BindingResourceKind::Sampler,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderCommandList {
    id: CommandListId,
    label: String,
    commands: Vec<RenderCommand>,
}

impl RenderCommandList {
    pub fn new(
        id: CommandListId,
        label: impl Into<String>,
        commands: impl IntoIterator<Item = RenderCommand>,
    ) -> Result<Self, GraphicsResourceError> {
        let label = label.into();
        let commands = commands.into_iter().collect();
        let list = Self {
            id,
            label,
            commands,
        };
        list.validate()?;
        Ok(list)
    }

    pub const fn id(&self) -> CommandListId {
        self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn commands(&self) -> &[RenderCommand] {
        &self.commands
    }

    pub fn builder(id: CommandListId, label: impl Into<String>) -> RenderCommandListBuilder {
        RenderCommandListBuilder::new(id, label)
    }

    pub fn debug_dump(&self) -> String {
        let mut output = String::new();
        writeln!(
            output,
            "command_list id={} label=\"{}\" commands={}",
            self.id.raw(),
            self.label,
            self.commands.len()
        )
        .expect("writing to a String cannot fail");

        for (index, command) in self.commands.iter().enumerate() {
            write!(output, "  {index}: ").expect("writing to a String cannot fail");
            write_command_dump(&mut output, command);
            output.push('\n');
        }

        output
    }

    pub fn validate(&self) -> Result<(), GraphicsResourceError> {
        if self.commands.is_empty() {
            return Err(GraphicsResourceError::EmptyCommandList {
                label: self.label.clone(),
            });
        }

        let mut active_pass: Option<ActiveCommandPass<'_>> = None;
        let mut render_pipeline_bound = false;
        let mut index_buffer_bound = false;
        let mut compute_pipeline_bound = false;

        for command in &self.commands {
            match command {
                RenderCommand::BeginRenderPass(pass) => {
                    if let Some(active) = active_pass {
                        return Err(GraphicsResourceError::RenderPassAlreadyActive {
                            label: self.label.clone(),
                            active_pass: active.label().to_string(),
                        });
                    }
                    pass.validate()?;
                    active_pass = Some(ActiveCommandPass::Render(pass.label()));
                    render_pipeline_bound = false;
                    index_buffer_bound = false;
                }
                RenderCommand::EndRenderPass => {
                    if !matches!(active_pass, Some(ActiveCommandPass::Render(_))) {
                        return Err(GraphicsResourceError::RenderPassNotActive {
                            label: self.label.clone(),
                            command: command.name(),
                        });
                    }
                    active_pass = None;
                    render_pipeline_bound = false;
                    index_buffer_bound = false;
                }
                RenderCommand::SetPipeline(_) => {
                    if !matches!(active_pass, Some(ActiveCommandPass::Render(_))) {
                        return Err(GraphicsResourceError::RenderPassNotActive {
                            label: self.label.clone(),
                            command: command.name(),
                        });
                    }
                    render_pipeline_bound = true;
                }
                RenderCommand::SetBindGroup { .. } | RenderCommand::SetVertexBuffer { .. } => {
                    if !matches!(active_pass, Some(ActiveCommandPass::Render(_))) {
                        return Err(GraphicsResourceError::RenderPassNotActive {
                            label: self.label.clone(),
                            command: command.name(),
                        });
                    }
                }
                RenderCommand::SetIndexBuffer(_) => {
                    if !matches!(active_pass, Some(ActiveCommandPass::Render(_))) {
                        return Err(GraphicsResourceError::RenderPassNotActive {
                            label: self.label.clone(),
                            command: command.name(),
                        });
                    }
                    index_buffer_bound = true;
                }
                RenderCommand::Draw(draw) => {
                    validate_draw_state(
                        &self.label,
                        command.name(),
                        active_pass,
                        render_pipeline_bound,
                    )?;
                    draw.validate(&self.label, command.name())?;
                }
                RenderCommand::DrawIndexed(draw) => {
                    validate_draw_state(
                        &self.label,
                        command.name(),
                        active_pass,
                        render_pipeline_bound,
                    )?;
                    if !index_buffer_bound {
                        return Err(GraphicsResourceError::DrawIndexedWithoutIndexBuffer {
                            label: self.label.clone(),
                            command: command.name(),
                            active_pass: active_pass
                                .map(|pass| pass.label().to_string())
                                .unwrap_or_default(),
                        });
                    }
                    draw.validate(&self.label, command.name())?;
                }
                RenderCommand::BeginComputePass(pass) => {
                    if let Some(active) = active_pass {
                        return Err(GraphicsResourceError::ComputePassAlreadyActive {
                            label: self.label.clone(),
                            active_pass: active.label().to_string(),
                        });
                    }
                    active_pass = Some(ActiveCommandPass::Compute(pass.label()));
                    compute_pipeline_bound = false;
                }
                RenderCommand::EndComputePass => {
                    if !matches!(active_pass, Some(ActiveCommandPass::Compute(_))) {
                        return Err(GraphicsResourceError::ComputePassNotActive {
                            label: self.label.clone(),
                            command: command.name(),
                        });
                    }
                    active_pass = None;
                    compute_pipeline_bound = false;
                }
                RenderCommand::SetComputePipeline(_) => {
                    if !matches!(active_pass, Some(ActiveCommandPass::Compute(_))) {
                        return Err(GraphicsResourceError::ComputePassNotActive {
                            label: self.label.clone(),
                            command: command.name(),
                        });
                    }
                    compute_pipeline_bound = true;
                }
                RenderCommand::SetComputeBindGroup { .. } => {
                    if !matches!(active_pass, Some(ActiveCommandPass::Compute(_))) {
                        return Err(GraphicsResourceError::ComputePassNotActive {
                            label: self.label.clone(),
                            command: command.name(),
                        });
                    }
                }
                RenderCommand::Dispatch(dispatch) => {
                    validate_dispatch_state(
                        &self.label,
                        command.name(),
                        active_pass,
                        compute_pipeline_bound,
                    )?;
                    dispatch.validate(&self.label, command.name())?;
                }
            }
        }

        if let Some(active) = active_pass {
            return match active {
                ActiveCommandPass::Render(active_label) => {
                    Err(GraphicsResourceError::RenderPassLeftOpen {
                        label: self.label.clone(),
                        active_pass: active_label.to_string(),
                    })
                }
                ActiveCommandPass::Compute(active_label) => {
                    Err(GraphicsResourceError::ComputePassLeftOpen {
                        label: self.label.clone(),
                        active_pass: active_label.to_string(),
                    })
                }
            };
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderCommandListBuilder {
    id: CommandListId,
    label: String,
    commands: Vec<RenderCommand>,
}

impl RenderCommandListBuilder {
    pub fn new(id: CommandListId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            commands: Vec::new(),
        }
    }

    pub const fn id(&self) -> CommandListId {
        self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn commands(&self) -> &[RenderCommand] {
        &self.commands
    }

    pub fn render_pass(
        &mut self,
        descriptor: RenderPassDescriptor,
        encode: impl FnOnce(&mut RenderPassBuilder) -> Result<(), GraphicsResourceError>,
    ) -> Result<&mut Self, GraphicsResourceError> {
        descriptor.validate()?;

        let mut pass = RenderPassBuilder::new(self.label.clone());
        encode(&mut pass)?;

        self.commands
            .push(RenderCommand::BeginRenderPass(descriptor));
        self.commands.extend(pass.finish());
        self.commands.push(RenderCommand::EndRenderPass);
        Ok(self)
    }

    pub fn compute_pass(
        &mut self,
        descriptor: ComputePassDescriptor,
        encode: impl FnOnce(&mut ComputePassBuilder) -> Result<(), GraphicsResourceError>,
    ) -> Result<&mut Self, GraphicsResourceError> {
        let mut pass = ComputePassBuilder::new(self.label.clone());
        encode(&mut pass)?;

        self.commands
            .push(RenderCommand::BeginComputePass(descriptor));
        self.commands.extend(pass.finish());
        self.commands.push(RenderCommand::EndComputePass);
        Ok(self)
    }

    pub fn finish(self) -> Result<RenderCommandList, GraphicsResourceError> {
        RenderCommandList::new(self.id, self.label, self.commands)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderPassBuilder {
    command_list_label: String,
    commands: Vec<RenderCommand>,
    pipeline_bound: bool,
    index_buffer_bound: bool,
}

impl RenderPassBuilder {
    fn new(command_list_label: String) -> Self {
        Self {
            command_list_label,
            commands: Vec::new(),
            pipeline_bound: false,
            index_buffer_bound: false,
        }
    }

    pub fn commands(&self) -> &[RenderCommand] {
        &self.commands
    }

    pub fn set_pipeline(&mut self, pipeline: RenderPipelineId) -> &mut Self {
        self.commands.push(RenderCommand::SetPipeline(pipeline));
        self.pipeline_bound = true;
        self
    }

    pub fn set_bind_group(&mut self, slot: u32, group: BindGroupId) -> &mut Self {
        self.commands
            .push(RenderCommand::SetBindGroup { slot, group });
        self
    }

    pub fn set_vertex_buffer(&mut self, slot: u32, buffer: GpuBufferId) -> &mut Self {
        self.commands
            .push(RenderCommand::SetVertexBuffer { slot, buffer });
        self
    }

    pub fn set_index_buffer(&mut self, buffer: GpuBufferId) -> &mut Self {
        self.commands.push(RenderCommand::SetIndexBuffer(buffer));
        self.index_buffer_bound = true;
        self
    }

    pub fn draw(&mut self, draw: DrawCall) -> Result<&mut Self, GraphicsResourceError> {
        validate_draw_state(
            &self.command_list_label,
            "draw",
            Some(ActiveCommandPass::Render("builder")),
            self.pipeline_bound,
        )?;
        draw.validate(&self.command_list_label, "draw")?;
        self.commands.push(RenderCommand::Draw(draw));
        Ok(self)
    }

    pub fn draw_indexed(
        &mut self,
        draw: IndexedDrawCall,
    ) -> Result<&mut Self, GraphicsResourceError> {
        validate_draw_state(
            &self.command_list_label,
            "draw_indexed",
            Some(ActiveCommandPass::Render("builder")),
            self.pipeline_bound,
        )?;
        if !self.index_buffer_bound {
            return Err(GraphicsResourceError::DrawIndexedWithoutIndexBuffer {
                label: self.command_list_label.clone(),
                command: "draw_indexed",
                active_pass: "builder".to_string(),
            });
        }
        draw.validate(&self.command_list_label, "draw_indexed")?;
        self.commands.push(RenderCommand::DrawIndexed(draw));
        Ok(self)
    }

    fn finish(self) -> Vec<RenderCommand> {
        self.commands
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputePassBuilder {
    command_list_label: String,
    commands: Vec<RenderCommand>,
    pipeline_bound: bool,
}

impl ComputePassBuilder {
    fn new(command_list_label: String) -> Self {
        Self {
            command_list_label,
            commands: Vec::new(),
            pipeline_bound: false,
        }
    }

    pub fn commands(&self) -> &[RenderCommand] {
        &self.commands
    }

    pub fn set_pipeline(&mut self, pipeline: ComputePipelineId) -> &mut Self {
        self.commands
            .push(RenderCommand::SetComputePipeline(pipeline));
        self.pipeline_bound = true;
        self
    }

    pub fn set_bind_group(&mut self, slot: u32, group: BindGroupId) -> &mut Self {
        self.commands
            .push(RenderCommand::SetComputeBindGroup { slot, group });
        self
    }

    pub fn dispatch(&mut self, dispatch: DispatchCall) -> Result<&mut Self, GraphicsResourceError> {
        validate_dispatch_state(
            &self.command_list_label,
            "dispatch",
            Some(ActiveCommandPass::Compute("builder")),
            self.pipeline_bound,
        )?;
        dispatch.validate(&self.command_list_label, "dispatch")?;
        self.commands.push(RenderCommand::Dispatch(dispatch));
        Ok(self)
    }

    fn finish(self) -> Vec<RenderCommand> {
        self.commands
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveCommandPass<'a> {
    Render(&'a str),
    Compute(&'a str),
}

impl<'a> ActiveCommandPass<'a> {
    fn label(self) -> &'a str {
        match self {
            Self::Render(label) | Self::Compute(label) => label,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderCommand {
    BeginRenderPass(RenderPassDescriptor),
    EndRenderPass,
    SetPipeline(RenderPipelineId),
    SetBindGroup { slot: u32, group: BindGroupId },
    SetVertexBuffer { slot: u32, buffer: GpuBufferId },
    SetIndexBuffer(GpuBufferId),
    Draw(DrawCall),
    DrawIndexed(IndexedDrawCall),
    BeginComputePass(ComputePassDescriptor),
    EndComputePass,
    SetComputePipeline(ComputePipelineId),
    SetComputeBindGroup { slot: u32, group: BindGroupId },
    Dispatch(DispatchCall),
}

impl RenderCommand {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::BeginRenderPass(_) => "begin_render_pass",
            Self::EndRenderPass => "end_render_pass",
            Self::SetPipeline(_) => "set_pipeline",
            Self::SetBindGroup { .. } => "set_bind_group",
            Self::SetVertexBuffer { .. } => "set_vertex_buffer",
            Self::SetIndexBuffer(_) => "set_index_buffer",
            Self::Draw(_) => "draw",
            Self::DrawIndexed(_) => "draw_indexed",
            Self::BeginComputePass(_) => "begin_compute_pass",
            Self::EndComputePass => "end_compute_pass",
            Self::SetComputePipeline(_) => "set_compute_pipeline",
            Self::SetComputeBindGroup { .. } => "set_compute_bind_group",
            Self::Dispatch(_) => "dispatch",
        }
    }
}

fn write_command_dump(output: &mut String, command: &RenderCommand) {
    match command {
        RenderCommand::BeginRenderPass(pass) => {
            write!(
                output,
                "begin_render_pass label=\"{}\" color_targets=[",
                pass.label()
            )
            .expect("writing to a String cannot fail");
            write_texture_ids(output, pass.color_targets());
            output.push(']');
            if let Some(depth_target) = pass.depth_target() {
                write!(output, " depth_target={}", depth_target.raw())
                    .expect("writing to a String cannot fail");
            }
        }
        RenderCommand::EndRenderPass => output.push_str("end_render_pass"),
        RenderCommand::SetPipeline(pipeline) => {
            write!(output, "set_pipeline pipeline={}", pipeline.raw())
                .expect("writing to a String cannot fail");
        }
        RenderCommand::SetBindGroup { slot, group } => {
            write!(output, "set_bind_group slot={slot} group={}", group.raw())
                .expect("writing to a String cannot fail");
        }
        RenderCommand::SetVertexBuffer { slot, buffer } => {
            write!(
                output,
                "set_vertex_buffer slot={slot} buffer={}",
                buffer.raw()
            )
            .expect("writing to a String cannot fail");
        }
        RenderCommand::SetIndexBuffer(buffer) => {
            write!(output, "set_index_buffer buffer={}", buffer.raw())
                .expect("writing to a String cannot fail");
        }
        RenderCommand::Draw(draw) => {
            write!(
                output,
                "draw vertices={} instances={}",
                draw.vertices(),
                draw.instances()
            )
            .expect("writing to a String cannot fail");
        }
        RenderCommand::DrawIndexed(draw) => {
            write!(
                output,
                "draw_indexed indices={} instances={}",
                draw.indices(),
                draw.instances()
            )
            .expect("writing to a String cannot fail");
        }
        RenderCommand::BeginComputePass(pass) => {
            write!(output, "begin_compute_pass label=\"{}\"", pass.label())
                .expect("writing to a String cannot fail");
        }
        RenderCommand::EndComputePass => output.push_str("end_compute_pass"),
        RenderCommand::SetComputePipeline(pipeline) => {
            write!(output, "set_compute_pipeline pipeline={}", pipeline.raw())
                .expect("writing to a String cannot fail");
        }
        RenderCommand::SetComputeBindGroup { slot, group } => {
            write!(
                output,
                "set_compute_bind_group slot={slot} group={}",
                group.raw()
            )
            .expect("writing to a String cannot fail");
        }
        RenderCommand::Dispatch(dispatch) => {
            write!(
                output,
                "dispatch workgroups={}x{}x{}",
                dispatch.workgroups_x(),
                dispatch.workgroups_y(),
                dispatch.workgroups_z()
            )
            .expect("writing to a String cannot fail");
        }
    }
}

fn write_texture_ids(output: &mut String, textures: &[GpuTextureId]) {
    for (index, texture) in textures.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        write!(output, "{}", texture.raw()).expect("writing to a String cannot fail");
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderPassDescriptor {
    label: String,
    color_targets: Vec<GpuTextureId>,
    depth_target: Option<GpuTextureId>,
}

impl RenderPassDescriptor {
    pub fn new(
        label: impl Into<String>,
        color_targets: impl IntoIterator<Item = GpuTextureId>,
        depth_target: Option<GpuTextureId>,
    ) -> Self {
        Self {
            label: label.into(),
            color_targets: color_targets.into_iter().collect(),
            depth_target,
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn color_targets(&self) -> &[GpuTextureId] {
        &self.color_targets
    }

    pub const fn depth_target(&self) -> Option<GpuTextureId> {
        self.depth_target
    }

    pub fn validate(&self) -> Result<(), GraphicsResourceError> {
        if self.color_targets.is_empty() && self.depth_target.is_none() {
            return Err(GraphicsResourceError::MissingRenderPassTarget {
                label: self.label.clone(),
            });
        }

        let mut color_targets = BTreeSet::new();
        for target in &self.color_targets {
            if !color_targets.insert(*target) {
                return Err(GraphicsResourceError::DuplicateRenderPassColorTarget {
                    label: self.label.clone(),
                    target: *target,
                });
            }
        }

        if let Some(depth_target) = self.depth_target
            && color_targets.contains(&depth_target)
        {
            return Err(GraphicsResourceError::RenderPassDepthAlsoColorTarget {
                label: self.label.clone(),
                target: depth_target,
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputePassDescriptor {
    label: String,
}

impl ComputePassDescriptor {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawCall {
    vertices: u32,
    instances: u32,
}

impl DrawCall {
    pub const fn new(vertices: u32, instances: u32) -> Self {
        Self {
            vertices,
            instances,
        }
    }

    pub const fn vertices(self) -> u32 {
        self.vertices
    }

    pub const fn instances(self) -> u32 {
        self.instances
    }

    fn validate(self, label: &str, command: &'static str) -> Result<(), GraphicsResourceError> {
        if self.vertices == 0 || self.instances == 0 {
            return Err(GraphicsResourceError::EmptyDraw {
                label: label.to_string(),
                command,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexedDrawCall {
    indices: u32,
    instances: u32,
}

impl IndexedDrawCall {
    pub const fn new(indices: u32, instances: u32) -> Self {
        Self { indices, instances }
    }

    pub const fn indices(self) -> u32 {
        self.indices
    }

    pub const fn instances(self) -> u32 {
        self.instances
    }

    fn validate(self, label: &str, command: &'static str) -> Result<(), GraphicsResourceError> {
        if self.indices == 0 || self.instances == 0 {
            return Err(GraphicsResourceError::EmptyDraw {
                label: label.to_string(),
                command,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DispatchCall {
    workgroups_x: u32,
    workgroups_y: u32,
    workgroups_z: u32,
}

impl DispatchCall {
    pub const fn new(workgroups_x: u32, workgroups_y: u32, workgroups_z: u32) -> Self {
        Self {
            workgroups_x,
            workgroups_y,
            workgroups_z,
        }
    }

    pub const fn workgroups_x(self) -> u32 {
        self.workgroups_x
    }

    pub const fn workgroups_y(self) -> u32 {
        self.workgroups_y
    }

    pub const fn workgroups_z(self) -> u32 {
        self.workgroups_z
    }

    fn validate(self, label: &str, command: &'static str) -> Result<(), GraphicsResourceError> {
        if self.workgroups_x == 0 || self.workgroups_y == 0 || self.workgroups_z == 0 {
            return Err(GraphicsResourceError::EmptyDispatch {
                label: label.to_string(),
                command,
            });
        }
        Ok(())
    }
}

fn validate_draw_state(
    label: &str,
    command: &'static str,
    active_pass: Option<ActiveCommandPass<'_>>,
    pipeline_bound: bool,
) -> Result<(), GraphicsResourceError> {
    if !matches!(active_pass, Some(ActiveCommandPass::Render(_))) {
        return Err(GraphicsResourceError::RenderPassNotActive {
            label: label.to_string(),
            command,
        });
    }
    if !pipeline_bound {
        return Err(GraphicsResourceError::DrawWithoutPipeline {
            label: label.to_string(),
            command,
            active_pass: active_pass
                .map(|pass| pass.label().to_string())
                .unwrap_or_default(),
        });
    }
    Ok(())
}

fn validate_dispatch_state(
    label: &str,
    command: &'static str,
    active_pass: Option<ActiveCommandPass<'_>>,
    pipeline_bound: bool,
) -> Result<(), GraphicsResourceError> {
    if !matches!(active_pass, Some(ActiveCommandPass::Compute(_))) {
        return Err(GraphicsResourceError::ComputePassNotActive {
            label: label.to_string(),
            command,
        });
    }
    if !pipeline_bound {
        return Err(GraphicsResourceError::DispatchWithoutPipeline {
            label: label.to_string(),
            command,
            active_pass: active_pass
                .map(|pass| pass.label().to_string())
                .unwrap_or_default(),
        });
    }
    Ok(())
}

impl fmt::Display for ShaderStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vertex => f.write_str("vertex"),
            Self::Fragment => f.write_str("fragment"),
            Self::Compute => f.write_str("compute"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphicsResourceError {
    EmptyBuffer {
        label: String,
    },
    MissingBufferUsage {
        label: String,
    },
    EmptyTexture {
        label: String,
    },
    MissingTextureUsage {
        label: String,
    },
    EmptyShaderSource {
        label: String,
    },
    MissingShaderEntryPoint {
        label: String,
    },
    InvalidPipelineShaderStage {
        label: String,
        expected: ShaderStage,
        actual: ShaderStage,
    },
    MissingPipelineTarget {
        label: String,
    },
    InvalidColorTargetFormat {
        label: String,
        format: TextureFormat,
    },
    InvalidDepthTargetFormat {
        label: String,
        format: TextureFormat,
    },
    EmptyVertexBufferLayout {
        label: String,
        slot: u32,
    },
    EmptyBindGroupLayout {
        label: String,
    },
    DuplicateBindGroupBinding {
        label: String,
        binding: u32,
    },
    MissingBindGroupVisibility {
        label: String,
        binding: u32,
    },
    DuplicatePipelineBindGroupLayout {
        label: String,
        layout: BindGroupLayoutId,
    },
    EmptyBindGroup {
        label: String,
    },
    DuplicateBindGroupResourceBinding {
        label: String,
        binding: u32,
    },
    MissingBindGroupResource {
        label: String,
        binding: u32,
    },
    UnknownBindGroupResource {
        label: String,
        binding: u32,
    },
    MismatchedBindGroupResource {
        label: String,
        binding: u32,
        expected: BindingResourceKind,
        actual: BindingResourceKind,
    },
    EmptyCommandList {
        label: String,
    },
    EmptyFrameSubmission {
        frame: FrameId,
    },
    DuplicateSubmittedCommandList {
        frame: FrameId,
        command_list: CommandListId,
    },
    UnknownSubmittedResource {
        frame: FrameId,
        command_list: Option<CommandListId>,
        command: &'static str,
        resource: SubmittedResource,
    },
    SubmittedResourceUsageMismatch {
        frame: FrameId,
        command_list: Option<CommandListId>,
        command: &'static str,
        resource: SubmittedResource,
        required: SubmittedResourceUsage,
    },
    SubmittedBindGroupSlotOutOfRange {
        frame: FrameId,
        command_list: CommandListId,
        command: &'static str,
        pipeline: SubmittedResource,
        slot: u32,
        bind_group: BindGroupId,
    },
    SubmittedBindGroupLayoutMismatch {
        frame: FrameId,
        command_list: CommandListId,
        command: &'static str,
        pipeline: SubmittedResource,
        slot: u32,
        bind_group: BindGroupId,
        expected: BindGroupLayoutId,
        actual: BindGroupLayoutId,
    },
    SubmittedVertexBufferSlotMissing {
        frame: FrameId,
        command_list: CommandListId,
        command: &'static str,
        pipeline: RenderPipelineId,
        slot: u32,
    },
    RenderPassAlreadyActive {
        label: String,
        active_pass: String,
    },
    RenderPassNotActive {
        label: String,
        command: &'static str,
    },
    RenderPassLeftOpen {
        label: String,
        active_pass: String,
    },
    MissingRenderPassTarget {
        label: String,
    },
    DuplicateRenderPassColorTarget {
        label: String,
        target: GpuTextureId,
    },
    RenderPassDepthAlsoColorTarget {
        label: String,
        target: GpuTextureId,
    },
    DrawWithoutPipeline {
        label: String,
        command: &'static str,
        active_pass: String,
    },
    DrawIndexedWithoutIndexBuffer {
        label: String,
        command: &'static str,
        active_pass: String,
    },
    EmptyDraw {
        label: String,
        command: &'static str,
    },
    ComputePassAlreadyActive {
        label: String,
        active_pass: String,
    },
    ComputePassNotActive {
        label: String,
        command: &'static str,
    },
    ComputePassLeftOpen {
        label: String,
        active_pass: String,
    },
    DispatchWithoutPipeline {
        label: String,
        command: &'static str,
        active_pass: String,
    },
    EmptyDispatch {
        label: String,
        command: &'static str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmittedResource {
    Device(GraphicsDeviceId),
    Surface(SurfaceTargetId),
    Buffer(GpuBufferId),
    Texture(GpuTextureId),
    Sampler(GpuSamplerId),
    RenderPipeline(RenderPipelineId),
    ComputePipeline(ComputePipelineId),
    BindGroup(BindGroupId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmittedResourceUsage {
    Buffer(BufferUsage),
    Texture(TextureUsage),
}

#[cfg(test)]
mod tests {
    use super::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupId, BindGroupLayout,
        BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindGroupLayoutId, BindingResourceKind,
        BoundResource, BufferUsage, CommandListId, ComputePassDescriptor, ComputePipeline,
        ComputePipelineDescriptor, ComputePipelineId, DispatchCall, DrawCall, FrameContext,
        FrameId, FrameSubmission, GpuBuffer, GpuBufferDescriptor, GpuBufferId, GpuTexture,
        GpuTextureDescriptor, GpuTextureId, GraphicsResourceCatalog, GraphicsResourceError,
        IndexedDrawCall, RenderCommand, RenderCommandList, RenderCommandListBuilder,
        RenderPassDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderModuleId,
        ShaderSourceKind, ShaderStage, SubmittedResource, SubmittedResourceUsage, SurfaceSize,
        SurfaceTarget, SurfaceTargetId, TextureFormat, TextureSize, TextureUsage,
    };
    use super::{
        PipelineShader, RenderPipeline, RenderPipelineDescriptor, RenderPipelineId,
        VertexBufferLayout, VertexStepMode,
    };

    #[test]
    fn structural_handles_preserve_identity_and_labels() {
        let target =
            SurfaceTarget::new(SurfaceTargetId::new(1), "headless", SurfaceSize::new(8, 9));
        let frame = FrameContext::new(
            FrameId::new(2),
            super::GraphicsDeviceId::new(3),
            target.id(),
        );
        let buffer = GpuBuffer::new(GpuBufferId::new(4), "vertices", 64);
        let texture = GpuTexture::new(GpuTextureId::new(5), "color", TextureSize::new(8, 9, 1));
        let shader = ShaderModule::new(ShaderModuleId::new(6), "main", ShaderStage::Vertex);

        assert_eq!(target.size().width(), 8);
        assert_eq!(frame.target(), target.id());
        assert_eq!(buffer.size_bytes(), 64);
        assert_eq!(texture.size().height(), 9);
        assert_eq!(shader.stage().to_string(), "vertex");
    }

    fn simple_render_command_list(id: u64) -> RenderCommandList {
        RenderCommandList::new(
            CommandListId::new(id),
            "simple",
            [
                RenderCommand::BeginRenderPass(RenderPassDescriptor::new(
                    "main",
                    [GpuTextureId::new(100)],
                    None,
                )),
                RenderCommand::SetPipeline(RenderPipelineId::new(101)),
                RenderCommand::Draw(DrawCall::new(3, 1)),
                RenderCommand::EndRenderPass,
            ],
        )
        .expect("valid simple render command list")
    }

    #[test]
    fn frame_submissions_validate_command_lists_for_submit_boundary() {
        let frame = FrameContext::new(
            FrameId::new(102),
            super::GraphicsDeviceId::new(103),
            SurfaceTargetId::new(104),
        );
        let submission = FrameSubmission::new(frame.clone(), [simple_render_command_list(105)])
            .expect("valid frame submission");

        assert_eq!(submission.frame().frame(), FrameId::new(102));
        assert_eq!(submission.command_lists().len(), 1);
        assert_eq!(
            FrameSubmission::new(frame.clone(), []).unwrap_err(),
            GraphicsResourceError::EmptyFrameSubmission {
                frame: FrameId::new(102),
            }
        );
        assert_eq!(
            FrameSubmission::new(
                frame,
                [
                    simple_render_command_list(106),
                    simple_render_command_list(106),
                ],
            )
            .unwrap_err(),
            GraphicsResourceError::DuplicateSubmittedCommandList {
                frame: FrameId::new(102),
                command_list: CommandListId::new(106),
            }
        );
    }

    #[test]
    fn resource_catalog_validates_submitted_resource_handles() {
        let instance = super::GraphicsInstanceId::new(107);
        let device = super::GraphicsDevice::new(super::GraphicsDeviceId::new(108), instance, "gpu");
        let surface = SurfaceTarget::new(
            SurfaceTargetId::new(109),
            "surface",
            SurfaceSize::new(640, 480),
        );
        let texture = GpuTexture::from_descriptor(
            GpuTextureId::new(100),
            GpuTextureDescriptor::new(
                "framebuffer",
                TextureSize::new(640, 480, 1),
                TextureFormat::Rgba8Unorm,
                [TextureUsage::RenderTarget],
            ),
        )
        .expect("valid render target texture");
        let pipeline = RenderPipeline::from_descriptor(
            RenderPipelineId::new(101),
            RenderPipelineDescriptor::new(
                "pipeline",
                PipelineShader::new(ShaderModuleId::new(110), ShaderStage::Vertex),
                Some(PipelineShader::new(
                    ShaderModuleId::new(111),
                    ShaderStage::Fragment,
                )),
                [],
                [TextureFormat::Rgba8Unorm],
                None,
            ),
        )
        .expect("valid render pipeline");
        let frame = FrameContext::new(FrameId::new(112), device.id(), surface.id());
        let submission = FrameSubmission::new(frame, [simple_render_command_list(113)])
            .expect("valid frame submission");

        let mut catalog = GraphicsResourceCatalog::new();
        catalog
            .register_device(&device)
            .register_surface(&surface)
            .register_texture(&texture)
            .register_render_pipeline(&pipeline);

        assert!(catalog.validate_submission(&submission).is_ok());

        let missing_pipeline_submission = FrameSubmission::new(
            FrameContext::new(FrameId::new(114), device.id(), surface.id()),
            [RenderCommandList::new(
                CommandListId::new(115),
                "missing_pipeline",
                [
                    RenderCommand::BeginRenderPass(RenderPassDescriptor::new(
                        "main",
                        [texture.id()],
                        None,
                    )),
                    RenderCommand::SetPipeline(RenderPipelineId::new(116)),
                    RenderCommand::Draw(DrawCall::new(3, 1)),
                    RenderCommand::EndRenderPass,
                ],
            )
            .expect("valid command list shape")],
        )
        .expect("valid submission shape");

        assert_eq!(
            catalog
                .validate_submission(&missing_pipeline_submission)
                .unwrap_err(),
            GraphicsResourceError::UnknownSubmittedResource {
                frame: FrameId::new(114),
                command_list: Some(CommandListId::new(115)),
                command: "set_pipeline",
                resource: SubmittedResource::RenderPipeline(RenderPipelineId::new(116)),
            }
        );

        let missing_target = FrameSubmission::new(
            FrameContext::new(FrameId::new(117), device.id(), SurfaceTargetId::new(118)),
            [simple_render_command_list(119)],
        )
        .expect("valid frame submission");

        assert_eq!(
            catalog.validate_submission(&missing_target).unwrap_err(),
            GraphicsResourceError::UnknownSubmittedResource {
                frame: FrameId::new(117),
                command_list: None,
                command: "frame",
                resource: SubmittedResource::Surface(SurfaceTargetId::new(118)),
            }
        );
    }

    #[test]
    fn resource_catalog_validates_submitted_resource_usages() {
        let device = super::GraphicsDevice::new(
            super::GraphicsDeviceId::new(120),
            super::GraphicsInstanceId::new(121),
            "gpu",
        );
        let surface = SurfaceTarget::new(
            SurfaceTargetId::new(122),
            "surface",
            SurfaceSize::new(16, 16),
        );
        let sampled_texture = GpuTexture::new(
            GpuTextureId::new(100),
            "not_renderable",
            TextureSize::new(16, 16, 1),
        );
        let uniform_buffer = GpuBuffer::from_descriptor(
            GpuBufferId::new(123),
            GpuBufferDescriptor::new("uniform", 64, [BufferUsage::Uniform]),
        )
        .expect("valid uniform buffer");
        let pipeline = RenderPipeline::from_descriptor(
            RenderPipelineId::new(101),
            RenderPipelineDescriptor::new(
                "pipeline",
                PipelineShader::new(ShaderModuleId::new(124), ShaderStage::Vertex),
                None,
                [],
                [TextureFormat::Rgba8Unorm],
                None,
            ),
        )
        .expect("valid render pipeline");
        let render_target_submission = FrameSubmission::new(
            FrameContext::new(FrameId::new(125), device.id(), surface.id()),
            [simple_render_command_list(126)],
        )
        .expect("valid submission shape");

        let mut catalog = GraphicsResourceCatalog::new();
        catalog
            .register_device(&device)
            .register_surface(&surface)
            .register_texture(&sampled_texture)
            .register_render_pipeline(&pipeline);

        assert_eq!(
            catalog
                .validate_submission(&render_target_submission)
                .unwrap_err(),
            GraphicsResourceError::SubmittedResourceUsageMismatch {
                frame: FrameId::new(125),
                command_list: Some(CommandListId::new(126)),
                command: "begin_render_pass",
                resource: SubmittedResource::Texture(GpuTextureId::new(100)),
                required: SubmittedResourceUsage::Texture(TextureUsage::RenderTarget),
            }
        );

        let vertex_submission = FrameSubmission::new(
            FrameContext::new(FrameId::new(127), device.id(), surface.id()),
            [RenderCommandList::new(
                CommandListId::new(128),
                "vertex_usage",
                [
                    RenderCommand::BeginRenderPass(RenderPassDescriptor::new(
                        "main",
                        [GpuTextureId::new(129)],
                        None,
                    )),
                    RenderCommand::SetPipeline(pipeline.id()),
                    RenderCommand::SetVertexBuffer {
                        slot: 0,
                        buffer: uniform_buffer.id(),
                    },
                    RenderCommand::Draw(DrawCall::new(3, 1)),
                    RenderCommand::EndRenderPass,
                ],
            )
            .expect("valid command list shape")],
        )
        .expect("valid submission shape");
        let render_target = GpuTexture::from_descriptor(
            GpuTextureId::new(129),
            GpuTextureDescriptor::new(
                "target",
                TextureSize::new(16, 16, 1),
                TextureFormat::Rgba8Unorm,
                [TextureUsage::RenderTarget],
            ),
        )
        .expect("valid render target");
        catalog
            .register_texture(&render_target)
            .register_buffer(&uniform_buffer);

        assert_eq!(
            catalog.validate_submission(&vertex_submission).unwrap_err(),
            GraphicsResourceError::SubmittedResourceUsageMismatch {
                frame: FrameId::new(127),
                command_list: Some(CommandListId::new(128)),
                command: "set_vertex_buffer",
                resource: SubmittedResource::Buffer(GpuBufferId::new(123)),
                required: SubmittedResourceUsage::Buffer(BufferUsage::Vertex),
            }
        );
    }

    #[test]
    fn resource_catalog_validates_pipeline_bind_group_layouts() {
        let device = super::GraphicsDevice::new(
            super::GraphicsDeviceId::new(130),
            super::GraphicsInstanceId::new(131),
            "gpu",
        );
        let surface = SurfaceTarget::new(
            SurfaceTargetId::new(132),
            "surface",
            SurfaceSize::new(32, 32),
        );
        let target = GpuTexture::from_descriptor(
            GpuTextureId::new(133),
            GpuTextureDescriptor::new(
                "target",
                TextureSize::new(32, 32, 1),
                TextureFormat::Rgba8Unorm,
                [TextureUsage::RenderTarget],
            ),
        )
        .expect("valid render target");
        let uniform = GpuBuffer::from_descriptor(
            GpuBufferId::new(134),
            GpuBufferDescriptor::new("uniform", 64, [BufferUsage::Uniform]),
        )
        .expect("valid uniform buffer");
        let pipeline = RenderPipeline::from_descriptor(
            RenderPipelineId::new(135),
            RenderPipelineDescriptor::new(
                "pipeline",
                PipelineShader::new(ShaderModuleId::new(136), ShaderStage::Vertex),
                None,
                [],
                [TextureFormat::Rgba8Unorm],
                None,
            )
            .with_bind_group_layouts([BindGroupLayoutId::new(137)]),
        )
        .expect("valid render pipeline");
        let compatible_group = BindGroup::from_descriptor(
            BindGroupId::new(138),
            BindGroupDescriptor::new(
                "compatible",
                BindGroupLayoutId::new(137),
                [BindGroupEntry::new(
                    0,
                    BoundResource::UniformBuffer(uniform.id()),
                )],
            ),
        )
        .expect("valid bind group");
        let mismatched_group = BindGroup::from_descriptor(
            BindGroupId::new(139),
            BindGroupDescriptor::new(
                "mismatch",
                BindGroupLayoutId::new(140),
                [BindGroupEntry::new(
                    0,
                    BoundResource::UniformBuffer(uniform.id()),
                )],
            ),
        )
        .expect("valid bind group");

        let mut catalog = GraphicsResourceCatalog::new();
        catalog
            .register_device(&device)
            .register_surface(&surface)
            .register_texture(&target)
            .register_buffer(&uniform)
            .register_render_pipeline(&pipeline)
            .register_bind_group(&compatible_group)
            .register_bind_group(&mismatched_group);

        let compatible_submission = FrameSubmission::new(
            FrameContext::new(FrameId::new(141), device.id(), surface.id()),
            [RenderCommandList::new(
                CommandListId::new(142),
                "compatible",
                [
                    RenderCommand::BeginRenderPass(RenderPassDescriptor::new(
                        "main",
                        [target.id()],
                        None,
                    )),
                    RenderCommand::SetPipeline(pipeline.id()),
                    RenderCommand::SetBindGroup {
                        slot: 0,
                        group: compatible_group.id(),
                    },
                    RenderCommand::Draw(DrawCall::new(3, 1)),
                    RenderCommand::EndRenderPass,
                ],
            )
            .expect("valid command list")],
        )
        .expect("valid submission");

        assert!(catalog.validate_submission(&compatible_submission).is_ok());

        let mismatch_submission = FrameSubmission::new(
            FrameContext::new(FrameId::new(143), device.id(), surface.id()),
            [RenderCommandList::new(
                CommandListId::new(144),
                "mismatch",
                [
                    RenderCommand::BeginRenderPass(RenderPassDescriptor::new(
                        "main",
                        [target.id()],
                        None,
                    )),
                    RenderCommand::SetPipeline(pipeline.id()),
                    RenderCommand::SetBindGroup {
                        slot: 0,
                        group: mismatched_group.id(),
                    },
                    RenderCommand::Draw(DrawCall::new(3, 1)),
                    RenderCommand::EndRenderPass,
                ],
            )
            .expect("valid command list")],
        )
        .expect("valid submission");

        assert_eq!(
            catalog
                .validate_submission(&mismatch_submission)
                .unwrap_err(),
            GraphicsResourceError::SubmittedBindGroupLayoutMismatch {
                frame: FrameId::new(143),
                command_list: CommandListId::new(144),
                command: "set_bind_group",
                pipeline: SubmittedResource::RenderPipeline(pipeline.id()),
                slot: 0,
                bind_group: mismatched_group.id(),
                expected: BindGroupLayoutId::new(137),
                actual: BindGroupLayoutId::new(140),
            }
        );

        let out_of_range_submission = FrameSubmission::new(
            FrameContext::new(FrameId::new(145), device.id(), surface.id()),
            [RenderCommandList::new(
                CommandListId::new(146),
                "out_of_range",
                [
                    RenderCommand::BeginRenderPass(RenderPassDescriptor::new(
                        "main",
                        [target.id()],
                        None,
                    )),
                    RenderCommand::SetPipeline(pipeline.id()),
                    RenderCommand::SetBindGroup {
                        slot: 1,
                        group: compatible_group.id(),
                    },
                    RenderCommand::Draw(DrawCall::new(3, 1)),
                    RenderCommand::EndRenderPass,
                ],
            )
            .expect("valid command list")],
        )
        .expect("valid submission");

        assert_eq!(
            catalog
                .validate_submission(&out_of_range_submission)
                .unwrap_err(),
            GraphicsResourceError::SubmittedBindGroupSlotOutOfRange {
                frame: FrameId::new(145),
                command_list: CommandListId::new(146),
                command: "set_bind_group",
                pipeline: SubmittedResource::RenderPipeline(pipeline.id()),
                slot: 1,
                bind_group: compatible_group.id(),
            }
        );
    }

    #[test]
    fn resource_catalog_validates_pipeline_vertex_buffer_slots() {
        let device = super::GraphicsDevice::new(
            super::GraphicsDeviceId::new(150),
            super::GraphicsInstanceId::new(151),
            "gpu",
        );
        let surface = SurfaceTarget::new(
            SurfaceTargetId::new(152),
            "surface",
            SurfaceSize::new(32, 32),
        );
        let target = GpuTexture::from_descriptor(
            GpuTextureId::new(153),
            GpuTextureDescriptor::new(
                "target",
                TextureSize::new(32, 32, 1),
                TextureFormat::Rgba8Unorm,
                [TextureUsage::RenderTarget],
            ),
        )
        .expect("valid render target");
        let vertices = GpuBuffer::from_descriptor(
            GpuBufferId::new(154),
            GpuBufferDescriptor::new("vertices", 96, [BufferUsage::Vertex]),
        )
        .expect("valid vertex buffer");
        let pipeline = RenderPipeline::from_descriptor(
            RenderPipelineId::new(155),
            RenderPipelineDescriptor::new(
                "pipeline",
                PipelineShader::new(ShaderModuleId::new(156), ShaderStage::Vertex),
                None,
                [VertexBufferLayout::new(0, 32, VertexStepMode::Vertex)],
                [TextureFormat::Rgba8Unorm],
                None,
            ),
        )
        .expect("valid render pipeline");

        let mut catalog = GraphicsResourceCatalog::new();
        catalog
            .register_device(&device)
            .register_surface(&surface)
            .register_texture(&target)
            .register_buffer(&vertices)
            .register_render_pipeline(&pipeline);

        let missing_vertex_buffer = FrameSubmission::new(
            FrameContext::new(FrameId::new(157), device.id(), surface.id()),
            [RenderCommandList::new(
                CommandListId::new(158),
                "missing_vertices",
                [
                    RenderCommand::BeginRenderPass(RenderPassDescriptor::new(
                        "main",
                        [target.id()],
                        None,
                    )),
                    RenderCommand::SetPipeline(pipeline.id()),
                    RenderCommand::Draw(DrawCall::new(3, 1)),
                    RenderCommand::EndRenderPass,
                ],
            )
            .expect("valid command list")],
        )
        .expect("valid submission");

        assert_eq!(
            catalog
                .validate_submission(&missing_vertex_buffer)
                .unwrap_err(),
            GraphicsResourceError::SubmittedVertexBufferSlotMissing {
                frame: FrameId::new(157),
                command_list: CommandListId::new(158),
                command: "draw",
                pipeline: pipeline.id(),
                slot: 0,
            }
        );

        let with_vertex_buffer = FrameSubmission::new(
            FrameContext::new(FrameId::new(159), device.id(), surface.id()),
            [RenderCommandList::new(
                CommandListId::new(160),
                "with_vertices",
                [
                    RenderCommand::BeginRenderPass(RenderPassDescriptor::new(
                        "main",
                        [target.id()],
                        None,
                    )),
                    RenderCommand::SetPipeline(pipeline.id()),
                    RenderCommand::SetVertexBuffer {
                        slot: 0,
                        buffer: vertices.id(),
                    },
                    RenderCommand::Draw(DrawCall::new(3, 1)),
                    RenderCommand::EndRenderPass,
                ],
            )
            .expect("valid command list")],
        )
        .expect("valid submission");

        assert!(catalog.validate_submission(&with_vertex_buffer).is_ok());
    }

    #[test]
    fn buffer_descriptors_validate_size_and_usage() {
        let descriptor = GpuBufferDescriptor::new(
            "indices",
            128,
            [BufferUsage::Index, BufferUsage::TransferDst],
        );
        let buffer = GpuBuffer::from_descriptor(GpuBufferId::new(7), descriptor).unwrap();

        assert_eq!(buffer.label(), "indices");
        assert!(buffer.descriptor().usages().contains(&BufferUsage::Index));
        assert_eq!(
            GpuBuffer::from_descriptor(
                GpuBufferId::new(8),
                GpuBufferDescriptor::new("empty", 0, [BufferUsage::Vertex]),
            )
            .unwrap_err(),
            GraphicsResourceError::EmptyBuffer {
                label: "empty".to_string(),
            }
        );
        assert_eq!(
            GpuBuffer::from_descriptor(
                GpuBufferId::new(9),
                GpuBufferDescriptor::new("missing", 16, []),
            )
            .unwrap_err(),
            GraphicsResourceError::MissingBufferUsage {
                label: "missing".to_string(),
            }
        );
    }

    #[test]
    fn texture_descriptors_validate_extent_format_and_usage() {
        let descriptor = GpuTextureDescriptor::new(
            "depth",
            TextureSize::new(64, 64, 1),
            TextureFormat::Depth32Float,
            [TextureUsage::DepthStencil, TextureUsage::TransferSrc],
        );
        let texture = GpuTexture::from_descriptor(GpuTextureId::new(10), descriptor).unwrap();

        assert_eq!(texture.label(), "depth");
        assert_eq!(texture.descriptor().format(), TextureFormat::Depth32Float);
        assert_eq!(
            GpuTexture::from_descriptor(
                GpuTextureId::new(11),
                GpuTextureDescriptor::new(
                    "empty",
                    TextureSize::new(0, 64, 1),
                    TextureFormat::Rgba8Unorm,
                    [TextureUsage::Sampled],
                ),
            )
            .unwrap_err(),
            GraphicsResourceError::EmptyTexture {
                label: "empty".to_string(),
            }
        );
        assert_eq!(
            GpuTexture::from_descriptor(
                GpuTextureId::new(12),
                GpuTextureDescriptor::new(
                    "missing",
                    TextureSize::new(64, 64, 1),
                    TextureFormat::Rgba8Unorm,
                    [],
                ),
            )
            .unwrap_err(),
            GraphicsResourceError::MissingTextureUsage {
                label: "missing".to_string(),
            }
        );
    }

    #[test]
    fn shader_descriptors_validate_entry_point_and_source() {
        let descriptor = ShaderModuleDescriptor::new(
            "sprite_vertex",
            ShaderStage::Vertex,
            ShaderSourceKind::Wgsl,
            "vs_main",
            "@vertex fn vs_main() {}",
        );
        let shader = ShaderModule::from_descriptor(ShaderModuleId::new(13), descriptor).unwrap();

        assert_eq!(shader.label(), "sprite_vertex");
        assert_eq!(shader.descriptor().entry_point(), "vs_main");
        assert_eq!(shader.descriptor().source_kind(), ShaderSourceKind::Wgsl);
        assert_eq!(
            ShaderModule::from_descriptor(
                ShaderModuleId::new(14),
                ShaderModuleDescriptor::new(
                    "missing_entry",
                    ShaderStage::Fragment,
                    ShaderSourceKind::Wgsl,
                    " ",
                    "@fragment fn fs_main() {}",
                ),
            )
            .unwrap_err(),
            GraphicsResourceError::MissingShaderEntryPoint {
                label: "missing_entry".to_string(),
            }
        );
        assert_eq!(
            ShaderModule::from_descriptor(
                ShaderModuleId::new(15),
                ShaderModuleDescriptor::new(
                    "empty_source",
                    ShaderStage::Compute,
                    ShaderSourceKind::Wgsl,
                    "cs_main",
                    "",
                ),
            )
            .unwrap_err(),
            GraphicsResourceError::EmptyShaderSource {
                label: "empty_source".to_string(),
            }
        );
    }

    #[test]
    fn render_pipeline_descriptors_validate_shader_stages_targets_and_vertex_buffers() {
        let descriptor = RenderPipelineDescriptor::new(
            "sprite_pipeline",
            PipelineShader::new(ShaderModuleId::new(16), ShaderStage::Vertex),
            Some(PipelineShader::new(
                ShaderModuleId::new(17),
                ShaderStage::Fragment,
            )),
            [VertexBufferLayout::new(0, 32, VertexStepMode::Vertex)],
            [TextureFormat::Bgra8Unorm],
            None,
        )
        .with_bind_group_layouts([BindGroupLayoutId::new(1), BindGroupLayoutId::new(2)]);
        let pipeline = RenderPipeline::from_descriptor(RenderPipelineId::new(18), descriptor)
            .expect("valid render pipeline descriptor");

        assert_eq!(pipeline.label(), "sprite_pipeline");
        assert_eq!(
            pipeline.descriptor().vertex_shader().module(),
            ShaderModuleId::new(16)
        );
        assert_eq!(
            pipeline.descriptor().color_targets(),
            &[TextureFormat::Bgra8Unorm]
        );
        assert_eq!(
            pipeline.descriptor().bind_group_layouts(),
            &[BindGroupLayoutId::new(1), BindGroupLayoutId::new(2)]
        );
        assert_eq!(
            RenderPipeline::from_descriptor(
                RenderPipelineId::new(19),
                RenderPipelineDescriptor::new(
                    "bad_vertex",
                    PipelineShader::new(ShaderModuleId::new(20), ShaderStage::Fragment),
                    None,
                    [],
                    [TextureFormat::Rgba8Unorm],
                    None,
                ),
            )
            .unwrap_err(),
            GraphicsResourceError::InvalidPipelineShaderStage {
                label: "bad_vertex".to_string(),
                expected: ShaderStage::Vertex,
                actual: ShaderStage::Fragment,
            }
        );
        assert_eq!(
            RenderPipeline::from_descriptor(
                RenderPipelineId::new(21),
                RenderPipelineDescriptor::new(
                    "no_target",
                    PipelineShader::new(ShaderModuleId::new(22), ShaderStage::Vertex),
                    None,
                    [],
                    [],
                    None,
                ),
            )
            .unwrap_err(),
            GraphicsResourceError::MissingPipelineTarget {
                label: "no_target".to_string(),
            }
        );
        assert_eq!(
            RenderPipeline::from_descriptor(
                RenderPipelineId::new(23),
                RenderPipelineDescriptor::new(
                    "empty_layout",
                    PipelineShader::new(ShaderModuleId::new(24), ShaderStage::Vertex),
                    None,
                    [VertexBufferLayout::new(3, 0, VertexStepMode::Instance)],
                    [TextureFormat::Rgba8Unorm],
                    None,
                ),
            )
            .unwrap_err(),
            GraphicsResourceError::EmptyVertexBufferLayout {
                label: "empty_layout".to_string(),
                slot: 3,
            }
        );
        assert_eq!(
            RenderPipeline::from_descriptor(
                RenderPipelineId::new(25),
                RenderPipelineDescriptor::new(
                    "depth_as_color",
                    PipelineShader::new(ShaderModuleId::new(26), ShaderStage::Vertex),
                    None,
                    [],
                    [TextureFormat::Depth32Float],
                    None,
                ),
            )
            .unwrap_err(),
            GraphicsResourceError::InvalidColorTargetFormat {
                label: "depth_as_color".to_string(),
                format: TextureFormat::Depth32Float,
            }
        );
        assert_eq!(
            RenderPipeline::from_descriptor(
                RenderPipelineId::new(27),
                RenderPipelineDescriptor::new(
                    "color_as_depth",
                    PipelineShader::new(ShaderModuleId::new(28), ShaderStage::Vertex),
                    None,
                    [],
                    [],
                    Some(TextureFormat::Rgba8Unorm),
                ),
            )
            .unwrap_err(),
            GraphicsResourceError::InvalidDepthTargetFormat {
                label: "color_as_depth".to_string(),
                format: TextureFormat::Rgba8Unorm,
            }
        );
        assert_eq!(
            RenderPipeline::from_descriptor(
                RenderPipelineId::new(33),
                RenderPipelineDescriptor::new(
                    "duplicate_bind_group_layout",
                    PipelineShader::new(ShaderModuleId::new(34), ShaderStage::Vertex),
                    None,
                    [],
                    [TextureFormat::Rgba8Unorm],
                    None,
                )
                .with_bind_group_layouts([BindGroupLayoutId::new(5), BindGroupLayoutId::new(5)]),
            )
            .unwrap_err(),
            GraphicsResourceError::DuplicatePipelineBindGroupLayout {
                label: "duplicate_bind_group_layout".to_string(),
                layout: BindGroupLayoutId::new(5),
            }
        );
    }

    #[test]
    fn compute_pipeline_descriptors_validate_shader_stage_and_layouts() {
        let descriptor = ComputePipelineDescriptor::new(
            "cull_pipeline",
            PipelineShader::new(ShaderModuleId::new(35), ShaderStage::Compute),
        )
        .with_bind_group_layouts([BindGroupLayoutId::new(36), BindGroupLayoutId::new(37)]);
        let pipeline = ComputePipeline::from_descriptor(ComputePipelineId::new(38), descriptor)
            .expect("valid compute pipeline descriptor");

        assert_eq!(pipeline.label(), "cull_pipeline");
        assert_eq!(pipeline.id(), ComputePipelineId::new(38));
        assert_eq!(
            pipeline.descriptor().compute_shader().module(),
            ShaderModuleId::new(35)
        );
        assert_eq!(
            pipeline.descriptor().bind_group_layouts(),
            &[BindGroupLayoutId::new(36), BindGroupLayoutId::new(37)]
        );
        assert_eq!(
            ComputePipeline::from_descriptor(
                ComputePipelineId::new(39),
                ComputePipelineDescriptor::new(
                    "bad_compute_stage",
                    PipelineShader::new(ShaderModuleId::new(40), ShaderStage::Fragment),
                ),
            )
            .unwrap_err(),
            GraphicsResourceError::InvalidPipelineShaderStage {
                label: "bad_compute_stage".to_string(),
                expected: ShaderStage::Compute,
                actual: ShaderStage::Fragment,
            }
        );
        assert_eq!(
            ComputePipeline::from_descriptor(
                ComputePipelineId::new(41),
                ComputePipelineDescriptor::new(
                    "duplicate_compute_layout",
                    PipelineShader::new(ShaderModuleId::new(42), ShaderStage::Compute),
                )
                .with_bind_group_layouts([BindGroupLayoutId::new(43), BindGroupLayoutId::new(43)]),
            )
            .unwrap_err(),
            GraphicsResourceError::DuplicatePipelineBindGroupLayout {
                label: "duplicate_compute_layout".to_string(),
                layout: BindGroupLayoutId::new(43),
            }
        );
    }

    #[test]
    fn bind_group_layout_descriptors_validate_entries_bindings_and_visibility() {
        let descriptor = BindGroupLayoutDescriptor::new(
            "sprite_bindings",
            [
                BindGroupLayoutEntry::new(
                    0,
                    BindingResourceKind::UniformBuffer,
                    [ShaderStage::Vertex],
                ),
                BindGroupLayoutEntry::new(
                    1,
                    BindingResourceKind::SampledTexture,
                    [ShaderStage::Fragment],
                ),
            ],
        );
        let layout = BindGroupLayout::from_descriptor(BindGroupLayoutId::new(29), descriptor)
            .expect("valid bind group layout descriptor");

        assert_eq!(layout.label(), "sprite_bindings");
        assert_eq!(layout.descriptor().entries().len(), 2);
        assert_eq!(
            layout.descriptor().entries()[1].resource(),
            BindingResourceKind::SampledTexture
        );
        assert_eq!(
            BindGroupLayout::from_descriptor(
                BindGroupLayoutId::new(30),
                BindGroupLayoutDescriptor::new("empty", []),
            )
            .unwrap_err(),
            GraphicsResourceError::EmptyBindGroupLayout {
                label: "empty".to_string(),
            }
        );
        assert_eq!(
            BindGroupLayout::from_descriptor(
                BindGroupLayoutId::new(31),
                BindGroupLayoutDescriptor::new(
                    "duplicate",
                    [
                        BindGroupLayoutEntry::new(
                            2,
                            BindingResourceKind::UniformBuffer,
                            [ShaderStage::Vertex],
                        ),
                        BindGroupLayoutEntry::new(
                            2,
                            BindingResourceKind::Sampler,
                            [ShaderStage::Fragment],
                        ),
                    ],
                ),
            )
            .unwrap_err(),
            GraphicsResourceError::DuplicateBindGroupBinding {
                label: "duplicate".to_string(),
                binding: 2,
            }
        );
        assert_eq!(
            BindGroupLayout::from_descriptor(
                BindGroupLayoutId::new(32),
                BindGroupLayoutDescriptor::new(
                    "missing_visibility",
                    [BindGroupLayoutEntry::new(
                        3,
                        BindingResourceKind::StorageBuffer,
                        [],
                    )],
                ),
            )
            .unwrap_err(),
            GraphicsResourceError::MissingBindGroupVisibility {
                label: "missing_visibility".to_string(),
                binding: 3,
            }
        );
    }

    #[test]
    fn bind_group_descriptors_validate_resources_against_layouts() {
        let layout = BindGroupLayoutDescriptor::new(
            "sprite_layout",
            [
                BindGroupLayoutEntry::new(
                    0,
                    BindingResourceKind::UniformBuffer,
                    [ShaderStage::Vertex],
                ),
                BindGroupLayoutEntry::new(
                    1,
                    BindingResourceKind::SampledTexture,
                    [ShaderStage::Fragment],
                ),
                BindGroupLayoutEntry::new(2, BindingResourceKind::Sampler, [ShaderStage::Fragment]),
            ],
        );
        let descriptor = BindGroupDescriptor::new(
            "sprite_group",
            BindGroupLayoutId::new(40),
            [
                BindGroupEntry::new(0, BoundResource::UniformBuffer(GpuBufferId::new(41))),
                BindGroupEntry::new(1, BoundResource::SampledTexture(GpuTextureId::new(42))),
                BindGroupEntry::new(2, BoundResource::Sampler(super::GpuSamplerId::new(43))),
            ],
        );
        let group = BindGroup::from_descriptor(BindGroupId::new(44), descriptor.clone())
            .expect("valid bind group descriptor");

        assert_eq!(group.label(), "sprite_group");
        assert_eq!(group.descriptor().layout(), BindGroupLayoutId::new(40));
        assert!(descriptor.validate_against_layout(&layout).is_ok());
        assert_eq!(
            BindGroup::from_descriptor(
                BindGroupId::new(45),
                BindGroupDescriptor::new("empty", BindGroupLayoutId::new(46), []),
            )
            .unwrap_err(),
            GraphicsResourceError::EmptyBindGroup {
                label: "empty".to_string(),
            }
        );
        assert_eq!(
            BindGroup::from_descriptor(
                BindGroupId::new(47),
                BindGroupDescriptor::new(
                    "duplicate_resource",
                    BindGroupLayoutId::new(48),
                    [
                        BindGroupEntry::new(0, BoundResource::UniformBuffer(GpuBufferId::new(49))),
                        BindGroupEntry::new(0, BoundResource::StorageBuffer(GpuBufferId::new(50))),
                    ],
                ),
            )
            .unwrap_err(),
            GraphicsResourceError::DuplicateBindGroupResourceBinding {
                label: "duplicate_resource".to_string(),
                binding: 0,
            }
        );
        assert_eq!(
            BindGroupDescriptor::new(
                "missing_resource",
                BindGroupLayoutId::new(51),
                [
                    BindGroupEntry::new(0, BoundResource::UniformBuffer(GpuBufferId::new(52))),
                    BindGroupEntry::new(1, BoundResource::SampledTexture(GpuTextureId::new(53))),
                ],
            )
            .validate_against_layout(&layout)
            .unwrap_err(),
            GraphicsResourceError::MissingBindGroupResource {
                label: "missing_resource".to_string(),
                binding: 2,
            }
        );
        assert_eq!(
            BindGroupDescriptor::new(
                "unknown_resource",
                BindGroupLayoutId::new(54),
                [
                    BindGroupEntry::new(0, BoundResource::UniformBuffer(GpuBufferId::new(55))),
                    BindGroupEntry::new(1, BoundResource::SampledTexture(GpuTextureId::new(56))),
                    BindGroupEntry::new(2, BoundResource::Sampler(super::GpuSamplerId::new(57))),
                    BindGroupEntry::new(9, BoundResource::StorageBuffer(GpuBufferId::new(58))),
                ],
            )
            .validate_against_layout(&layout)
            .unwrap_err(),
            GraphicsResourceError::UnknownBindGroupResource {
                label: "unknown_resource".to_string(),
                binding: 9,
            }
        );
        assert_eq!(
            BindGroupDescriptor::new(
                "mismatch",
                BindGroupLayoutId::new(59),
                [
                    BindGroupEntry::new(0, BoundResource::UniformBuffer(GpuBufferId::new(60))),
                    BindGroupEntry::new(1, BoundResource::StorageTexture(GpuTextureId::new(61))),
                    BindGroupEntry::new(2, BoundResource::Sampler(super::GpuSamplerId::new(62))),
                ],
            )
            .validate_against_layout(&layout)
            .unwrap_err(),
            GraphicsResourceError::MismatchedBindGroupResource {
                label: "mismatch".to_string(),
                binding: 1,
                expected: BindingResourceKind::SampledTexture,
                actual: BindingResourceKind::StorageTexture,
            }
        );
    }

    #[test]
    fn render_command_lists_validate_pass_pipeline_and_draw_order() {
        let list = RenderCommandList::new(
            CommandListId::new(63),
            "sprite_commands",
            [
                RenderCommand::BeginRenderPass(RenderPassDescriptor::new(
                    "main",
                    [GpuTextureId::new(64)],
                    Some(GpuTextureId::new(65)),
                )),
                RenderCommand::SetPipeline(RenderPipelineId::new(66)),
                RenderCommand::SetBindGroup {
                    slot: 0,
                    group: BindGroupId::new(67),
                },
                RenderCommand::SetVertexBuffer {
                    slot: 0,
                    buffer: GpuBufferId::new(68),
                },
                RenderCommand::Draw(DrawCall::new(6, 1)),
                RenderCommand::SetIndexBuffer(GpuBufferId::new(69)),
                RenderCommand::DrawIndexed(IndexedDrawCall::new(12, 2)),
                RenderCommand::EndRenderPass,
            ],
        )
        .expect("valid render command list");

        assert_eq!(list.label(), "sprite_commands");
        assert_eq!(list.id(), CommandListId::new(63));
        assert_eq!(list.commands().len(), 8);
        assert_eq!(list.commands()[0].name(), "begin_render_pass");
        assert_eq!(
            RenderCommandList::new(CommandListId::new(70), "empty", []).unwrap_err(),
            GraphicsResourceError::EmptyCommandList {
                label: "empty".to_string(),
            }
        );
        assert_eq!(
            RenderCommandList::new(
                CommandListId::new(71),
                "draw_without_pipeline",
                [
                    RenderCommand::BeginRenderPass(RenderPassDescriptor::new(
                        "main",
                        [GpuTextureId::new(72)],
                        None,
                    )),
                    RenderCommand::Draw(DrawCall::new(3, 1)),
                    RenderCommand::EndRenderPass,
                ],
            )
            .unwrap_err(),
            GraphicsResourceError::DrawWithoutPipeline {
                label: "draw_without_pipeline".to_string(),
                command: "draw",
                active_pass: "main".to_string(),
            }
        );
        assert_eq!(
            RenderCommandList::new(
                CommandListId::new(73),
                "draw_outside_pass",
                [RenderCommand::Draw(DrawCall::new(3, 1))],
            )
            .unwrap_err(),
            GraphicsResourceError::RenderPassNotActive {
                label: "draw_outside_pass".to_string(),
                command: "draw",
            }
        );
        assert_eq!(
            RenderCommandList::new(
                CommandListId::new(74),
                "empty_draw",
                [
                    RenderCommand::BeginRenderPass(RenderPassDescriptor::new(
                        "main",
                        [GpuTextureId::new(75)],
                        None,
                    )),
                    RenderCommand::SetPipeline(RenderPipelineId::new(76)),
                    RenderCommand::Draw(DrawCall::new(0, 1)),
                    RenderCommand::EndRenderPass,
                ],
            )
            .unwrap_err(),
            GraphicsResourceError::EmptyDraw {
                label: "empty_draw".to_string(),
                command: "draw",
            }
        );
        assert_eq!(
            RenderCommandList::new(
                CommandListId::new(147),
                "indexed_without_buffer",
                [
                    RenderCommand::BeginRenderPass(RenderPassDescriptor::new(
                        "main",
                        [GpuTextureId::new(148)],
                        None,
                    )),
                    RenderCommand::SetPipeline(RenderPipelineId::new(149)),
                    RenderCommand::DrawIndexed(IndexedDrawCall::new(3, 1)),
                    RenderCommand::EndRenderPass,
                ],
            )
            .unwrap_err(),
            GraphicsResourceError::DrawIndexedWithoutIndexBuffer {
                label: "indexed_without_buffer".to_string(),
                command: "draw_indexed",
                active_pass: "main".to_string(),
            }
        );
    }

    #[test]
    fn render_command_lists_validate_compute_pass_and_dispatch_order() {
        let list = RenderCommandList::new(
            CommandListId::new(84),
            "compute_commands",
            [
                RenderCommand::BeginComputePass(ComputePassDescriptor::new("cull")),
                RenderCommand::SetComputePipeline(ComputePipelineId::new(85)),
                RenderCommand::SetComputeBindGroup {
                    slot: 0,
                    group: BindGroupId::new(86),
                },
                RenderCommand::Dispatch(DispatchCall::new(8, 4, 1)),
                RenderCommand::EndComputePass,
            ],
        )
        .expect("valid compute command list");

        assert_eq!(list.label(), "compute_commands");
        assert_eq!(list.commands()[0].name(), "begin_compute_pass");
        assert_eq!(
            RenderCommandList::new(
                CommandListId::new(87),
                "dispatch_without_pipeline",
                [
                    RenderCommand::BeginComputePass(ComputePassDescriptor::new("cull")),
                    RenderCommand::Dispatch(DispatchCall::new(1, 1, 1)),
                    RenderCommand::EndComputePass,
                ],
            )
            .unwrap_err(),
            GraphicsResourceError::DispatchWithoutPipeline {
                label: "dispatch_without_pipeline".to_string(),
                command: "dispatch",
                active_pass: "cull".to_string(),
            }
        );
        assert_eq!(
            RenderCommandList::new(
                CommandListId::new(88),
                "dispatch_outside_pass",
                [RenderCommand::Dispatch(DispatchCall::new(1, 1, 1))],
            )
            .unwrap_err(),
            GraphicsResourceError::ComputePassNotActive {
                label: "dispatch_outside_pass".to_string(),
                command: "dispatch",
            }
        );
        assert_eq!(
            RenderCommandList::new(
                CommandListId::new(89),
                "empty_dispatch",
                [
                    RenderCommand::BeginComputePass(ComputePassDescriptor::new("cull")),
                    RenderCommand::SetComputePipeline(ComputePipelineId::new(90)),
                    RenderCommand::Dispatch(DispatchCall::new(0, 1, 1)),
                    RenderCommand::EndComputePass,
                ],
            )
            .unwrap_err(),
            GraphicsResourceError::EmptyDispatch {
                label: "empty_dispatch".to_string(),
                command: "dispatch",
            }
        );
        assert_eq!(
            RenderCommandList::new(
                CommandListId::new(91),
                "compute_left_open",
                [RenderCommand::BeginComputePass(ComputePassDescriptor::new(
                    "cull",
                ))],
            )
            .unwrap_err(),
            GraphicsResourceError::ComputePassLeftOpen {
                label: "compute_left_open".to_string(),
                active_pass: "cull".to_string(),
            }
        );
    }

    #[test]
    fn render_command_list_builder_encodes_closed_render_and_compute_passes() {
        let mut builder = RenderCommandListBuilder::new(CommandListId::new(170), "built_commands");
        assert_eq!(builder.id(), CommandListId::new(170));
        assert_eq!(builder.label(), "built_commands");
        assert!(builder.commands().is_empty());

        builder
            .render_pass(
                RenderPassDescriptor::new("main", [GpuTextureId::new(171)], None),
                |pass| {
                    assert!(pass.commands().is_empty());
                    pass.set_pipeline(RenderPipelineId::new(172))
                        .set_bind_group(0, BindGroupId::new(173))
                        .set_vertex_buffer(0, GpuBufferId::new(174))
                        .draw(DrawCall::new(6, 1))?
                        .set_index_buffer(GpuBufferId::new(175))
                        .draw_indexed(IndexedDrawCall::new(12, 2))?;
                    Ok(())
                },
            )
            .expect("render pass should encode");
        builder
            .compute_pass(ComputePassDescriptor::new("cull"), |pass| {
                assert!(pass.commands().is_empty());
                pass.set_pipeline(ComputePipelineId::new(176))
                    .set_bind_group(0, BindGroupId::new(177))
                    .dispatch(DispatchCall::new(2, 3, 1))?;
                Ok(())
            })
            .expect("compute pass should encode");

        let list = builder.finish().expect("builder should produce valid list");

        assert_eq!(list.commands().len(), 13);
        assert!(matches!(
            list.commands()[0],
            RenderCommand::BeginRenderPass(_)
        ));
        assert_eq!(list.commands()[7], RenderCommand::EndRenderPass);
        assert!(matches!(
            list.commands()[8],
            RenderCommand::BeginComputePass(_)
        ));
        assert_eq!(list.commands()[12], RenderCommand::EndComputePass);
    }

    #[test]
    fn render_command_list_builder_rejects_common_invalid_render_states() {
        assert_eq!(
            RenderCommandList::builder(CommandListId::new(178), "empty_builder")
                .finish()
                .unwrap_err(),
            GraphicsResourceError::EmptyCommandList {
                label: "empty_builder".to_string(),
            }
        );

        let mut missing_target =
            RenderCommandListBuilder::new(CommandListId::new(179), "missing_target");
        assert_eq!(
            missing_target
                .render_pass(RenderPassDescriptor::new("main", [], None), |_| Ok(()))
                .unwrap_err(),
            GraphicsResourceError::MissingRenderPassTarget {
                label: "main".to_string(),
            }
        );
        assert!(missing_target.commands().is_empty());

        let mut draw_without_pipeline =
            RenderCommandListBuilder::new(CommandListId::new(180), "draw_without_pipeline");
        assert_eq!(
            draw_without_pipeline
                .render_pass(
                    RenderPassDescriptor::new("main", [GpuTextureId::new(181)], None),
                    |pass| {
                        pass.draw(DrawCall::new(3, 1))?;
                        Ok(())
                    },
                )
                .unwrap_err(),
            GraphicsResourceError::DrawWithoutPipeline {
                label: "draw_without_pipeline".to_string(),
                command: "draw",
                active_pass: "builder".to_string(),
            }
        );
        assert!(draw_without_pipeline.commands().is_empty());

        let mut indexed_without_buffer =
            RenderCommandListBuilder::new(CommandListId::new(182), "indexed_without_buffer");
        assert_eq!(
            indexed_without_buffer
                .render_pass(
                    RenderPassDescriptor::new("main", [GpuTextureId::new(183)], None),
                    |pass| {
                        pass.set_pipeline(RenderPipelineId::new(184))
                            .draw_indexed(IndexedDrawCall::new(3, 1))?;
                        Ok(())
                    },
                )
                .unwrap_err(),
            GraphicsResourceError::DrawIndexedWithoutIndexBuffer {
                label: "indexed_without_buffer".to_string(),
                command: "draw_indexed",
                active_pass: "builder".to_string(),
            }
        );
        assert!(indexed_without_buffer.commands().is_empty());
    }

    #[test]
    fn render_command_list_builder_rejects_common_invalid_compute_states() {
        let mut dispatch_without_pipeline =
            RenderCommandListBuilder::new(CommandListId::new(185), "dispatch_without_pipeline");
        assert_eq!(
            dispatch_without_pipeline
                .compute_pass(ComputePassDescriptor::new("cull"), |pass| {
                    pass.dispatch(DispatchCall::new(1, 1, 1))?;
                    Ok(())
                })
                .unwrap_err(),
            GraphicsResourceError::DispatchWithoutPipeline {
                label: "dispatch_without_pipeline".to_string(),
                command: "dispatch",
                active_pass: "builder".to_string(),
            }
        );
        assert!(dispatch_without_pipeline.commands().is_empty());

        let mut empty_dispatch =
            RenderCommandListBuilder::new(CommandListId::new(186), "empty_dispatch");
        assert_eq!(
            empty_dispatch
                .compute_pass(ComputePassDescriptor::new("cull"), |pass| {
                    pass.set_pipeline(ComputePipelineId::new(187))
                        .dispatch(DispatchCall::new(1, 0, 1))?;
                    Ok(())
                })
                .unwrap_err(),
            GraphicsResourceError::EmptyDispatch {
                label: "empty_dispatch".to_string(),
                command: "dispatch",
            }
        );
        assert!(empty_dispatch.commands().is_empty());
    }

    #[test]
    fn render_command_list_debug_dump_is_stable() {
        let mut builder = RenderCommandList::builder(CommandListId::new(188), "dumpable");
        builder
            .render_pass(
                RenderPassDescriptor::new(
                    "main",
                    [GpuTextureId::new(189), GpuTextureId::new(190)],
                    Some(GpuTextureId::new(191)),
                ),
                |pass| {
                    pass.set_pipeline(RenderPipelineId::new(192))
                        .set_bind_group(0, BindGroupId::new(193))
                        .set_vertex_buffer(0, GpuBufferId::new(194))
                        .draw(DrawCall::new(6, 1))?
                        .set_index_buffer(GpuBufferId::new(195))
                        .draw_indexed(IndexedDrawCall::new(12, 2))?;
                    Ok(())
                },
            )
            .expect("render pass should encode")
            .compute_pass(ComputePassDescriptor::new("cull"), |pass| {
                pass.set_pipeline(ComputePipelineId::new(196))
                    .set_bind_group(1, BindGroupId::new(197))
                    .dispatch(DispatchCall::new(2, 3, 4))?;
                Ok(())
            })
            .expect("compute pass should encode");
        let list = builder.finish().expect("command list should validate");

        assert_eq!(
            list.debug_dump(),
            concat!(
                "command_list id=188 label=\"dumpable\" commands=13\n",
                "  0: begin_render_pass label=\"main\" color_targets=[189, 190] depth_target=191\n",
                "  1: set_pipeline pipeline=192\n",
                "  2: set_bind_group slot=0 group=193\n",
                "  3: set_vertex_buffer slot=0 buffer=194\n",
                "  4: draw vertices=6 instances=1\n",
                "  5: set_index_buffer buffer=195\n",
                "  6: draw_indexed indices=12 instances=2\n",
                "  7: end_render_pass\n",
                "  8: begin_compute_pass label=\"cull\"\n",
                "  9: set_compute_pipeline pipeline=196\n",
                "  10: set_compute_bind_group slot=1 group=197\n",
                "  11: dispatch workgroups=2x3x4\n",
                "  12: end_compute_pass\n",
            )
        );
    }

    #[test]
    fn frame_submission_debug_dump_is_stable() {
        let first = simple_render_command_list(198);
        let mut builder = RenderCommandList::builder(CommandListId::new(199), "compute_only");
        builder
            .compute_pass(ComputePassDescriptor::new("cull"), |pass| {
                pass.set_pipeline(ComputePipelineId::new(200))
                    .dispatch(DispatchCall::new(1, 1, 1))?;
                Ok(())
            })
            .expect("compute pass should encode");
        let second = builder.finish().expect("command list should validate");
        let submission = FrameSubmission::new(
            FrameContext::new(
                FrameId::new(201),
                super::GraphicsDeviceId::new(202),
                SurfaceTargetId::new(203),
            ),
            [first, second],
        )
        .expect("submission should validate");

        assert_eq!(
            submission.debug_dump(),
            concat!(
                "frame_submission frame=201 device=202 target=203 command_lists=2\n",
                "  command_list id=198 label=\"simple\" commands=4\n",
                "  command_list id=199 label=\"compute_only\" commands=4\n",
            )
        );
    }

    #[test]
    fn render_pass_descriptors_validate_targets_and_scope() {
        assert_eq!(
            RenderPassDescriptor::new("missing_target", [], None)
                .validate()
                .unwrap_err(),
            GraphicsResourceError::MissingRenderPassTarget {
                label: "missing_target".to_string(),
            }
        );
        assert_eq!(
            RenderPassDescriptor::new(
                "duplicate_color",
                [GpuTextureId::new(77), GpuTextureId::new(77)],
                None,
            )
            .validate()
            .unwrap_err(),
            GraphicsResourceError::DuplicateRenderPassColorTarget {
                label: "duplicate_color".to_string(),
                target: GpuTextureId::new(77),
            }
        );
        assert_eq!(
            RenderPassDescriptor::new(
                "depth_alias",
                [GpuTextureId::new(78)],
                Some(GpuTextureId::new(78)),
            )
            .validate()
            .unwrap_err(),
            GraphicsResourceError::RenderPassDepthAlsoColorTarget {
                label: "depth_alias".to_string(),
                target: GpuTextureId::new(78),
            }
        );
        assert_eq!(
            RenderCommandList::new(
                CommandListId::new(79),
                "nested",
                [
                    RenderCommand::BeginRenderPass(RenderPassDescriptor::new(
                        "first",
                        [GpuTextureId::new(80)],
                        None,
                    )),
                    RenderCommand::BeginRenderPass(RenderPassDescriptor::new(
                        "second",
                        [GpuTextureId::new(81)],
                        None,
                    )),
                ],
            )
            .unwrap_err(),
            GraphicsResourceError::RenderPassAlreadyActive {
                label: "nested".to_string(),
                active_pass: "first".to_string(),
            }
        );
        assert_eq!(
            RenderCommandList::new(
                CommandListId::new(82),
                "left_open",
                [RenderCommand::BeginRenderPass(RenderPassDescriptor::new(
                    "main",
                    [GpuTextureId::new(83)],
                    None,
                ))],
            )
            .unwrap_err(),
            GraphicsResourceError::RenderPassLeftOpen {
                label: "left_open".to_string(),
                active_pass: "main".to_string(),
            }
        );
    }
}
