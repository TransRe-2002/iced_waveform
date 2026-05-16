use iced::Event;
use iced::Rectangle;
use iced::wgpu::{self as wgpu};
use iced::widget::shader::{self, Pipeline, Primitive};
use iced::mouse::{self, ScrollDelta};
use bytemuck;
use std::sync::Arc;

use crate::state::Interaction;
use crate::lod::LodPyramid;
// ============================================================
// WGSL 着色器
// ============================================================
const SHADER_SOURCE: &str = include_str!("shader.wgsl");

pub struct WaveformPipeline {
    render_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
}

impl Pipeline for WaveformPipeline {
    fn new(
        device: &iced::wgpu::Device,
        _queue: &iced::wgpu::Queue,
        format: iced::wgpu::TextureFormat,
    ) -> Self
    where
        Self: Sized,
    {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("waveform shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });

        // Uniform buffer: 16 bytes (vec4<f32> color)
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("waveform shader"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("waveform bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("waveform bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(uniform_buffer.as_entire_buffer_binding()),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("waveform pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("waveform pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 2 * 4, // vec2<f32>
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // 初始顶点缓冲区
        let vertex_capacity = 4096;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("waveform vertex buffer"),
            size: (vertex_capacity * 2 * 4) as u64, // capacity 顶点 × vec2 × f32
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            render_pipeline,
            uniform_buffer,
            bind_group,
            vertex_buffer,
        }
    }

    fn trim(&mut self) {}
}

// ============================================================
// Primitive — 每帧创建，携带 LineList 顶点数据
// ============================================================
#[derive(Debug)]
pub struct WaveformPrimitive {
    vertices: Vec<[f32; 2]>, // NDC 坐标对
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

        // 写入 uniform 颜色
        let color: [f32; 4] = [0.3, 0.7, 0.4, 1.0];
        queue.write_buffer(&pipeline.uniform_buffer, 0, bytemuck::cast_slice(&color));

        // 写入顶点缓冲区（不够就扩容）
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
    }

    fn draw(&self, pipeline: &Self::Pipeline, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        if self.vertices.is_empty() {
            return true;
        }
        render_pass.set_pipeline(&pipeline.render_pipeline);
        render_pass.set_bind_group(0, &pipeline.bind_group, &[]);
        render_pass.set_vertex_buffer(0, pipeline.vertex_buffer.slice(..));
        render_pass.draw(0..self.vertices.len() as u32, 0..1);
        true
    }
}

// ============================================================
// Program — 逻辑层，替代原来的 Widget trait
// ============================================================
pub struct WaveformProgram {
    data: Arc<LodPyramid>,
}

impl WaveformProgram {
    pub fn new(data: Arc<LodPyramid>) -> Self {
        Self { data }
    }
}

impl<Message> shader::Program<Message> for WaveformProgram {
    type State = crate::state::WaveformState;
    type Primitive = WaveformPrimitive;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Option<shader::Action<Message>> {
        let Some(cursor_pos) = cursor.position() else {
            // 光标离开 widget 时，强制结束拖拽
            state.interaction = Interaction::Idle;
            state.last_mouse_pos = None;
            return None;
        };

        match event {
            // ---- 滚轮缩放 ----
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let dy = match delta {
                    ScrollDelta::Lines { y, .. } => *y as f64,
                    ScrollDelta::Pixels { y, .. } => *y as f64,
                };
                state.zoom(1.05_f64.powf(-dy));
                return Some(shader::Action::request_redraw());
            }

            // ---- 左键按下：开始拖拽 ----
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                state.interaction = Interaction::Panning;
                state.last_mouse_pos = Some(cursor_pos);
            }

            // ---- 鼠标移动：拖拽中 ----
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Interaction::Panning = state.interaction {
                    if let Some(last) = state.last_mouse_pos {
                        let dx_px = cursor_pos.x - last.x;
                        let dy_px = cursor_pos.y - last.y;
                        let (dx_data, dy_data) = state.pixel_to_data_delta(dx_px, dy_px, bounds);
                        state.pan(-dx_data, dy_data);
                        state.last_mouse_pos = Some(cursor_pos);
                        return Some(shader::Action::request_redraw());
                    }
                }
            }

            // ---- 左键释放：结束拖拽 ----
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.interaction = Interaction::Idle;
                state.last_mouse_pos = None;
            }

            _ => {}
        }
        
        None
    }

    fn draw(
        &self,
        state: &Self::State,
        _cursor: iced::mouse::Cursor,
        bounds: Rectangle,
    ) -> Self::Primitive
    {
        let screen_width = bounds.width as usize;
        if screen_width == 0 {
            return WaveformPrimitive { vertices: vec![] };
        }

        let x_span = (state.x_range.1 - state.x_range.0).max(1e-8);
        let data_len = self.data.len() as f64;

        // 有效数据在屏幕上的像素列范围
        let data_px_start = if state.x_range.0 >= 0.0 {
            0.0
        } else {
            (-state.x_range.0 / x_span * bounds.width as f64) as f32
        };
        let data_px_end = if state.x_range.1 <= data_len {
            bounds.width
        } else {
            ((data_len - state.x_range.0) / x_span * bounds.width as f64) as f32
        };

        let visible_cols: usize = (data_px_end - data_px_start).max(0.0) as usize;
        if visible_cols == 0 {
            return WaveformPrimitive { vertices: vec![] };
        }

        let start = state.x_range.0.max(0.0) as usize;
        let end = (state.x_range.1.min(data_len) as usize).max(start + 1);
        let columns = self.data.query(start, end, visible_cols);

        let col_width = visible_cols as f32 / columns.len() as f32;
        let y_min = state.y_range.0;
        let y_span = (state.y_range.1 - state.y_range.0).max(1e-8);

        let mut vertices: Vec<[f32; 2]> = Vec::with_capacity(columns.len());
        let mut prev: Option<[f32; 2]> = None;

        for (i, col) in columns.iter().enumerate() {
            let px = data_px_start + i as f32 * col_width + col_width * 0.5;
            let x_ndc = (px / bounds.width) * 2.0 - 1.0;
            let y_ndc = ((col.max as f64 - y_min) / y_span) as f32 * 2.0 - 1.0;

            if y_ndc.is_finite() {
                let curr = [x_ndc, y_ndc];
                if let Some(p) = prev {
                    vertices.push(p);
                    vertices.push(curr);
                }
                prev = Some(curr);
            } else {
                prev = None;
            }
        }
        WaveformPrimitive { vertices }
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction
    {
        mouse::Interaction::default()
    }
}
