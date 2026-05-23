use iced::{Point, Rectangle};
use iced::advanced::text;
use iced::alignment;

use crate::pipeline::nice_step;
// ============================================================
// 第零块：轴标签数据
// ============================================================

pub struct AxisLabel {
    pub text: String,
    pub position: Point,
    pub align_x: text::Alignment,
    pub align_y: alignment::Vertical,
}

fn format_tick(value: f64, step: f64) -> String {
    if step >= 1.0 && step.fract() < 1e-8 {
        format!("{:.0}", value)
    } else if step >= 0.1 {
        format!("{:.1}", value)
    } else if step >= 0.01 {
        format!("{:.2}", value)
    } else {
        format!("{:.3}", value)
    }
}

// ============================================================
// 第一块：跨帧状态 State
// ============================================================
pub struct WaveformState {
    /// 视口在widget内的对齐范围
    pub margins: Margins,
    /// 视口显示的数据范围 (x轴) —— 比如 (0.0, 1000.0) 表示显示第 0~1000 个采样点
    pub x_range: (f64, f64),
    /// 视口显示的数据范围 (y轴左侧) —— 比如 (-1.0, 1.0) 表示振幅范围
    pub y_range_left: (f64, f64),
    /// 视口显示的数据范围 (y轴右侧) —— 比如 (-1.0, 1.0) 表示振幅范围
    pub y_range_right: Option<(f64, f64)>,
    /// 当前交互状态
    pub interaction: Interaction,
    /// 上一帧鼠标位置，用于计算拖拽 delta
    pub last_mouse_pos: Option<Point>,
    pub cursor_data: Option<(f64, f64)>,
}

#[derive(Default)]
pub enum Interaction {
    #[default]
    Idle,
    Panning,
}

pub enum Axis {
    Both,
    X,
    Y,
}

impl WaveformState {
    pub fn new() -> Self {
        Self {
            margins: Margins::default(),
            x_range: (0.0, 1000.0),
            y_range_left: (-1.0, 1.0),
            y_range_right: None,
            interaction: Interaction::Idle,
            last_mouse_pos: None,
            cursor_data: None,
        }
    }

    /// 平移视口（数据坐标）
    pub fn pan(&mut self, dx_data: f64, dy_data: f64) {
        self.x_range.0 += dx_data;
        self.x_range.1 += dx_data;
        self.y_range_left.0 += dy_data;
        self.y_range_left.1 += dy_data;
        if let Some(ref mut right) = self.y_range_right {
            right.0 += dy_data;
            right.1 += dy_data;
        }
    }

    /// 缩放视口，以视口中心为锚点
    /// factor < 1.0 = 放大（范围缩小）
    /// factor > 1.0 = 缩小（范围扩大）
    pub fn zoom(&mut self, factor: f64) {
        let half_range_x = (self.x_range.1 - self.x_range.0) * factor / 2.0;
        let center_x = (self.x_range.0 + self.x_range.1) / 2.0;
        self.x_range.0 = center_x - half_range_x;
        self.x_range.1 = center_x + half_range_x;

        // y 轴同步缩放
        // y 轴左
        let half_range_y_left = (self.y_range_left.1 - self.y_range_left.0) * factor / 2.0;
        let center_y_left = (self.y_range_left.0 + self.y_range_left.1) / 2.0;
        self.y_range_left.0 = center_y_left - half_range_y_left;
        self.y_range_left.1 = center_y_left + half_range_y_left;

        if let Some(ref mut right) = self.y_range_right {
            let half_range_y_right = (right.1 - right.0) * factor / 2.0;
            let center_y_right = (right.0 + right.1) / 2.0;
            right.0 = center_y_right - half_range_y_right;
            right.1 = center_y_right + half_range_y_right;
        }
    }

    /// 像素坐标 → 数据坐标
    /// px: 鼠标像素位置，left: widget 左边缘像素，width: widget 像素宽度
    pub fn pixel_to_data(&self, px: f32, py: f32, bounds: Rectangle) -> (f64, f64) {
        let tx = (px - bounds.x) as f64 / bounds.width as f64; // 0.0 ~ 1.0
        let ty = 1.0 - (py - bounds.y) as f64 / bounds.height as f64;
        let x = self.x_range.0 + tx * (self.x_range.1 - self.x_range.0);
        let y = self.y_range_left.0 + ty * (self.y_range_left.1 - self.y_range_left.0);
        (x, y)
    }

    /// 像素坐标位移量 → 数据坐标位移量
    pub fn pixel_to_data_delta(
        &self,
        px_delta: f32,
        py_delta: f32,
        bounds: Rectangle,
    ) -> (f64, f64) {
        let data_range_x = self.x_range.1 - self.x_range.0;
        let data_range_y = self.y_range_left.1 - self.y_range_left.0;
        let dx = px_delta as f64 * data_range_x / bounds.width as f64;
        let dy = py_delta as f64 * data_range_y / bounds.height as f64;
        (dx, dy)
    }

    /// 根据当前视口范围生成轴刻度标签
    pub fn axis_labels(&self, bounds: Rectangle) -> Vec<AxisLabel> {
        let plot = self.margins.plot_bounds(bounds);
        let mut labels = Vec::new();

        // X 轴标签
        let x_span = (self.x_range.1 - self.x_range.0).max(1e-8);
        let x_step = nice_step(x_span / 8.0);
        let x_start = (self.x_range.0 / x_step).floor() as i64;
        let x_end = (self.x_range.1 / x_step).ceil() as i64;

        for i in x_start..=x_end {
            let data_x = i as f64 * x_step;
            let tx = ((data_x - self.x_range.0) / x_span) as f32;
            let x_px = plot.x + tx * plot.width;
            if x_px >= plot.x && x_px <= plot.x + plot.width {
                // Bottom margin area: axis line at bounds.height - margins.bottom,
                // labels sit in the blank space below the axis, near widget bottom.
                let rel_x = self.margins.left + tx * plot.width;
                let axis_y = bounds.height - self.margins.bottom;
                labels.push(AxisLabel {
                    text: format_tick(data_x, x_step),
                    position: Point::new(rel_x + 8.0, axis_y + 50.0),
                    align_x: text::Alignment::Center,
                    align_y: alignment::Vertical::Top,
                });
            }
        }

        // 左 Y 轴标签
        let y_span = (self.y_range_left.1 - self.y_range_left.0).max(1e-8);
        let y_step = nice_step(y_span / 8.0);
        let y_start = (self.y_range_left.0 / y_step).floor() as i64;
        let y_end = (self.y_range_left.1 / y_step).ceil() as i64;

        for i in y_start..=y_end {
            let data_y = i as f64 * y_step;
            let ty = ((data_y - self.y_range_left.0) / y_span) as f32;
            let y_px = plot.y + (1.0 - ty) * plot.height;
            if y_px >= plot.y && y_px <= plot.y + plot.height {
                let rel_y = self.margins.top + (1.0 - ty) * plot.height;
                let left_border = self.margins.left;
                labels.push(AxisLabel {
                    text: format_tick(data_y, y_step),
                    position: Point::new(left_border - 2.0, rel_y + 47.0),
                    align_x: text::Alignment::Right,
                    align_y: alignment::Vertical::Center,
                });
            }
        }

        // 右 Y 轴标签
        if let Some(right) = self.y_range_right {
            let y_span_r = (right.1 - right.0).max(1e-8);
            let y_step_r = nice_step(y_span_r / 8.0);
            let y_start_r = (right.0 / y_step_r).floor() as i64;
            let y_end_r = (right.1 / y_step_r).ceil() as i64;

            for i in y_start_r..=y_end_r {
                let data_y = i as f64 * y_step_r;
                let ty = ((data_y - right.0) / y_span_r) as f32;
                let y_px = plot.y + (1.0 - ty) * plot.height;
                if y_px >= plot.y && y_px <= plot.y + plot.height {
                    let rel_y = self.margins.top + (1.0 - ty) * plot.height;
                    let right_border = bounds.width - self.margins.right;
                    labels.push(AxisLabel {
                        text: format_tick(data_y, y_step_r),
                        position: Point::new(right_border + 2.0, rel_y + 47.0),
                        align_x: text::Alignment::Left,
                        align_y: alignment::Vertical::Center,
                    });
                }
            }
        }

        labels
    }
}

impl Default for WaveformState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 第二块：视口对齐状态 Margins
// ============================================================

#[derive(Clone, Copy, Debug)]
pub struct Margins {
    pub left: f32,   // 左 Y 轴区域
    pub right: f32,  // 右 Y 轴区域
    pub bottom: f32, // X 轴区域
    pub top: f32,    // 顶部留白
}

impl Default for Margins {
    fn default() -> Self {
        Self {
            left: 30.0,
            right: 20.0,
            bottom: 50.0,
            top: 30.0,
        }
    }
}

impl Margins {
    /// 从 widget bounds 算出实际的绘图区域
    pub fn plot_bounds(&self, bounds: Rectangle) -> Rectangle {
        Rectangle {
            x: bounds.x + self.left,
            y: bounds.y + self.top,
            width: bounds.width - self.left - self.right,
            height: bounds.height - self.top - self.bottom,
        }
    }
}