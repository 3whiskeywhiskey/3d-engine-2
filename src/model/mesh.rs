pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material_index: usize,
}

impl Mesh {
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.num_elements, 0, 0..1);
    }

    pub fn clone_with_device(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        // Create new vertex buffer
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{} Vertex Buffer", self.name)),
            size: self.vertex_buffer.size(),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create new index buffer
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{} Index Buffer", self.name)),
            size: self.index_buffer.size(),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create command encoder for copying
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Buffer Copy Encoder"),
        });

        // Copy data from original buffers to new buffers
        encoder.copy_buffer_to_buffer(
            &self.vertex_buffer,
            0,
            &vertex_buffer,
            0,
            self.vertex_buffer.size(),
        );

        encoder.copy_buffer_to_buffer(
            &self.index_buffer,
            0,
            &index_buffer,
            0,
            self.index_buffer.size(),
        );

        // Submit copy commands
        queue.submit(std::iter::once(encoder.finish()));

        Self {
            name: self.name.clone(),
            vertex_buffer,
            index_buffer,
            num_elements: self.num_elements,
            material_index: self.material_index,
        }
    }
} 