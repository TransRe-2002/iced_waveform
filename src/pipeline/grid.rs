use iced::Rectangle;
use palette::Srgba;

use crate::state::WaveformState;

pub fn nice_step(span: f64) -> f64 {
    let exponent = span.log10().floor();
    let fraction = span / 10_f64.powf(exponent);
    let nice = if fraction <= 1.5 {
        1.0
    } else if fraction <= 3.5 {
        2.0
    } else if fraction <= 7.5 {
        5.0
    } else {
        10.0
    };
    nice * 10_f64.powf(exponent)
}

pub fn generate_grid_lines(
    state: &WaveformState,
    left_ndc: f32,
    right_ndc: f32,
    top_ndc: f32,
    bottom_ndc: f32,
    bounds: Rectangle,
) -> Vec<[f32; 6]> {
    let grid_color = Srgba::new(0.3, 0.3, 0.3, 0.7);
    let axis_color = Srgba::new(0.7, 0.7, 0.7, 1.0);
    let mut vertices = Vec::new();

    let ndc_width = right_ndc - left_ndc;
    let ndc_height = top_ndc - bottom_ndc;

    let tick_px = 6.0;
    let tick_ndc_w = tick_px / bounds.width * ndc_width;
    let tick_ndc_h = tick_px / bounds.height * ndc_height;

    let x_span = (state.x_range.1 - state.x_range.0).max(1e-8);
    let x_step = nice_step(x_span / 8.0);
    let x_start = (state.x_range.0 / x_step).floor() as i64;
    let x_end = (state.x_range.1 / x_step).ceil() as i64;

    let y_span_left = (state.y_range_left.1 - state.y_range_left.0).max(1e-8);
    let y_step_left = nice_step(y_span_left / 8.0);
    let y_start_left = (state.y_range_left.0 / y_step_left).floor() as i64;
    let y_end_left = (state.y_range_left.1 / y_step_left).ceil() as i64;

    // 1. 网格线

    for i in x_start..=x_end {
        let data_x = i as f64 * x_step;
        let tx = ((data_x - state.x_range.0) / x_span) as f32;
        let ndc_x = left_ndc + tx * ndc_width;
        if ndc_x >= left_ndc && ndc_x <= right_ndc {
            vertices.push([ndc_x, bottom_ndc,
                grid_color.red, grid_color.green, grid_color.blue, grid_color.alpha]);
            vertices.push([ndc_x, top_ndc,
                grid_color.red, grid_color.green, grid_color.blue, grid_color.alpha]);
        }
    }

    for i in y_start_left..=y_end_left {
        let data_y = i as f64 * y_step_left;
        let ty = ((data_y - state.y_range_left.0) / y_span_left) as f32;
        let ndc_y = bottom_ndc + ty * ndc_height;
        if ndc_y >= bottom_ndc && ndc_y <= top_ndc {
            vertices.push([left_ndc, ndc_y,
                grid_color.red, grid_color.green, grid_color.blue, grid_color.alpha]);
            vertices.push([right_ndc, ndc_y,
                grid_color.red, grid_color.green, grid_color.blue, grid_color.alpha]);
        }
    }

    if let Some(right) = state.y_range_right {
        let y_span_right = (right.1 - right.0).max(1e-8);
        let y_step_right = nice_step(y_span_right / 8.0);
        let y_start_right = (right.0 / y_step_right).floor() as i64;
        let y_end_right = (right.1 / y_step_right).ceil() as i64;

        for i in y_start_right..=y_end_right {
            let data_y = i as f64 * y_step_right;
            let ty = ((data_y - right.0) / y_span_right) as f32;
            let ndc_y = bottom_ndc + ty * ndc_height;
            if ndc_y >= bottom_ndc && ndc_y <= top_ndc {
                vertices.push([left_ndc, ndc_y,
                    grid_color.red, grid_color.green, grid_color.blue, grid_color.alpha]);
                vertices.push([right_ndc, ndc_y,
                    grid_color.red, grid_color.green, grid_color.blue, grid_color.alpha]);
            }
        }
    }

    // 2. 数轴 + 刻度

    vertices.push([left_ndc, bottom_ndc,
        axis_color.red, axis_color.green, axis_color.blue, axis_color.alpha]);
    vertices.push([left_ndc, top_ndc,
        axis_color.red, axis_color.green, axis_color.blue, axis_color.alpha]);

    vertices.push([left_ndc, bottom_ndc,
        axis_color.red, axis_color.green, axis_color.blue, axis_color.alpha]);
    vertices.push([right_ndc, bottom_ndc,
        axis_color.red, axis_color.green, axis_color.blue, axis_color.alpha]);

    for i in x_start..=x_end {
        let data_x = i as f64 * x_step;
        let tx = ((data_x - state.x_range.0) / x_span) as f32;
        let ndc_x = left_ndc + tx * ndc_width;
        if ndc_x >= left_ndc && ndc_x <= right_ndc {
            vertices.push([ndc_x, bottom_ndc,
                axis_color.red, axis_color.green, axis_color.blue, axis_color.alpha]);
            vertices.push([ndc_x, bottom_ndc + tick_ndc_h,
                axis_color.red, axis_color.green, axis_color.blue, axis_color.alpha]);
        }
    }

    for i in y_start_left..=y_end_left {
        let data_y = i as f64 * y_step_left;
        let ty = ((data_y - state.y_range_left.0) / y_span_left) as f32;
        let ndc_y = bottom_ndc + ty * ndc_height;
        if ndc_y >= bottom_ndc && ndc_y <= top_ndc {
            vertices.push([left_ndc, ndc_y,
                axis_color.red, axis_color.green, axis_color.blue, axis_color.alpha]);
            vertices.push([left_ndc + tick_ndc_w, ndc_y,
                axis_color.red, axis_color.green, axis_color.blue, axis_color.alpha]);
        }
    }

    vertices
}
