#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ComponentMessage {
    pub remote: bool,
    pub directed: bool,
}

impl ComponentMessage {
    pub const fn new(remote: bool, directed: bool) -> Self {
        Self { remote, directed }
    }
}
