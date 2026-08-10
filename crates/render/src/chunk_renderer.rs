//! ChunkRenderer: wgpu pipeline that draws voxel chunk meshes.
//! Uses a procedural colored atlas so terrain.png is not required.

use std::collections::HashMap;
use bytemuck::cast_slice;
use glam::{IVec2, Mat4, Vec3};
use wgpu::util::DeviceExt;
use wgpu::*;

use crate::mesh::{ChunkVertex, build_chunk_mesh};
use world::World;

// ---- WGSL shader (inlined so no path issues) --------------------------------
const CHUNK_WGSL: &str = r#"
struct Camera {
    view_proj:  mat4x4<f32>,
    camera_pos: vec3<f32>,
    _pad:       f32,
}
@group(0) @binding(0) var<uniform> cam: Camera;
@group(1) @binding(0) var t_terrain: texture_2d<f32>;
@group(1) @binding(1) var s_terrain: sampler;

struct VIn {
    @location(0) position: vec3<f32>,
    @location(1) uv:       vec2<f32>,
    @location(2) normal:   vec3<f32>,
    @location(3) ao:       f32,
}
struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv:    vec2<f32>,
    @location(1) light: f32,
    @location(2) ao:    f32,
}
@vertex
fn vs_main(v: VIn) -> VOut {
    var o: VOut;
    o.clip  = cam.view_proj * vec4<f32>(v.position, 1.0);
    o.uv    = v.uv;
    let sun = normalize(vec3<f32>(0.6, 1.0, 0.4));
    o.light = clamp(dot(v.normal, sun), 0.25, 1.0);
    o.ao    = v.ao;
    return o;
}
@fragment
fn fs_main(f: VOut) -> @location(0) vec4<f32> {
    let c = textureSample(t_terrain, s_terrain, f.uv);
    if c.a < 0.1 { discard; }
    let b = f.light * (0.5 + 0.5 * f.ao);
    return vec4<f32>(c.rgb * b, c.a);
}
"#;

// ---- Camera uniform (mirrors the WGSL struct) --------------------------------
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj:  [[f32; 4]; 4],
    camera_pos: [f32; 3],
    _pad:       f32,
}

// ---- Per-chunk GPU buffers ---------------------------------------------------
pub struct GpuChunk {
    pub vbo:         Buffer,
    pub ibo:         Buffer,
    pub index_count: u32,
}

// ---- ChunkRenderer -----------------------------------------------------------
pub struct ChunkRenderer {
    pipeline:       RenderPipeline,
    cam_buf:        Buffer,
    cam_bg:         BindGroup,
    tex_bg:         BindGroup,
    pub gpu_chunks: HashMap<IVec2, GpuChunk>,
}

impl ChunkRenderer {
    pub fn new(device: &Device, queue: &Queue, surface_fmt: TextureFormat) -> Self {
        // Shader
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label:  Some("chunk-shader"),
            source: ShaderSource::Wgsl(CHUNK_WGSL.into()),
        });

        // Camera UBO
        let cam_buf = device.create_buffer(&BufferDescriptor {
            label:             Some("cam-ubo"),
            size:              std::mem::size_of::<CameraUniform>() as u64,
            usage:             BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let cam_bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label:   Some("cam-bgl"),
            entries: &[BindGroupLayoutEntry {
                binding:    0,
                visibility: ShaderStages::VERTEX,
                ty:         BindingType::Buffer {
                    ty:                 BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size:   None,
                },
                count: None,
            }],
        });
        let cam_bg = device.create_bind_group(&BindGroupDescriptor {
            label:   Some("cam-bg"),
            layout:  &cam_bgl,
            entries: &[BindGroupEntry { binding: 0, resource: cam_buf.as_entire_binding() }],
        });

        // Procedural texture atlas (no terrain.png needed)
        let atlas_bytes = make_colored_atlas();
        let atlas_size  = Extent3d { width: 256, height: 256, depth_or_array_layers: 1 };
        let atlas_tex   = device.create_texture(&TextureDescriptor {
            label:           Some("atlas"),
            size:            atlas_size,
            mip_level_count: 1,
            sample_count:    1,
            dimension:       TextureDimension::D2,
            format:          TextureFormat::Rgba8UnormSrgb,
            usage:           TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats:    &[],
        });
        queue.write_texture(
            atlas_tex.as_image_copy(),
            &atlas_bytes,
            ImageDataLayout {
                offset:         0,
                bytes_per_row:  Some(256 * 4),
                rows_per_image: Some(256),
            },
            atlas_size,
        );
        let atlas_view = atlas_tex.create_view(&TextureViewDescriptor::default());
        let atlas_samp = device.create_sampler(&SamplerDescriptor {
            label:       Some("atlas-samp"),
            mag_filter:  FilterMode::Nearest,
            min_filter:  FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });
        let tex_bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label:   Some("tex-bgl"),
            entries: &[
                BindGroupLayoutEntry {
                    binding:    0,
                    visibility: ShaderStages::FRAGMENT,
                    ty:         BindingType::Texture {
                        sample_type:    TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled:   false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding:    1,
                    visibility: ShaderStages::FRAGMENT,
                    ty:         BindingType::Sampler(SamplerBindingType::Filtering),
                    count:      None,
                },
            ],
        });
        let tex_bg = device.create_bind_group(&BindGroupDescriptor {
            label:   Some("tex-bg"),
            layout:  &tex_bgl,
            entries: &[
                BindGroupEntry { binding: 0, resource: BindingResource::TextureView(&atlas_view) },
                BindGroupEntry { binding: 1, resource: BindingResource::Sampler(&atlas_samp) },
            ],
        });

        // ChunkVertex layout: pos[f32;3] @ 0, uv[f32;2] @ 12, normal[f32;3] @ 20, ao[f32] @ 32
        let vbl = VertexBufferLayout {
            array_stride: std::mem::size_of::<ChunkVertex>() as u64,
            step_mode:    VertexStepMode::Vertex,
            attributes:   &[
                VertexAttribute { format: VertexFormat::Float32x3, offset:  0, shader_location: 0 },
                VertexAttribute { format: VertexFormat::Float32x2, offset: 12, shader_location: 1 },
                VertexAttribute { format: VertexFormat::Float32x3, offset: 20, shader_location: 2 },
                VertexAttribute { format: VertexFormat::Float32x1, offset: 32, shader_location: 3 },
            ],
        };

        let pl = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:                Some("chunk-pl"),
            bind_group_layouts:   &[&cam_bgl, &tex_bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label:  Some("chunk-pipeline"),
            layout: Some(&pl),
            vertex: VertexState {
                module:              &shader,
                entry_point:         Some("vs_main"),
                buffers:             &[vbl],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module:              &shader,
                entry_point:         Some("fs_main"),
                targets:             &[Some(ColorTargetState {
                    format:     surface_fmt,
                    blend:      Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology:  PrimitiveTopology::TriangleList,
                cull_mode: Some(Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format:               TextureFormat::Depth32Float,
                depth_write_enabled:  true,
                depth_compare:        CompareFunction::Less,
                stencil:              Default::default(),
                bias:                 Default::default(),
            }),
            multisample: MultisampleState::default(),
            multiview:   None,
            cache:       None,
        });

        Self { pipeline, cam_buf, cam_bg, tex_bg, gpu_chunks: HashMap::new() }
    }

    /// Write updated view-projection + camera position to the GPU uniform buffer.
    pub fn update_camera(&self, queue: &Queue, vp: Mat4, cam_pos: Vec3) {
        let u = CameraUniform {
            view_proj:  vp.to_cols_array_2d(),
            camera_pos: cam_pos.to_array(),
            _pad:       0.0,
        };
        queue.write_buffer(&self.cam_buf, 0, cast_slice(&[u]));
    }

    /// Build and upload a chunk mesh.  Call after dirty flag is set.
    pub fn upload_chunk(&mut self, device: &Device, key: IVec2, world: &World) {
        let chunk = match world.chunks.get(&key) { Some(c) => c, None => return };
        let mesh  = build_chunk_mesh(chunk, world);
        if mesh.vertices.is_empty() {
            self.gpu_chunks.remove(&key);
            return;
        }
        let vbo = device.create_buffer_init(&util::BufferInitDescriptor {
            label:    Some("chunk-vbo"),
            contents: cast_slice(&mesh.vertices),
            usage:    BufferUsages::VERTEX,
        });
        let ibo = device.create_buffer_init(&util::BufferInitDescriptor {
            label:    Some("chunk-ibo"),
            contents: cast_slice(&mesh.indices),
            usage:    BufferUsages::INDEX,
        });
        self.gpu_chunks.insert(key, GpuChunk {
            vbo,
            ibo,
            index_count: mesh.indices.len() as u32,
        });
    }

    /// Record draw calls for all uploaded chunks into an active render pass.
    pub fn draw<'rp>(&'rp self, rpass: &mut RenderPass<'rp>) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.cam_bg,  &[]);
        rpass.set_bind_group(1, &self.tex_bg,  &[]);
        for gc in self.gpu_chunks.values() {
            rpass.set_vertex_buffer(0, gc.vbo.slice(..));
            rpass.set_index_buffer(gc.ibo.slice(..), IndexFormat::Uint32);
            rpass.draw_indexed(0..gc.index_count, 0, 0..1);
        }
    }
}

// ---- Procedural atlas -------------------------------------------------------
/// Build a 256x256 RGBA8 texture with solid-colored 16x16 tiles.
/// Each tile index corresponds to the block_def.tex values in world/block.rs.
fn make_colored_atlas() -> Vec<u8> {
    // tile_index -> solid RGB  (tile 0 = top-left in a 16-column grid)
    let tiles: &[(usize, [u8; 3])] = &[
        (0,   [100, 200,  80]),  // grass top (bright green)
        (1,   [128, 128, 128]),  // stone
        (2,   [139,  90,  43]),  // dirt
        (3,   [ 90, 150,  60]),  // grass side
        (4,   [180, 140,  90]),  // planks
        (16,  [105, 105, 105]),  // cobblestone
        (17,  [ 38,  38,  38]),  // bedrock
        (18,  [194, 178, 128]),  // sand
        (19,  [138, 132, 128]),  // gravel
        (20,  [155, 128,  65]),  // log top
        (21,  [115,  80,  50]),  // log side
        (22,  [ 48, 140,  38]),  // leaves
        (32,  [220, 190,  40]),  // gold ore
        (33,  [180, 120,  90]),  // iron ore
        (34,  [ 52,  52,  52]),  // coal ore
        (49,  [180, 215, 245]),  // glass (light blue)
        (205, [ 28,  80, 210]),  // water (blue)
        (237, [225,  75,  15]),  // lava  (orange)
    ];

    // Fill with magenta (missing-tile indicator) then paint known tiles
    let mut data = vec![0u8; 256 * 256 * 4];
    for px in data.chunks_exact_mut(4) {
        px[0] = 255; px[1] = 0; px[2] = 255; px[3] = 255;
    }

    for &(idx, rgb) in tiles {
        let tx = (idx % 16) * 16;  // pixel x origin of tile
        let ty = (idx / 16) * 16;  // pixel y origin of tile
        if tx >= 256 || ty >= 256 { continue; }

        for py in 0..16usize {
            for px in 0..16usize {
                let x = tx + px;
                let y = ty + py;
                let i = (y * 256 + x) * 4;
                // subtle checkerboard shading so tiles look textured
                let shade: u8 = if (px + py) % 2 == 0 { 14 } else { 0 };
                data[i]   = rgb[0].saturating_sub(shade);
                data[i+1] = rgb[1].saturating_sub(shade);
                data[i+2] = rgb[2].saturating_sub(shade);
                data[i+3] = 255;
            }
        }
    }
    data
}
