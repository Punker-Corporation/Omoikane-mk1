use std::fmt::Debug;

use hikari::{
    BufferUsage, Camera2dExtract, CommandListId, ExtractCameraId, ExtractTextureId, ExtractTileId,
    FrameContext, FrameId, FrameSubmission, GpuBuffer, GpuBufferDescriptor, GpuBufferId,
    GpuTexture, GpuTextureDescriptor, GpuTextureId, GraphicsDevice, GraphicsDeviceId,
    GraphicsInstance, GraphicsInstanceId, GraphicsResourceCatalog, PreparedFrame,
    RenderCommandList, RenderGraph, RenderPass, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, RenderPipelineId, ShaderModule, ShaderModuleDescriptor,
    ShaderModuleId, ShaderSourceKind, ShaderStage, SpriteExtract, SurfaceSize, SurfaceTarget,
    SurfaceTargetId, TextureFormat, TextureSize, TextureUsage, TileBatchExtract, TileExtract,
    VertexBufferLayout, VertexStepMode,
};
use keisan::{Box2, Color, Vector2, Vector2i};

fn main() -> Result<(), String> {
    let mut graph = RenderGraph::new();
    graph.import_resource("swapchain");
    graph.add_pass(RenderPass::new("clear").write("swapchain"));
    graph.add_pass(
        RenderPass::new("sprites")
            .read("swapchain")
            .write("swapchain")
            .preserve("swapchain"),
    );
    let graph_validation = graph.validate().map_err(|errors| {
        let joined = errors
            .into_iter()
            .map(|error| format!("{error:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("render graph validation failed: {joined}")
    })?;

    let mut extract = hikari::RenderExtract::new();
    let camera = ExtractCameraId::new(1);
    let sprite_texture = ExtractTextureId::new(10);
    let tile_texture = ExtractTextureId::new(11);
    extract
        .add_camera(
            Camera2dExtract::new(
                camera,
                "main",
                Box2::centered_around(Vector2::ZERO, Vector2::new(16.0, 9.0)),
                Vector2::new(1280.0, 720.0),
            )
            .with_clear_color(Color::BLACK),
        )
        .map_err(debug_error)?;
    extract.push_sprite(
        SpriteExtract::new(
            camera,
            sprite_texture,
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 1.0),
        )
        .with_depth(0.25),
    );
    let mut tiles =
        TileBatchExtract::new(camera, tile_texture, Vector2::new(-2.0, -2.0), Vector2::ONE)
            .with_depth(0.0);
    tiles
        .push_tile(TileExtract::new(Vector2i::new(0, 0), ExtractTileId::new(1)))
        .push_tile(TileExtract::new(Vector2i::new(1, 0), ExtractTileId::new(2)));
    extract.push_tile_batch(tiles);

    let extract_validation = extract.validate().map_err(debug_error)?;
    let prepared = PreparedFrame::from_extract(&extract).map_err(debug_error)?;
    let queued = prepared.queue();

    let instance = GraphicsInstance::new(GraphicsInstanceId::new(1), "headless");
    let device = GraphicsDevice::new(GraphicsDeviceId::new(2), instance.id(), "cpu-device");
    let surface = SurfaceTarget::new(
        SurfaceTargetId::new(3),
        "headless",
        SurfaceSize::new(1280, 720),
    );
    let target = GpuTexture::from_descriptor(
        GpuTextureId::new(4),
        GpuTextureDescriptor::new(
            "swapchain",
            TextureSize::new(1280, 720, 1),
            TextureFormat::Rgba8Unorm,
            [TextureUsage::RenderTarget],
        ),
    )
    .map_err(debug_error)?;
    let vertices = GpuBuffer::from_descriptor(
        GpuBufferId::new(5),
        GpuBufferDescriptor::new("quad_vertices", 256, [BufferUsage::Vertex]),
    )
    .map_err(debug_error)?;
    let vertex_shader = ShaderModule::from_descriptor(
        ShaderModuleId::new(6),
        ShaderModuleDescriptor::new(
            "sprite_vs",
            ShaderStage::Vertex,
            ShaderSourceKind::Wgsl,
            "main",
            "fn main() {}",
        ),
    )
    .map_err(debug_error)?;
    let fragment_shader = ShaderModule::from_descriptor(
        ShaderModuleId::new(7),
        ShaderModuleDescriptor::new(
            "sprite_fs",
            ShaderStage::Fragment,
            ShaderSourceKind::Wgsl,
            "main",
            "fn main() {}",
        ),
    )
    .map_err(debug_error)?;
    let pipeline = RenderPipeline::from_descriptor(
        RenderPipelineId::new(8),
        RenderPipelineDescriptor::new(
            "sprite_pipeline",
            hikari::PipelineShader::new(vertex_shader.id(), ShaderStage::Vertex),
            Some(hikari::PipelineShader::new(
                fragment_shader.id(),
                ShaderStage::Fragment,
            )),
            [VertexBufferLayout::new(0, 16, VertexStepMode::Vertex)],
            [TextureFormat::Rgba8Unorm],
            None,
        ),
    )
    .map_err(debug_error)?;

    let mut commands = RenderCommandList::builder(CommandListId::new(9), "frame_commands");
    commands
        .render_pass(
            RenderPassDescriptor::new("main", [target.id()], None),
            |pass| {
                pass.set_pipeline(pipeline.id())
                    .set_vertex_buffer(0, vertices.id())
                    .draw(hikari::DrawCall::new(6, queued.draws().len() as u32))?;
                Ok(())
            },
        )
        .map_err(debug_error)?;
    let commands = commands.finish().map_err(debug_error)?;
    let submission = FrameSubmission::new(
        FrameContext::new(FrameId::new(10), device.id(), surface.id()),
        [commands],
    )
    .map_err(debug_error)?;

    let mut catalog = GraphicsResourceCatalog::new();
    catalog
        .register_device(&device)
        .register_surface(&surface)
        .register_texture(&target)
        .register_buffer(&vertices)
        .register_render_pipeline(&pipeline);
    catalog
        .validate_submission(&submission)
        .map_err(debug_error)?;

    println!("{}", graph.debug_dump());
    println!("{}", graph_validation.debug_dump());
    println!(
        "extract cameras={} sprites={} tile_batches={} tiles={} debug_lines={}",
        extract_validation.camera_count(),
        extract_validation.sprite_count(),
        extract_validation.tile_batch_count(),
        extract_validation.tile_count(),
        extract_validation.debug_line_count(),
    );
    println!(
        "prepared primitives={} queued_draws={}",
        prepared.primitive_count(),
        queued.draws().len(),
    );
    for draw in queued.draws() {
        println!(
            "queued {:?} camera={} texture={:?} depth={} instances={}",
            draw.primitive(),
            draw.camera().raw(),
            draw.texture().map(|texture| texture.raw()),
            draw.depth(),
            draw.instances(),
        );
    }
    println!("{}", submission.command_lists()[0].debug_dump());
    println!("{}", submission.debug_dump());

    Ok(())
}

fn debug_error(error: impl Debug) -> String {
    format!("{error:?}")
}
