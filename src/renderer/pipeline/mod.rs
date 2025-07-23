use winit::dpi::PhysicalSize;

use crate::renderer::pipeline::{
    background::BackgroundPipeline, color::ColorPipeline, hdr::HdrPipeline,
    texture::TexturePipeline,
};

pub mod background;
pub mod color;
pub mod hdr;
pub mod texture;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    pub model: [[f32; 4]; 4],
    pub normal: [[f32; 3]; 3],
}

impl InstanceRaw {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 16]>() as _,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 19]>() as _,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 22]>() as _,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct Pipelines {
    pub hdr_pipeline: HdrPipeline,
    pub background_pipeline: BackgroundPipeline,
    pub color_pipeline: ColorPipeline,
    pub texture_pipeline: TexturePipeline,
}

impl Pipelines {
    pub fn new(
        device: &wgpu::Device,
        size: &PhysicalSize<u32>,
        base_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let hdr_pipeline = HdrPipeline::new(device, size, base_bind_group_layout);
        Self {
            background_pipeline: BackgroundPipeline::new(
                device,
                hdr_pipeline.format(),
                base_bind_group_layout,
            ),
            color_pipeline: ColorPipeline::new(
                device,
                hdr_pipeline.format(),
                base_bind_group_layout,
            ),
            texture_pipeline: TexturePipeline::new(
                device,
                hdr_pipeline.format(),
                base_bind_group_layout,
            ),
            hdr_pipeline,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: &PhysicalSize<u32>) {
        self.hdr_pipeline.resize(device, size.width, size.height);
    }

    pub fn begin_render_pass(&self, pass: &mut wgpu::RenderPass) {
        self.background_pipeline.begin_render_pass(pass);

        self.color_pipeline.begin_render_pass(pass);
        self.texture_pipeline.begin_render_pass(pass);
    }
}
