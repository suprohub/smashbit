use anyhow::Result;
use camera::Camera;
use gltf::Gltf;
use pipeline::{
    InstanceRaw,
    color::{ColorPipeline, ColoredVertex},
    texture::{TexturePipeline, TexturedVertex},
};
use std::{collections::HashMap, fs, sync::Arc};
use winit::{dpi::PhysicalSize, window::Window};

pub mod camera;
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
            .await
            .ok_or(anyhow::anyhow!("No compatible adapter found"))?;

        log::info!("Requesting device & queue");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device & Queue"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
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

        Ok(Self {
            color_pipeline: ColorPipeline::new(&device, surface_format, &camera.bind_group_layout),
            texture_pipeline: TexturePipeline::new(
                &device,
                surface_format,
                &camera.bind_group_layout,
            ),
            depth_texture,
            camera,
            window,
            surface,
            surface_config,
            device,
            queue,
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

            self.color_pipeline
                .begin_render_pass(&mut render_pass, &self.camera.bind_group);
            self.texture_pipeline
                .begin_render_pass(&mut render_pass, &self.camera.bind_group);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }

    pub fn add_gltf(&mut self, path: &str) {
        log::info!("Adding gltf to scene");

        let gltf = Gltf::from_slice(&fs::read(path).unwrap()).unwrap();
        let mut instances = HashMap::<String, Vec<InstanceRaw>>::new();
        let mut textured_meshes =
            HashMap::<String, (Vec<TexturedVertex>, Vec<u16>, Vec<u8>)>::new();
        let mut colored_meshes = HashMap::<String, (Vec<ColoredVertex>, Vec<u16>, [f32; 4])>::new();

        let blob = gltf.blob.clone().unwrap();

        log::info!("Data collection");
        // Phase 1: Data collection
        for node in gltf.nodes() {
            if let Some(mesh) = node.mesh() {
                let Some(name) = mesh.name() else { continue };

                if let Some((base_name, _)) = name.split_once('.') {
                    // Handle instances
                    instances
                        .entry(base_name.to_string())
                        .or_default()
                        .push(InstanceRaw {
                            model: node.transform().matrix(),
                        });
                } else {
                    // Handle meshes
                    for primitive in mesh.primitives() {
                        let reader = primitive.reader(|buffer| {
                            if buffer.index() == 0 {
                                Some(&blob)
                            } else {
                                None
                            }
                        });
                        let Some(indices) = reader
                            .read_indices()
                            .map(|i| i.into_u32().map(|v| v as u16).collect::<Vec<_>>())
                        else {
                            continue;
                        };
                        let positions = reader.read_positions().unwrap().collect::<Vec<_>>();

                        // Try to read texture data
                        let mut texture_data = None;
                        if let Some(tex_coords) = reader.read_tex_coords(0) {
                            let tex_coords = tex_coords.into_f32().collect::<Vec<_>>();
                            if let Some(texture_info) = primitive
                                .material()
                                .pbr_metallic_roughness()
                                .base_color_texture()
                            {
                                if let gltf::image::Source::View { view, .. } =
                                    texture_info.texture().source().source()
                                {
                                    texture_data = Some((
                                        positions
                                            .iter()
                                            .zip(tex_coords.iter())
                                            .map(|(pos, uv)| TexturedVertex {
                                                position: *pos,
                                                tex_coords: *uv,
                                            })
                                            .collect(),
                                        indices.clone(),
                                        blob[view.offset()..view.offset() + view.length()].to_vec(),
                                    ));
                                }
                            }
                        }

                        if let Some((vertices, indices, image_data)) = texture_data {
                            textured_meshes
                                .insert(name.to_string(), (vertices, indices, image_data));
                        } else {
                            // Handle colored mesh
                            let colors = reader
                                .read_colors(0)
                                .map(|c| c.into_rgba_f32().map(|v| [v[0], v[1], v[2]]).collect())
                                .unwrap_or_else(|| {
                                    let base = primitive
                                        .material()
                                        .pbr_metallic_roughness()
                                        .base_color_factor();
                                    vec![[base[0], base[1], base[2]]; positions.len()]
                                });

                            colored_meshes.insert(
                                name.to_string(),
                                (
                                    positions
                                        .iter()
                                        .zip(colors.iter())
                                        .map(|(pos, color)| ColoredVertex {
                                            position: *pos,
                                            color: *color,
                                        })
                                        .collect(),
                                    indices,
                                    primitive
                                        .material()
                                        .pbr_metallic_roughness()
                                        .base_color_factor(),
                                ),
                            );
                        }
                    }
                }
            }
        }

        // Phase 2: Resource creation and rendering
        log::info!("Process colored meshes");
        // Process colored meshes
        for (name, (vertices, indices, base_color)) in colored_meshes {
            let instances = instances.remove(name.as_str()).unwrap_or_default();
            self.color_pipeline
                .add_mesh(&self.device, &vertices, &indices, &instances);
        }

        log::info!("Process textured meshes");
        // Process textured meshes
        for (name, (vertices, indices, image_data)) in textured_meshes {
            let texture =
                texture::Texture::from_bytes(&self.device, &self.queue, &image_data, &name)
                    .unwrap();
            let instances = instances.remove(name.as_str()).unwrap_or_default();
            self.texture_pipeline
                .add_mesh(&self.device, &texture, &vertices, &indices, &instances);
        }
    }
}
