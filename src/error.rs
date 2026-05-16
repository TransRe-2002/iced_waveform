#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("wgpu error: {0}")]
    Wgpu(String),
    #[error("input data is empty")]
    EmptyData,
    #[error("internal error: {0}")]
    Internal(&'static str),
}
