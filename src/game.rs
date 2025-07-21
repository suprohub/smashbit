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
    last_update: Instant,
    last_frame: Instant,
    target_fps: f32,
    target_physics_ps: f32,
    mouse_left: bool,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            scene: None,
            last_update: Instant::now(),
            last_frame: Instant::now(),
            target_fps: 1.0 / 60.0,
            target_physics_ps: 1.0 / 32.0,
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

                    if dt.as_secs_f32() >= self.target_fps {
                        scene.renderer.render().unwrap();
                        self.last_frame = now;
                    }
                }
                WindowEvent::Resized(new_size) => {
                    scene.renderer.resize(new_size);
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

                    if MouseButton::Left == button && state.is_pressed() {
                        scene.spawn_ball_instance(
                            scene.renderer.camera.position,
                            scene.renderer.camera.calc_view_dir(),
                            15.0,
                            0.5,
                        );
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
        if let Some(scene) = &mut self.scene {
            let now = Instant::now();
            let dt = now - self.last_update;
            self.last_update = now;

            scene
                .camera_controller
                .update_camera(&mut scene.renderer.camera, dt);
            scene
                .physics
                .step(dt.as_secs_f32(), self.target_physics_ps, 1.0, 1);
            scene.update_objects();
            scene.cull_instances_behind_camera();
            scene.renderer.window.request_redraw();
        }
    }
}
