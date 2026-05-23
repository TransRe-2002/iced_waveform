const SHADER_SOURCE: &str = include_str!("shader.wgsl");

mod waveform;
mod primitive;
mod grid;
mod program;

pub(crate) use grid::nice_step;
pub use waveform::WaveformPipeline;
pub use primitive::WaveformPrimitive;
pub use program::WaveformProgram;
