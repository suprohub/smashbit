use glam::Vec3;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseScrollDelta},
    keyboard::KeyCode,
};

use std::f32::consts::FRAC_PI_2;
use std::time::Duration;

use crate::renderer::camera::Camera;

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

pub struct CameraController {
    movement: [f32; 3],
    rotation: [f32; 2],
    scroll: f32,
    speed: f32,
    sensitivity: f32,
}

impl Default for CameraController {
    fn default() -> Self {
        CameraController::new(4.0, 1.0)
    }
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            movement: [0.0; 3],
            rotation: [0.0; 2],
            scroll: 0.0,
            speed,
            sensitivity,
        }
    }

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        let value = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match key {
            KeyCode::KeyW => {
                self.movement[2] = value;
                true
            }
            KeyCode::KeyS => {
                self.movement[2] = -value;
                true
            }
            KeyCode::KeyA => {
                self.movement[0] = -value;
                true
            }
            KeyCode::KeyD => {
                self.movement[0] = value;
                true
            }
            KeyCode::Space => {
                self.movement[1] = value;
                true
            }
            KeyCode::ShiftLeft => {
                self.movement[1] = -value;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, delta: (f64, f64)) {
        self.rotation[0] = delta.0 as f32;
        self.rotation[1] = delta.1 as f32;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = match delta {
            MouseScrollDelta::LineDelta(_, y) => -y * 2.0,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => -*y as f32,
        };
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();

        // Move camera
        let (yaw_sin, yaw_cos) = camera.yaw.sin_cos();
        let forward = Vec3::new(yaw_cos, 0.0, yaw_sin);
        let right = Vec3::new(-yaw_sin, 0.0, yaw_cos);

        camera.position += forward * self.movement[2] * self.speed * dt;
        camera.position += right * self.movement[0] * self.speed * dt;
        camera.position.y += self.movement[1] * self.speed * dt;

        // Rotate camera
        camera.yaw += self.rotation[0] * self.sensitivity * dt;
        camera.pitch -= self.rotation[1] * self.sensitivity * dt;
        camera.pitch = camera.pitch.clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);

        // Handle zoom
        camera.position += forward * self.scroll * self.speed * dt;
        self.scroll = 0.0;

        // Reset rotation
        self.rotation = [0.0; 2];

        camera.update_uniform();
    }
}
