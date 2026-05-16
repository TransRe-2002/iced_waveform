use std::sync::Arc;
use iced::widget::shader::Shader;
use iced::{Element, Length};

use crate::pipeline::WaveformProgram;
use crate::lod::LodPyramid;

pub struct WaveformWidget<Message> {
    shader: Shader<Message, WaveformProgram>,
}

impl <Message> WaveformWidget<Message> {
    pub fn from_y(data: Vec<f32>) -> Result<Self, crate::error::Error> {
        let lod = LodPyramid::from_samples(&data)?;
        Ok(Self {
            shader: Shader::new(WaveformProgram::new(
                Arc::new(lod)
            ))
                .width(Length::Fill)
                .height(Length::Fill),
        })
    }

    pub fn from_xy(_x: Vec<f32>, y: Vec<f32>) -> Result<Self, crate::error::Error> {
        // todo!();
        Self::from_y(y)
    }
}

impl <'a, Message: 'a> From<WaveformWidget<Message>> for Element<'a, Message> {
    fn from(widget: WaveformWidget<Message>) -> Self {
        widget.shader.into()
    }
}
