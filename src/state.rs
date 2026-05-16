use iced::{Point, Rectangle};
// ============================================================
// 第一块：跨帧状态 State
// ============================================================
pub struct WaveformState {
    /// 视口显示的数据范围 (x轴) —— 比如 (0.0, 1000.0) 表示显示第 0~1000 个采样点
    pub x_range: (f64, f64),
    /// 视口显示的数据范围 (y轴) —— 比如 (-1.0, 1.0) 表示振幅范围
    pub y_range: (f64, f64),
    /// 当前交互状态
    pub interaction: Interaction,
    /// 上一帧鼠标位置，用于计算拖拽 delta
    pub last_mouse_pos: Option<Point>,
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
            x_range: (0.0, 1000.0),
            y_range: (-1.0, 1.0),
            interaction: Interaction::Idle,
            last_mouse_pos: None,
        }
    }

    /// 平移视口（数据坐标）
    pub fn pan(&mut self, dx_data: f64, dy_data: f64) {
        self.x_range.0 += dx_data;
        self.x_range.1 += dx_data;
        self.y_range.0 += dy_data;
        self.y_range.1 += dy_data;
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
        let half_range_y = (self.y_range.1 - self.y_range.0) * factor / 2.0;
        let center_y = (self.y_range.0 + self.y_range.1) / 2.0;
        self.y_range.0 = center_y - half_range_y;
        self.y_range.1 = center_y + half_range_y;
    }

    /// 像素坐标 → 数据坐标
    /// px: 鼠标像素位置，left: widget 左边缘像素，width: widget 像素宽度
    pub fn pixel_to_data(&self, px: f32, py: f32, bounds: Rectangle) -> (f64, f64) {
        let tx = (px - bounds.x) as f64 / bounds.width as f64; // 0.0 ~ 1.0
        let ty = 1.0 - (py - bounds.y) as f64 / bounds.height as f64;
        let x = self.x_range.0 + tx * (self.x_range.1 - self.x_range.0);
        let y = self.y_range.0 + ty * (self.y_range.1 - self.y_range.0);
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
        let data_range_y = self.y_range.1 - self.y_range.0;
        let dx = px_delta as f64 * data_range_x / bounds.width as f64;
        let dy = py_delta as f64 * data_range_y / bounds.height as f64;
        (dx, dy)
    }
}

impl Default for WaveformState {
    fn default() -> Self {
        Self::new()
    }
}
