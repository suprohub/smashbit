use winit::dpi::PhysicalSize;

use crate::renderer::uniform::{camera::Camera, fog::Fog, light::Light};

pub mod camera;
pub mod fog;
pub mod light;

pub struct Uniforms {
    pub camera: Camera,
    pub light: Light,
    pub fog: Fog,

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl Uniforms {
    pub fn new(device: &wgpu::Device, size: &PhysicalSize<u32>) -> Self {
        let camera = Camera::new(device, size.width, size.height);
        let light = Light::new(device);
        let fog = Fog::new(device);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Base bind group layout"),
            entries: &[
                camera.bind_layout_entry,
                light.bind_layout_entry,
                fog.bind_layout_entry,
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Base bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: fog.buffer.as_entire_binding(),
                },
            ],
        });
        Self {
            camera,
            light,
            fog,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn resize(&mut self, size: &PhysicalSize<u32>) {
        self.camera.resize(size.width, size.height);
    }

    pub fn update(&self, queue: &wgpu::Queue) {
        self.camera.update(queue);
    }
}
