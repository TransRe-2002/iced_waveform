use crate::lod::LodPyramid;
use palette::Srgba;
use std::sync::Arc;
#[derive(Default)]
pub enum YAxis {
    #[default]
    Left,
    Right,
}

pub struct CurveItem {
    pub data: Arc<LodPyramid>,
    pub color: Srgba,
    pub y_axis: YAxis,
    pub name: String,
}

impl CurveItem {
    pub fn from_y(data: Vec<f32>) -> Result<Self, crate::error::Error> {
        let lod = LodPyramid::from_samples(&data)?;
        Ok(Self {
            data: Arc::new(lod),
            color: Srgba::new(0.3, 0.7, 0.4, 1.0),
            y_axis: YAxis::default(),
            name: String::new(),
        })
    }
}
