use crate::{
    camera_controller::CameraController,
    renderer::Renderer,
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

        renderer.add_gltf("map.glb");

        self.renderer = Some(renderer);
        self.audio = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).ok();

        if let Some(audio) = &mut self.audio {
            audio
                .play(StaticSoundData::from_file("assets/music/0.ogg").unwrap())
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
}
