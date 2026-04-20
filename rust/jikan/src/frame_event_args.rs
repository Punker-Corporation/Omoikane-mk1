#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameEventArgs {
    pub delta_seconds: f32,
}

impl FrameEventArgs {
    pub const fn new(delta_seconds: f32) -> Self {
        Self { delta_seconds }
    }
}
