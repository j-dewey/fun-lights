//
// A basic deferred rendering implementation
//
// Design Notes:
//  -

use engine_core::Spawner;
use engine_core::query::Query;
use engine_core::render_core::{DynamicNodeDefinition, DynamicRenderPipeline, GivesBindGroup};
use frosty_alloc::Tag;
use render::mesh::{MeshData, MeshyObject};
use render::scheduled_pipeline::{
    ScheduledBindGroup, ScheduledBindGroupType, ScheduledBuffer, ScheduledTexture, ScheduledUniform,
};
use render::shader::default_render_target;
use render::texture::{
    self, DEFAULT_TEXTURE_BIND_GROUP_LAYOUT_DESCRIPTOR,
    RENDER_TEXTURE_BIND_GROUP_LAYOUT_DESCRIPTOR, Texture, render_texture_target,
};
use render::vertex::ScreenQuadVertex;
use render::wgpu::{
    self, BindGroupLayout, BufferUsages, CompareFunction, DepthBiasState, DepthStencilState,
    StencilState, TextureFormat,
};
use render::winit::dpi::PhysicalSize;
use render::{
    mesh::Mesh,
    scheduled_pipeline::{
        ScheduledPipelineDescription, ScheduledShaderNodeDescription, ShaderLabel,
    },
    shader::ShaderDefinition,
    vertex::{MeshVertex, Vertex},
    window_state::WindowState,
};

// These node layouts will automatically init missing components required for
// rendering
use engine_core::{MASTER_THREAD, query::DynQuery};

use basic_3d::camera::Camera3d;

//  https://learnopengl.com/Advanced-Lighting/Deferred-Shading
//
// | Meshes |   | Textures |   | Camera |
// ----------   ------------   ----------
//      |           |              |
//      V           V              V
//  -------------------------------------
//  |                                   |
//  |               g_pass              |
//  |                                   |
//  -------------------------------------
//                  | | | |
//  g_buffer        V V V V
//  -------------------------------
//  |  { position } { specular }  |
//  |   { albedo }   { normal }   |
//  -------------------------------
//                  | | | |
//  |  Screen |     | | | |
//  |   Quad  |     | | | |
//  -----------     | | | |
//        |         | | | |
//        V         V V V V
//  -------------------------------
//  |        Lighting Pass        |
//  |               /             | ----> { Screen }
//  |        screen effects       |
//  -------------------------------

// For the gpass
const MESH_BUFFER_LABEL: ShaderLabel = ShaderLabel("mesh-buffer");
const CAMERA_BUFFER_LABEL: ShaderLabel = ShaderLabel("camera-uniform");
const G_PASS_LABEL: ShaderLabel = ShaderLabel("g-pass-shader");
const TEXTURE_BUFFER_LABEL: ShaderLabel = ShaderLabel("texture-buffer");

// G Buffer
// specular is stored in albedo alpha channel
// may switch this to the normal buffer when implementing transparency
//
// also, the dpeth buffer isn't necessarily a part of the g buffer, but
// it is needed for some post processing effects that are applied during
// the lighting pass
const G_BUFFER_LABEL: ShaderLabel = ShaderLabel("g-buffer-bg");
const G_POSITION_LABEL: ShaderLabel = ShaderLabel("g-position-buffer");
const G_ALBEDO_LABEL: ShaderLabel = ShaderLabel("g-albedo-buffer");
const G_NORMAL_LABEL: ShaderLabel = ShaderLabel("g-normal-buffer");
const G_DEPTH_LABEL: ShaderLabel = ShaderLabel("g-depth-buffer");

// For the lighting pass
const SCREEN_QUAD_BUFFER_LABEL: ShaderLabel = ShaderLabel("screen-quad-buffer");
const COMPOSITE_PASS_LABEL: ShaderLabel = ShaderLabel("composite-shader");

const CAMERA_BG_INDEX: usize = 0;

const RENDER_DIMENSIONS: PhysicalSize<u32> = PhysicalSize {
    width: 1920,
    height: 1080,
};

fn load_g_pass_shader_layout<'a>(
    alloc: &mut Spawner,
    layouts: &'a [&'a BindGroupLayout],
    render_targets: &'a [Option<wgpu::ColorTargetState>],
    ws: &WindowState,
) -> (
    ScheduledShaderNodeDescription<'a>,
    DynamicNodeDefinition<Mesh<MeshVertex>>,
    Vec<MeshData>,
    Box<[u8]>,
) {
    let mut bind_groups: DynQuery<dyn GivesBindGroup> = DynQuery::new_empty();
    let mut camera = alloc
        .get_query::<Camera3d>(MASTER_THREAD)
        .expect("No Camera3d detected during mesh shader init")
        .next_handle()
        .expect("Failed to get Camera3D from Query");

    camera.set_tag_3(Tag {
        double: [CAMERA_BG_INDEX as u16, 0],
    });

    let camera_data = camera
        .get_access(MASTER_THREAD)
        .expect("Camera3D lost during Shader Init")
        .as_ref()
        .get_uniform_data();

    bind_groups.push(&mut camera);

    let mut mesh_data = Vec::new();
    if let Some(mut meshes) = alloc.get_query::<Mesh<MeshVertex>>(MASTER_THREAD) {
        meshes.for_each(|mesh| {
            let (inds, inds_count) = mesh.as_ref().get_indices();
            let verts = mesh.as_ref().get_verts();

            let i_buf = ws.load_index_buffer("mesh_indices", &inds[..]);
            let v_buf = ws.load_vertex_buffer("mesh_vertices", &verts[..]);
            mesh_data.push(MeshData {
                v_buf,
                i_buf,
                num_indices: inds_count as u32,
                unique_bind_groups: vec![],
            });
        });
    }

    let depth = Texture::new_depth_non_filter("depth_texture", ws.size, &ws.device);

    let schedule_node = ScheduledShaderNodeDescription {
        buffer_group: MESH_BUFFER_LABEL,
        bind_groups: vec![CAMERA_BUFFER_LABEL, TEXTURE_BUFFER_LABEL], // camera, texture array
        targets: Some(vec![G_NORMAL_LABEL, G_ALBEDO_LABEL, G_POSITION_LABEL]),
        depth: Some(G_DEPTH_LABEL),
        shader: ShaderDefinition {
            shader_source: include_str!("g_pass.wgsl"),
            bg_layouts: layouts, // camera, texture array
            const_ranges: &[],
            vertex_desc: MeshVertex::desc(),
            primitive_state: render::wgpu::PrimitiveState::default(),
            depth_buffer: Some(depth),
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::LessEqual,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            targets: render_targets,
        },
    };

    let dynamic_node = DynamicNodeDefinition {
        bind_groups,
        node: MESH_BUFFER_LABEL,
        _pd: std::marker::PhantomData {},
    };

    (schedule_node, dynamic_node, mesh_data, camera_data)
}

fn load_composite_pass_shader_layout<'a>(
    alloc: &mut Spawner,
    layouts: &'a [&'a BindGroupLayout],
    render_targets: &'a [Option<wgpu::ColorTargetState>],
    ws: &WindowState,
) -> (
    ScheduledShaderNodeDescription<'a>,
    DynamicNodeDefinition<Mesh<ScreenQuadVertex>>,
    Vec<MeshData>,
) {
    // Collect screen quad
    let mut quad_query: Query<Mesh<ScreenQuadVertex>> = alloc
        .get_query(MASTER_THREAD)
        .expect("failed to allocate screen quad before shader init");
    let quad_access = quad_query
        .next(MASTER_THREAD)
        .expect("failed to read screen quad from query");
    let quad = quad_access.as_ref();

    let i_buf = ws.load_index_buffer("screen-quad-indices", quad.indices.get_bytes());
    let v_buf = ws.load_vertex_buffer("screen-quad-verts", &quad.get_verts());
    let mesh_data = MeshData {
        v_buf,
        i_buf,
        num_indices: 6,
        unique_bind_groups: vec![],
    };

    // Set up shader
    let schedule_node = ScheduledShaderNodeDescription {
        buffer_group: SCREEN_QUAD_BUFFER_LABEL,
        bind_groups: vec![G_NORMAL_LABEL, G_ALBEDO_LABEL, G_POSITION_LABEL],
        targets: None,
        depth: None,
        shader: ShaderDefinition {
            shader_source: include_str!("composite.wgsl"),
            bg_layouts: layouts,
            const_ranges: &[],
            vertex_desc: ScreenQuadVertex::desc(),
            primitive_state: render::wgpu::PrimitiveState::default(),
            depth_buffer: None,
            depth_stencil: None,
            targets: render_targets,
        },
    };

    let dynamic_node = DynamicNodeDefinition {
        bind_groups: DynQuery::new_empty(),
        node: SCREEN_QUAD_BUFFER_LABEL,
        _pd: std::marker::PhantomData {},
    };

    (schedule_node, dynamic_node, vec![mesh_data])
}

// For now this just renders meshes
pub fn deferred_3d_pipeline(alloc: &mut Spawner, ws: &WindowState) -> DynamicRenderPipeline {
    // Load Textures
    let texture_bg_layout = ws
        .device
        .create_bind_group_layout(&DEFAULT_TEXTURE_BIND_GROUP_LAYOUT_DESCRIPTOR);
    let render_texture_bg_layout = ws
        .device
        .create_bind_group_layout(&RENDER_TEXTURE_BIND_GROUP_LAYOUT_DESCRIPTOR);
    let (default_text_desc, default_view_desc, default_sample_desc) =
        basic_3d::render::load_default_textures(ws);

    let camera_layout = Camera3d::get_bind_group_layout(ws);
    let layouts = &[&camera_layout, &texture_bg_layout];
    let render_targets = &[
        render_texture_target(None),
        render_texture_target(None),
        render_texture_target(None),
    ];
    let (g_pass_snode, g_pass_dnode, mesh_data, camera) =
        load_g_pass_shader_layout(alloc, &layouts[..], render_targets, ws);

    let layouts = &[
        &render_texture_bg_layout,
        &render_texture_bg_layout,
        &render_texture_bg_layout,
    ];
    let render_targets = &[default_render_target(None, &ws.config)];
    let (composite_snode, composite_dnode, screen_quad) =
        load_composite_pass_shader_layout(alloc, layouts, render_targets, ws);

    let rp = ScheduledPipelineDescription {
        shader_nodes: vec![g_pass_snode, composite_snode],
        buffers: vec![
            (MESH_BUFFER_LABEL, mesh_data),
            (SCREEN_QUAD_BUFFER_LABEL, screen_quad),
        ],
        bind_groups: vec![
            ScheduledBindGroup {
                label: CAMERA_BUFFER_LABEL,
                form: ScheduledBindGroupType::Uniform(ScheduledUniform {
                    layout: &Camera3d::get_bind_group_layout(ws),
                    buffers: &[ScheduledBuffer {
                        desc: render::wgpu::util::BufferInitDescriptor {
                            label: Some(CAMERA_BUFFER_LABEL.0),
                            contents: &camera[..],
                            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                        },
                    }],
                }),
            },
            ScheduledBindGroup {
                label: TEXTURE_BUFFER_LABEL,
                form: ScheduledBindGroupType::ReadOnlyTexture(ScheduledTexture::Unloaded {
                    label: TEXTURE_BUFFER_LABEL,
                    desc: default_text_desc,
                    sample_desc: default_sample_desc,
                    view_desc: default_view_desc,
                    bg_layout_desc: DEFAULT_TEXTURE_BIND_GROUP_LAYOUT_DESCRIPTOR,
                    data: Some(Box::new([255, 0, 0, 0])), // red
                }),
            },
        ],
        textures: vec![
            ScheduledTexture::depth(G_DEPTH_LABEL, RENDER_DIMENSIONS),
            ScheduledTexture::render_target(G_NORMAL_LABEL, RENDER_DIMENSIONS, &ws.device),
            ScheduledTexture::render_target(G_ALBEDO_LABEL, RENDER_DIMENSIONS, &ws.device),
            ScheduledTexture::render_target(G_POSITION_LABEL, RENDER_DIMENSIONS, &ws.device),
        ],
    }
    .finalize(ws);

    DynamicRenderPipeline::new(rp, vec![MESH_BUFFER_LABEL])
        .register_shader::<Mesh<MeshVertex>, MeshVertex>(g_pass_dnode, ws, alloc)
        .register_shader::<Mesh<ScreenQuadVertex>, ScreenQuadVertex>(composite_dnode, ws, alloc)
}
