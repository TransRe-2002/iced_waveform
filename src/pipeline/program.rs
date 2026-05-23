use iced::{Event, Rectangle};
use iced::mouse::{self, ScrollDelta};
use iced::widget::shader;
use palette::Srgba;

use crate::curve::{CurveItem, YAxis};
use crate::state::{Interaction, WaveformState};

use super::primitive::WaveformPrimitive;
use super::grid::generate_grid_lines;

pub struct WaveformProgram {
    curves: Vec<CurveItem>,
}

impl WaveformProgram {
    pub fn new(curves: Vec<CurveItem>) -> Self {
        Self { curves }
    }
}

impl<Message> shader::Program<Message> for WaveformProgram {
    type State = WaveformState;
    type Primitive = WaveformPrimitive;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Option<shader::Action<Message>> {
        let Some(cursor_pos) = cursor.position() else {
            state.interaction = Interaction::Idle;
            state.last_mouse_pos = None;
            state.cursor_data = None;
            return None;
        };

        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let dy = match delta {
                    ScrollDelta::Lines { y, .. } => *y as f64,
                    ScrollDelta::Pixels { y, .. } => *y as f64,
                };
                state.zoom(1.05_f64.powf(-dy));
                return Some(shader::Action::request_redraw());
            }

            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                state.interaction = Interaction::Panning;
                state.last_mouse_pos = Some(cursor_pos);
            }

            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Interaction::Panning = state.interaction {
                    if let Some(last) = state.last_mouse_pos {
                        let dx_px = cursor_pos.x - last.x;
                        let dy_px = cursor_pos.y - last.y;
                        let plot = state.margins.plot_bounds(bounds);
                        let (dx_data, dy_data) = state.pixel_to_data_delta(dx_px, dy_px, plot);
                        state.pan(-dx_data, dy_data);
                    }
                }
                state.last_mouse_pos = Some(cursor_pos);

                // 十字准线 + overlay 的命中检测
                let plot = state.margins.plot_bounds(bounds);
                // Only check for hits within the plot area — cursor in margins
                // would map to out-of-range data coords and cause false positives.
                let in_plot = cursor_pos.x >= plot.x
                    && cursor_pos.x <= plot.x + plot.width
                    && cursor_pos.y >= plot.y
                    && cursor_pos.y <= plot.y + plot.height;
                if in_plot {
                    let (cx, cy) = state.pixel_to_data(cursor_pos.x, cursor_pos.y, plot);
                    let idx = cx as usize;
                    let margin = (state.y_range_left.1 - state.y_range_left.0) * 0.015;
                    let hit = self.curves.iter().any(|curve| {
                        // Reject out-of-bounds: negative cx saturates to 0 as
                        // usize, and idx beyond data len is always invalid.
                        if cx < 0.0 || idx >= curve.data.len() {
                            return false;
                        }
                        let mm = curve.data.query(idx, (idx + 1).min(curve.data.len()), 1);
                        !mm.is_empty() && cy >= mm[0].min as f64 - margin && cy <= mm[0].max as f64 + margin
                    });
                    state.cursor_data = if hit { Some((cx, cy)) } else { None };
                } else {
                    state.cursor_data = None;
                }

                return Some(shader::Action::request_redraw());
            }

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.interaction = Interaction::Idle;
                state.last_mouse_pos = Some(cursor_pos);
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
    ) -> Self::Primitive {
        let plot = state.margins.plot_bounds(bounds);
        let left_ndc = -1.0 + 2.0 * state.margins.left / bounds.width;
        let right_ndc = 1.0 - 2.0 * state.margins.right / bounds.width;
        let top_ndc = 1.0 - 2.0 * state.margins.top / bounds.height;
        let bottom_ndc = -1.0 + 2.0 * state.margins.bottom / bounds.height;

        if self.curves.is_empty() || plot.width as usize == 0 {
            return WaveformPrimitive { vertices: vec![], margins: state.margins };
        }
        let mut vertices: Vec<[f32; 6]> = Vec::new();
        let grid_lines = generate_grid_lines(state, left_ndc, right_ndc, top_ndc, bottom_ndc, bounds);
        vertices.extend(grid_lines);
        for curve in &self.curves {
            let x_span = (state.x_range.1 - state.x_range.0).max(1e-8);
            let data_len = curve.data.len() as f64;

            let data_px_start = if state.x_range.0 >= 0.0 {
                0.0
            } else {
                (-state.x_range.0 / x_span * plot.width as f64) as f32
            };
            let data_px_end = if state.x_range.1 <= data_len {
                plot.width
            } else {
                ((data_len - state.x_range.0) / x_span * plot.width as f64) as f32
            };

            let visible_cols: usize = (data_px_end - data_px_start).max(0.0) as usize;
            if visible_cols == 0 {
                continue;
            }

            let start = state.x_range.0.max(0.0) as usize;
            let end = (state.x_range.1.min(data_len) as usize).max(start + 1);
            let columns = curve.data.query(start, end, visible_cols);

            let col_width = visible_cols as f32 / columns.len() as f32;
            let y_range = match curve.y_axis {
                YAxis::Left => state.y_range_left,
                YAxis::Right => state.y_range_right.unwrap_or(state.y_range_left),
            };
            let y_min = y_range.0;
            let y_span = (y_range.1 - y_range.0).max(1e-8);

            let mut prev: Option<[f32; 6]> = None;
            for (i, col) in columns.iter().enumerate() {
                let px = data_px_start + i as f32 * col_width + col_width * 0.5;
                let x_ndc = left_ndc + (px / plot.width) * (right_ndc - left_ndc);
                let y_ndc = bottom_ndc + ((col.max as f64 - y_min) / y_span) as f32 * (top_ndc - bottom_ndc);
                if y_ndc.is_finite() {
                    let color = &curve.color;
                    let curr = [x_ndc, y_ndc, color.red, color.green, color.blue, color.alpha];
                    if let Some(p) = prev {
                        vertices.push(p);
                        vertices.push(curr);
                    }
                    prev = Some(curr);
                } else {
                    prev = None;
                }
            }
        }

        // 十字准线
        if let Some((cx, cy)) = state.cursor_data {
            let crosshair_color = Srgba::new(0.4, 0.8, 1.0, 0.6);

            let x_span = (state.x_range.1 - state.x_range.0).max(1e-8);
            let y_span = (state.y_range_left.1 - state.y_range_left.0).max(1e-8);

            let cx_ndc = left_ndc + ((cx - state.x_range.0) / x_span) as f32 * (right_ndc - left_ndc);
            let cy_ndc = bottom_ndc + ((cy - state.y_range_left.0) / y_span) as f32 * (top_ndc - bottom_ndc);

            vertices.push([cx_ndc, bottom_ndc, crosshair_color.red, crosshair_color.green, crosshair_color.blue, crosshair_color.alpha]);
            vertices.push([cx_ndc, top_ndc, crosshair_color.red, crosshair_color.green, crosshair_color.blue, crosshair_color.alpha]);
            vertices.push([left_ndc, cy_ndc, crosshair_color.red, crosshair_color.green, crosshair_color.blue, crosshair_color.alpha]);
            vertices.push([right_ndc, cy_ndc, crosshair_color.red, crosshair_color.green, crosshair_color.blue, crosshair_color.alpha]);
        }

        WaveformPrimitive { vertices, margins: state.margins }
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        mouse::Interaction::default()
    }
}
