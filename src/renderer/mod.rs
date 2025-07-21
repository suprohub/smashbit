use anyhow::Result;
use camera::Camera;
use glam::Vec3;
use light::Light;
use pipeline::{color::ColorPipeline, texture::TexturePipeline};
use std::sync::Arc;
use wgpu::Trace;
use winit::{dpi::PhysicalSize, window::Window};

pub mod camera;
pub mod light;
pub mod pipeline;
pub mod texture;

pub struct Renderer {
    pub window: Arc<Window>,

    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub depth_texture: texture::Texture,

    pub camera: Camera,

    pub color_pipeline: ColorPipeline,
    pub texture_pipeline: TexturePipeline,

    pub light: Light,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        log::info!("Creating renderer...");

        let size = window.inner_size();
        log::info!("Getting instance");
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        log::info!("Creating surface");

        let surface = instance.create_surface(window.clone())?;

        log::info!("Requesting adapter");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        log::info!("Requesting device & queue");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device & Queue"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: Trace::Off,
            })
            .await?;

        log::info!("Getting possible texture format");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        log::info!("Configuring surface");

        surface.configure(&device, &surface_config);

        let depth_texture = texture::Texture::create_depth_texture(
            &device,
            size.width,
            size.height,
            "depth_texture",
        );

        let camera = Camera::new(&device, size.width, size.height);

        let light = Light::new(&device);

        Ok(Self {
            color_pipeline: ColorPipeline::new(
                &device,
                surface_format,
                &camera.bind_group_layout,
                &light.light_bind_group_layout,
            ),
            texture_pipeline: TexturePipeline::new(
                &device,
                surface_format,
                &camera.bind_group_layout,
                &light.light_bind_group_layout,
            ),
            depth_texture,
            camera,
            window,
            surface,
            surface_config,
            device,
            queue,
            light,
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        log::info!("Resizing window");
        self.camera.resize(new_size.width, new_size.height);
        (self.surface_config.width, self.surface_config.height) = (new_size.width, new_size.height);
        self.surface.configure(&self.device, &self.surface_config);

        self.depth_texture = texture::Texture::create_depth_texture(
            &self.device,
            new_size.width,
            new_size.height,
            "depth_texture",
        );
    }

    pub fn render(&self) -> Result<()> {
        self.camera.update(&self.queue);
        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            self.color_pipeline.begin_render_pass(
                &mut render_pass,
                &self.camera.bind_group,
                &self.light.light_bind_group,
            );

            self.texture_pipeline.begin_render_pass(
                &mut render_pass,
                &self.camera.bind_group,
                &self.light.light_bind_group,
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }

    pub fn compute_normals(positions: &[Vec3], indices: &[u16]) -> Vec<Vec3> {
        let mut normals = vec![Vec3::ZERO; positions.len()];

        for tri in indices.chunks_exact(3) {
            let [a, b, c] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
            let edge1 = positions[b] - positions[a];
            let edge2 = positions[c] - positions[a];
            let normal = edge1.cross(edge2).normalize();

            normals[a] += normal;
            normals[b] += normal;
            normals[c] += normal;
        }

        normals.iter_mut().for_each(|n| *n = n.normalize());
        normals
    }
}
