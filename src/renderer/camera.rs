use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
}

pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,

    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
    uniform: CameraUniform,
    buffer: wgpu::Buffer,
    
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl Camera {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        log::info!("Creating camera");

        let position = Vec3::new(0.0, 1.0, 2.0);
        let yaw = -90.0f32.to_radians();
        let pitch = 0.0;
        let aspect = width as f32 / height as f32;
        let fovy = 45.0;
        let znear = 0.1;
        let zfar = 100.0;

        let uniform = CameraUniform::new();

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
        });

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let mut camera = Self {
            position,
            yaw,
            pitch,
            aspect,
            fovy,
            znear,
            zfar,
            uniform,
            buffer,
            bind_group,
            bind_group_layout,
        };
        camera.update_uniform();
        camera
    }

    fn calc_view_matrix(&self) -> Mat4 {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();

        let dir = Vec3::new(
            cos_pitch * cos_yaw,
            sin_pitch,
            cos_pitch * sin_yaw,
        ).normalize();

        Mat4::look_to_rh(self.position, dir, Vec3::Y)
    }

    fn calc_proj_matrix(&self) -> Mat4 {
        Mat4::perspective_rh_gl(self.fovy.to_radians(), self.aspect, self.znear, self.zfar)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
        self.update_uniform();
    }

    pub fn update(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }

    pub fn update_uniform(&mut self) {
        let view = self.calc_view_matrix();
        let proj = self.calc_proj_matrix();
        self.uniform.view_proj = (proj * view).to_cols_array_2d();
    }
}