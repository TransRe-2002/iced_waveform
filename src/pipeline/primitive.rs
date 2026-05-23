use iced::Rectangle;
use iced::wgpu;
use iced::widget::shader::{self, Primitive};
use bytemuck;

use super::waveform::WaveformPipeline;

#[derive(Debug)]
pub struct WaveformPrimitive {
    pub vertices: Vec<[f32; 6]>,
    pub margins: crate::state::Margins,
}

impl Primitive for WaveformPrimitive {
    type Pipeline = WaveformPipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _bounds: &Rectangle,
        _viewport: &shader::Viewport,
    ) {
        if self.vertices.is_empty() {
            return;
        }

        let data = bytemuck::cast_slice(&self.vertices);
        if data.len() as u64 > pipeline.vertex_buffer.size() {
            pipeline.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("waveform vertex buffer"),
                size: data.len() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&pipeline.vertex_buffer, 0, data);

        let scale = _viewport.scale_factor() as f32;
        let left = ((_bounds.x + self.margins.left) * scale).floor() as u32;
        let top = ((_bounds.y + self.margins.top) * scale).floor() as u32;
        let right = ((_bounds.x + _bounds.width - self.margins.right) * scale).ceil() as u32 + 1;
        let bottom = ((_bounds.y + _bounds.height - self.margins.bottom) * scale).ceil() as u32 + 1;
        let w = right.saturating_sub(left).max(1);
        let h = bottom.saturating_sub(top).max(1);
        pipeline.scissor_rect = Some((left, top, w, h));
    }

    fn draw(&self, pipeline: &Self::Pipeline, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        if self.vertices.is_empty() {
            return true;
        }
        if let Some(r) = pipeline.scissor_rect {
            render_pass.set_scissor_rect(r.0, r.1, r.2, r.3);
        }
        render_pass.set_pipeline(&pipeline.render_pipeline);
        render_pass.set_vertex_buffer(0, pipeline.vertex_buffer.slice(..));
        render_pass.draw(0..self.vertices.len() as u32, 0..1);
        true
    }
}
