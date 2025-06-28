use std::{sync::Arc, time::Instant};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, MouseButton, WindowEvent},
    keyboard::PhysicalKey,
    window::WindowAttributes,
};

use crate::scene::Scene;

pub struct Game {
    scene: Option<Scene>,
    last_frame: Instant,
    mouse_left: bool,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            scene: None,
            last_frame: Instant::now(),
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
        let mut scene = Scene::new(window);

        scene.init_level();

        self.scene = Some(scene);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if let Some(scene) = &mut self.scene {
            match event {
                WindowEvent::RedrawRequested => {
                    let now = Instant::now();
                    let dt = now - self.last_frame;
                    self.last_frame = now;

                    scene.renderer.render().unwrap();
                    scene
                        .camera_controller
                        .update_camera(&mut scene.renderer.camera, dt);
                }
                WindowEvent::Resized(new_size) => {
                    if let Some(scene) = &mut self.scene {
                        scene.renderer.resize(new_size);
                    }
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    if let PhysicalKey::Code(keycode) = event.physical_key {
                        scene
                            .camera_controller
                            .process_keyboard(keycode, event.state);
                    }
                }
                WindowEvent::CloseRequested => {
                    log::info!("Dropping renderer");
                    // We need drop scene or else we get SIGSEGV
                    self.scene = None;

                    log::info!("Exiting");
                    event_loop.exit();
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    if MouseButton::Left == button {
                        self.mouse_left = state.is_pressed();
                    }
                }
                _ => {}
            }
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
                if let Some(scene) = &mut self.scene {
                    scene.camera_controller.process_mouse(delta);
                }
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        // Todo limit fps & physics step
        if let Some(scene) = &self.scene {
            scene.renderer.window.request_redraw();
        }
    }
}
