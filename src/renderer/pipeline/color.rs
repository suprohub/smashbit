use crate::renderer::{mesh::Mesh, texture};
use glam::Vec3;
use litemap::LiteMap;
use wgpu::{ShaderModuleDescriptor, ShaderSource, util::DeviceExt};

use super::InstanceRaw;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ColoredVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub normal: [f32; 3],
}

impl ColoredVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ColoredVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as _,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as _,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct ColorPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub meshes: LiteMap<u64, Mesh>,
}

impl ColorPipeline {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        base_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Color shader"),
            source: ShaderSource::Wgsl(wesl::include_wesl!("main").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Color Pipeline Layout"),
            bind_group_layouts: &[base_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Color Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[ColoredVertex::desc(), super::InstanceRaw::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            meshes: LiteMap::new(),
        }
    }

    pub fn add_mesh(
        &mut self,
        device: &wgpu::Device,
        id: u64,
        vertices: &[ColoredVertex],
        indices: &[u16],
        instances: &[InstanceRaw],
    ) {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(instances),
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });

        self.meshes.insert(
            id,
            Mesh {
                vertex_buffer,
                index_buffer,
                index_count: indices.len() as u32,
                instance_buffer,
                instance_capacity: instances.len() as u32,
                instances: instances.to_vec(),
            },
        );
    }

    pub fn begin_render_pass(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        for mesh in self.meshes.values() {
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, mesh.instance_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..mesh.index_count, 0, 0..mesh.instances.len() as u32);
        }
    }
}

pub fn generate_sphere(
    radius: f32,
    sectors: u16,
    stacks: u16,
    color: [f32; 3],
) -> (Vec<ColoredVertex>, Vec<u16>) {
    let vertex_count = (stacks as usize + 1) * (sectors as usize + 1);
    let index_count = if stacks >= 2 {
        6 * sectors as usize * (stacks as usize - 1)
    } else {
        0
    };

    let mut vertices = Vec::with_capacity(vertex_count);
    let mut indices = Vec::with_capacity(index_count);

    let sector_step = 2.0 * std::f32::consts::PI / sectors as f32;
    let stack_step = std::f32::consts::PI / stacks as f32;

    // Precompute sector trigonometric values
    let mut sector_cos = Vec::with_capacity(sectors as usize + 1);
    let mut sector_sin = Vec::with_capacity(sectors as usize + 1);
    for j in 0..=sectors {
        let sector_angle = j as f32 * sector_step;
        sector_cos.push(sector_angle.cos());
        sector_sin.push(sector_angle.sin());
    }

    // Precompute stack trigonometric values
    let mut stack_cos = Vec::with_capacity(stacks as usize + 1);
    let mut stack_sin = Vec::with_capacity(stacks as usize + 1);
    for i in 0..=stacks {
        let stack_angle = std::f32::consts::PI / 2.0 - i as f32 * stack_step;
        stack_cos.push(stack_angle.cos());
        stack_sin.push(stack_angle.sin());
    }

    for i in 0..=stacks {
        let i_idx = i as usize;
        let xy = radius * stack_cos[i_idx];
        let z = radius * stack_sin[i_idx];

        for j in 0..=sectors {
            let j_idx = j as usize;
            let x = xy * sector_cos[j_idx];
            let y = xy * sector_sin[j_idx];
            let normal = Vec3::new(x, y, z).normalize();

            vertices.push(ColoredVertex {
                position: [x, y, z],
                color,
                normal: normal.to_array(),
            });
        }
    }

    for i in 0..stacks {
        let mut k1 = i * (sectors + 1);
        let mut k2 = k1 + sectors + 1;

        for _ in 0..sectors {
            if i != 0 {
                indices.push(k1);
                indices.push(k2);
                indices.push(k1 + 1);
            }
            if i != stacks - 1 {
                indices.push(k1 + 1);
                indices.push(k2);
                indices.push(k2 + 1);
            }
            k1 += 1;
            k2 += 1;
        }
    }

    (vertices, indices)
}
