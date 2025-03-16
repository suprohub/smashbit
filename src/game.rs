use crate::{
    camera_controller::CameraController,
    renderer::{
        Renderer,
        pipeline::{InstanceRaw, color::ColoredVertex},
    },
};
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, sound::static_sound::StaticSoundData,
};
use std::{sync::Arc, time::Instant};
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

        renderer.add_gltf("map.glb");

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
