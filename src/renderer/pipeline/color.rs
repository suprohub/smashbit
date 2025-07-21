use std::collections::HashMap;

use crate::renderer::texture;
use glam::Vec3;
use wgpu::util::DeviceExt;

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

pub struct ColorMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub instance_buffer: wgpu::Buffer,
    pub instance_capacity: u32,
    pub instances: Vec<InstanceRaw>,
}

impl ColorMesh {
    pub fn add_instance(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instance: &InstanceRaw,
    ) {
        if self.instances.len() as u32 >= self.instance_capacity {
            let new_capacity = (self.instance_capacity * 2).max(1);
            self.resize_instance_buffer(device, queue, new_capacity);
        }

        self.instances.push(*instance);
        let offset = ((self.instances.len() - 1) * std::mem::size_of::<InstanceRaw>())
            as wgpu::BufferAddress;
        queue.write_buffer(&self.instance_buffer, offset, bytemuck::bytes_of(instance));
    }

    pub fn remove_instance(
        &mut self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        instance_index: usize,
    ) {
        assert!(
            instance_index < self.instances.len(),
            "instance_index out of bounds"
        );

        let last_index = self.instances.len() - 1;
        if instance_index == last_index {
            self.instances.pop();
            return;
        }

        let last_instance = self.instances[last_index];
        self.instances[instance_index] = last_instance;
        self.instances.pop();

        let offset = (instance_index * std::mem::size_of::<InstanceRaw>()) as wgpu::BufferAddress;
        queue.write_buffer(
            &self.instance_buffer,
            offset,
            bytemuck::bytes_of(&last_instance),
        );
    }

    pub fn remove_instances_batch(
        &mut self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        indices: &[usize],
    ) {
        let new_instances = self
            .instances
            .iter()
            .enumerate()
            .filter(|(i, _)| !indices.contains(i))
            .map(|(_, inst)| *inst)
            .collect::<Vec<_>>();

        if !new_instances.is_empty() {
            queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&new_instances),
            );
        }

        self.instances = new_instances;
    }

    pub fn update_all_instances(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instances: &[InstanceRaw],
    ) {
        let new_count = instances.len() as u32;
        if new_count > self.instance_capacity {
            self.resize_instance_buffer(device, queue, new_count);
        }

        if !instances.is_empty() {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instances));
        }

        self.instances = instances.to_vec();
    }

    fn resize_instance_buffer(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        new_capacity: u32,
    ) {
        let new_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Resized Instance Buffer"),
            size: (new_capacity as usize * std::mem::size_of::<InstanceRaw>())
                as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        if !self.instances.is_empty() {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Instance Buffer Copy Encoder"),
            });

            encoder.copy_buffer_to_buffer(
                &self.instance_buffer,
                0,
                &new_buffer,
                0,
                (self.instances.len() * std::mem::size_of::<InstanceRaw>()) as wgpu::BufferAddress,
            );

            queue.submit(std::iter::once(encoder.finish()));
        }

        self.instance_buffer = new_buffer;
        self.instance_capacity = new_capacity;
    }

    pub fn update_instance(
        &mut self,
        queue: &wgpu::Queue,
        instance_index: usize,
        new_instance: &InstanceRaw,
    ) {
        if instance_index < self.instances.len() {
            self.instances[instance_index] = *new_instance;
            let offset =
                (instance_index * std::mem::size_of::<InstanceRaw>()) as wgpu::BufferAddress;
            queue.write_buffer(
                &self.instance_buffer,
                offset,
                bytemuck::bytes_of(new_instance),
            );
        }
    }
}

pub struct ColorPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub meshes: HashMap<u64, ColorMesh>,
}

impl ColorPipeline {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        light_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/main.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Color Pipeline Layout"),
            bind_group_layouts: &[camera_bind_group_layout, light_bind_group_layout],
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
            meshes: HashMap::new(),
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
            ColorMesh {
                vertex_buffer,
                index_buffer,
                index_count: indices.len() as u32,
                instance_buffer,
                instance_capacity: instances.len() as u32,
                instances: instances.to_vec(),
            },
        );
    }

    pub fn begin_render_pass(
        &self,
        render_pass: &mut wgpu::RenderPass,
        camera_bind_group: &wgpu::BindGroup,
        light_bind_group: &wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, light_bind_group, &[]);
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
