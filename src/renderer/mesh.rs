use crate::renderer::pipeline::InstanceRaw;

pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub instance_buffer: wgpu::Buffer,
    pub instance_capacity: u32,
    pub instances: Vec<InstanceRaw>,
}

impl Mesh {
    pub fn add_instance(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instance: &InstanceRaw,
    ) {
        if self.instances.len() as u32 >= self.instance_capacity {
            let new_capacity = (self.instance_capacity * 2).max(1);
            self.resize_instance_buffer(device, queue, new_capacity);
        }

        self.instances.push(*instance);
        let offset = ((self.instances.len() - 1) * std::mem::size_of::<InstanceRaw>())
            as wgpu::BufferAddress;
        queue.write_buffer(&self.instance_buffer, offset, bytemuck::bytes_of(instance));
    }

    pub fn remove_instance(
        &mut self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        instance_index: usize,
    ) {
        assert!(
            instance_index < self.instances.len(),
            "instance_index out of bounds"
        );

        let last_index = self.instances.len() - 1;
        if instance_index == last_index {
            self.instances.pop();
            return;
        }

        let last_instance = self.instances[last_index];
        self.instances[instance_index] = last_instance;
        self.instances.pop();

        let offset = (instance_index * std::mem::size_of::<InstanceRaw>()) as wgpu::BufferAddress;
        queue.write_buffer(
            &self.instance_buffer,
            offset,
            bytemuck::bytes_of(&last_instance),
        );
    }

    pub fn remove_instances_batch(
        &mut self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        indices: &[usize],
    ) {
        let new_instances = self
            .instances
            .iter()
            .enumerate()
            .filter(|(i, _)| !indices.contains(i))
            .map(|(_, inst)| *inst)
            .collect::<Vec<_>>();

        if !new_instances.is_empty() {
            queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&new_instances),
            );
        }

        self.instances = new_instances;
    }

    pub fn update_instance(
        &mut self,
        queue: &wgpu::Queue,
        instance_index: usize,
        new_instance: &InstanceRaw,
    ) {
        if instance_index < self.instances.len() {
            self.instances[instance_index] = *new_instance;
            let offset =
                (instance_index * std::mem::size_of::<InstanceRaw>()) as wgpu::BufferAddress;
            queue.write_buffer(
                &self.instance_buffer,
                offset,
                bytemuck::bytes_of(new_instance),
            );
        }
    }

    pub fn update_all_instances(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instances: &[InstanceRaw],
    ) {
        let new_count = instances.len() as u32;
        if new_count > self.instance_capacity {
            self.resize_instance_buffer(device, queue, new_count);
        }

        if !instances.is_empty() {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instances));
        }

        self.instances = instances.to_vec();
    }

    fn resize_instance_buffer(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        new_capacity: u32,
    ) {
        let new_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Resized Instance Buffer"),
            size: (new_capacity as usize * std::mem::size_of::<InstanceRaw>())
                as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        if !self.instances.is_empty() {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Instance Buffer Copy Encoder"),
            });

            encoder.copy_buffer_to_buffer(
                &self.instance_buffer,
                0,
                &new_buffer,
                0,
                (self.instances.len() * std::mem::size_of::<InstanceRaw>()) as wgpu::BufferAddress,
            );

            queue.submit(std::iter::once(encoder.finish()));
        }

        self.instance_buffer = new_buffer;
        self.instance_capacity = new_capacity;
    }
}
