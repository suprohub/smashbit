use std::{sync::Arc, time::Instant};
use winit::{application::ApplicationHandler, event::{DeviceEvent, MouseButton, WindowEvent}, keyboard::PhysicalKey, window::WindowAttributes};
use crate::{camera_controller::CameraController, renderer::Renderer};

pub struct Game {
    renderer: Option<Renderer>,
    last_frame: Instant,
    camera_controller: CameraController,
    mouse_left: bool,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            renderer: None,
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
        self.renderer = Some(pollster::block_on(Renderer::new(window)).unwrap());
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
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
                    self.last_frame  = now;

                    renderer.window.request_redraw();

                    renderer.render().unwrap();
                    self.camera_controller.update_camera(&mut renderer.camera, dt);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    self.camera_controller.process_keyboard(keycode, event.state);
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
            event_loop: &winit::event_loop::ActiveEventLoop,
            device_id: winit::event::DeviceId,
            event: winit::event::DeviceEvent,
        ) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                if self.mouse_left {
                    self.camera_controller.process_mouse(delta);
                }
            },
            _ => {}
        }
    }
}
