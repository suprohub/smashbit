use crate::{
    camera_controller::CameraController,
    renderer::{
        Renderer,
        pipeline::{InstanceRaw, color::ColoredVertex, texture::TexturedVertex},
        texture,
    },
};
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
    audio: Option<AudioManager>,
    last_frame: Instant,
    camera_controller: CameraController,
    mouse_left: bool,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            renderer: None,
            audio: None,
            last_frame: Instant::now(),
            camera_controller: CameraController::default(),
            mouse_left: false,
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

        let mut renderer = pollster::block_on(Renderer::new(window)).unwrap();

        #[rustfmt::skip]
        let cube_vertices = vec![
            ColoredVertex { position: [-1.0, -1.0,  1.0], color: [1.0, 0.0, 0.0] }, // 0
            ColoredVertex { position: [ 1.0, -1.0,  1.0], color: [0.0, 1.0, 0.0] }, // 1
            ColoredVertex { position: [ 1.0,  1.0,  1.0], color: [0.0, 0.0, 1.0] }, // 2
            ColoredVertex { position: [-1.0,  1.0,  1.0], color: [1.0, 1.0, 0.0] }, // 3

            ColoredVertex { position: [-1.0, -1.0, -1.0], color: [1.0, 0.0, 1.0] }, // 4
            ColoredVertex { position: [ 1.0, -1.0, -1.0], color: [0.0, 1.0, 1.0] }, // 5
            ColoredVertex { position: [ 1.0,  1.0, -1.0], color: [0.0, 0.0, 0.0] }, // 6
            ColoredVertex { position: [-1.0,  1.0, -1.0], color: [1.0, 1.0, 1.0] }, // 7
        ];

        #[rustfmt::skip]
        let cube_indices = vec![
            0, 1, 2, 2, 3, 0,
            1, 5, 6, 6, 2, 1,
            5, 4, 7, 7, 6, 5,
            4, 0, 3, 3, 7, 4,
            3, 2, 6, 6, 7, 3,
            4, 5, 1, 1, 0, 4,
        ];

        let instances = (0..10)
            .map(|i| InstanceRaw {
                model: glam::Mat4::from_translation(glam::Vec3::new(i as f32 * 3.0, 0.0, 0.0))
                    .to_cols_array_2d(),
            })
            .collect::<Vec<_>>();

        renderer.color_pipeline.add_mesh(
            &renderer.device,
            &cube_vertices,
            &cube_indices,
            &instances,
        );

        let gltf = Gltf::from_slice(&fs::read("map.glb").unwrap()).unwrap();
        let mut instances = HashMap::<String, Vec<InstanceRaw>>::new();
        let mut textured_meshes =
            HashMap::<String, (Vec<TexturedVertex>, Vec<u16>, Vec<u8>)>::new();
        let mut colored_meshes = HashMap::<String, (Vec<ColoredVertex>, Vec<u16>, [f32; 4])>::new();

        let blob = gltf.blob.clone().unwrap();

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
        // Process textured meshes
        for (name, (vertices, indices, image_data)) in textured_meshes {
            let texture =
                texture::Texture::from_bytes(&renderer.device, &renderer.queue, &image_data, &name)
                    .unwrap();
            let instances = instances.remove(name.as_str()).unwrap_or_default();
            renderer.texture_pipeline.add_mesh(
                &renderer.device,
                &texture,
                &vertices,
                &indices,
                &instances,
            );
        }

        // Process colored meshes
        for (name, (vertices, indices, base_color)) in colored_meshes {
            let instances = instances.remove(name.as_str()).unwrap_or_default();
            renderer
                .color_pipeline
                .add_mesh(&renderer.device, &vertices, &indices, &instances);
        }

        self.renderer = Some(renderer);
        self.audio = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).ok();

        if let Some(audio) = &mut self.audio {
            audio
                .play(StaticSoundData::from_file("0.ogg").unwrap())
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

                    renderer.window.request_redraw();

                    renderer.render().unwrap();
                    self.camera_controller
                        .update_camera(&mut renderer.camera, dt);
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
}
