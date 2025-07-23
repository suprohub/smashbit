use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FogUniform {
    pub lower_color: [f32; 4],
    pub upper_color: [f32; 4],
    pub density: f32,
    pub start: f32,
    _padding: [f32; 2],
}

impl Default for FogUniform {
    fn default() -> Self {
        Self {
            lower_color: [1.0, 0.294, 0.361, 1.0],
            upper_color: [1.0, 0.765, 0.443, 1.0],
            density: 0.05,
            start: 5.0,
            _padding: [0.0; 2],
        }
    }
}

pub struct Fog {
    pub uniform: FogUniform,
    pub buffer: wgpu::Buffer,
    pub bind_layout_entry: wgpu::BindGroupLayoutEntry,
}

impl Fog {
    pub fn new(device: &wgpu::Device) -> Self {
        let uniform = FogUniform::default();

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Fog Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            uniform,
            buffer,
            bind_layout_entry: wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        }
    }
}
