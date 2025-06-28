use crate::{
    camera_controller::CameraController,
    physics::Physics,
    renderer::{
        Renderer,
        pipeline::{InstanceRaw, color::ColoredVertex, texture::TexturedVertex},
        texture::Texture,
    },
};
use glam::{Mat3, Mat4, Vec2, Vec3};
use gltf::Gltf;
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, sound::static_sound::StaticSoundData,
};
use std::{collections::HashMap, fs, sync::Arc, time::Instant};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, MouseButton, WindowEvent},
    keyboard::PhysicalKey,
    window::WindowAttributes,
};

pub struct Game {
    renderer: Option<Renderer>,
    physics: Physics,
    audio: Option<AudioManager>,
    last_frame: Instant,
    camera_controller: CameraController,
    mouse_left: bool,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            renderer: None,
            physics: Physics::new(),
            audio: None,
            last_frame: Instant::now(),
            camera_controller: CameraController::default(),
            mouse_left: false,
        }
    }
}

impl Game {
    pub fn add_gltf(&mut self, path: &str) {
        if let Some(renderer) = &mut self.renderer {
            log::info!("Adding gltf to scene");

            let gltf = Gltf::from_slice(&fs::read(path).unwrap()).unwrap();
            let mut instances: HashMap<String, Vec<InstanceRaw>> = HashMap::new();
            let mut textured_meshes: HashMap<String, (Vec<TexturedVertex>, Vec<u16>, Vec<u8>)> =
                HashMap::new();
            let mut colored_meshes: HashMap<String, (Vec<ColoredVertex>, Vec<u16>, [f32; 4])> =
                HashMap::new();
            let blob = gltf.blob.clone().unwrap();

            log::info!("Data collection");
            for node in gltf.nodes() {
                let Some(mesh) = node.mesh() else { continue };
                let Some(name) = mesh.name() else { continue };

                let model_matrix =
                    Mat3::from_mat4(Mat4::from_cols_array_2d(&node.transform().matrix()));
                let normal_matrix = model_matrix.inverse().transpose();

                if let Some((base_name, _)) = name.split_once('.') {
                    instances
                        .entry(base_name.to_string())
                        .or_default()
                        .push(InstanceRaw {
                            model: node.transform().matrix(),
                            normal: normal_matrix.to_cols_array_2d(),
                        });
                    continue;
                }

                for primitive in mesh.primitives() {
                    let reader = primitive.reader(|buffer| {
                        if buffer.index() == 0 {
                            Some(&blob)
                        } else {
                            None
                        }
                    });

                    let indices: Vec<u16> = match reader
                        .read_indices()
                        .map(|i| i.into_u32().map(|v| v as u16))
                    {
                        Some(indices) => indices.collect(),
                        None => continue,
                    };

                    let positions: Vec<Vec3> =
                        reader.read_positions().unwrap().map(Vec3::from).collect();
                    if positions.is_empty() {
                        log::warn!("Mesh '{}' has no positions, skipping", name);
                        continue;
                    }

                    let normals = match reader.read_normals() {
                        Some(n) => n.map(Vec3::from).collect(),
                        None => {
                            log::warn!("Normals not found for '{}', generating...", name);
                            Renderer::compute_normals(&positions, &indices)
                        }
                    };

                    if let Some(tex_coords) = reader
                        .read_tex_coords(0)
                        .map(|t| t.into_f32().map(Vec2::from).collect::<Vec<_>>())
                    {
                        log::info!("Finded texture coords of {name}, trying load texture info");

                        if let Some(texture_info) = primitive
                            .material()
                            .pbr_metallic_roughness()
                            .base_color_texture()
                        {
                            log::info!("Texture info loaded, trying get texture");
                            if let gltf::image::Source::View { view, .. } =
                                texture_info.texture().source().source()
                            {
                                log::info!("Try load texture mesh");

                                let image_data =
                                    &blob[view.offset()..view.offset() + view.length()];

                                let vertices = positions
                                    .iter()
                                    .zip(tex_coords.iter())
                                    .zip(normals.iter())
                                    .map(|((pos, uv), normal)| TexturedVertex {
                                        position: pos.to_array(),
                                        tex_coords: [uv.x, uv.y],
                                        normal: normal.to_array(),
                                    })
                                    .collect();

                                textured_meshes.insert(
                                    name.to_string(),
                                    (vertices, indices, image_data.to_vec()),
                                );
                                continue;
                            }
                        }
                    }

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

                    let vertices = positions
                        .iter()
                        .zip(colors.iter())
                        .zip(normals.iter())
                        .map(|((pos, color), normal)| ColoredVertex {
                            position: pos.to_array(),
                            color: *color,
                            normal: normal.to_array(),
                        })
                        .collect();

                    colored_meshes.insert(
                        name.to_string(),
                        (
                            vertices,
                            indices,
                            primitive
                                .material()
                                .pbr_metallic_roughness()
                                .base_color_factor(),
                        ),
                    );
                }
            }

            log::info!("Processing meshes");
            for (name, (vertices, indices, _base_color)) in colored_meshes {
                let mut instances = instances.remove(&name).unwrap_or_default();
                if instances.is_empty() {
                    instances.push(InstanceRaw {
                        model: glam::Mat4::IDENTITY.to_cols_array_2d(),
                        normal: Mat3::IDENTITY.to_cols_array_2d(),
                    });
                }
                renderer
                    .color_pipeline
                    .add_mesh(&renderer.device, &vertices, &indices, &instances);
            }

            for (name, (vertices, indices, image_data)) in textured_meshes {
                let mut instances = instances.remove(&name).unwrap_or_default();
                if instances.is_empty() {
                    instances.push(InstanceRaw {
                        model: glam::Mat4::IDENTITY.to_cols_array_2d(),
                        normal: Mat3::IDENTITY.to_cols_array_2d(),
                    });
                }

                let texture =
                    Texture::from_bytes(&renderer.device, &renderer.queue, &image_data, &name)
                        .unwrap();

                renderer.texture_pipeline.add_mesh(
                    &renderer.device,
                    &texture,
                    &vertices,
                    &indices,
                    &instances,
                );
            }
        }
    }
}

impl ApplicationHandler for Game {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        log::info!("Game resumed!");

        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        );

        let renderer = pollster::block_on(Renderer::new(window)).unwrap();
        self.renderer = Some(renderer);
        self.audio = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).ok();

        self.add_gltf("map.glb");

        if let Some(audio) = &mut self.audio {
            audio
                .play(StaticSoundData::from_file("assets/music/12.ogg").unwrap())
                .unwrap();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                log::info!("Dropping renderer");
                // We need drop renderer or else we get SIGSEGV
                self.renderer = None;

                log::info!("Exiting");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = &mut self.renderer {
                    let now = Instant::now();
                    let dt = now - self.last_frame;
                    self.last_frame = now;

                    renderer.render().unwrap();
                    self.camera_controller
                        .update_camera(&mut renderer.camera, dt);
                }
            }
            WindowEvent::Resized(new_size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(new_size);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    self.camera_controller
                        .process_keyboard(keycode, event.state);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if MouseButton::Left == button {
                    self.mouse_left = state.is_pressed();
                }
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if self.mouse_left {
                self.camera_controller.process_mouse(delta);
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // Todo
        if let Some(renderer) = &self.renderer {
            renderer.window.request_redraw();
        }
    }
}
