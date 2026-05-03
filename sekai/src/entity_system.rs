use std::any::TypeId;

#[derive(Debug, Clone, PartialEq)]
pub struct EntitySystemInfo {
    pub name: String,
    pub updates_before: Vec<TypeId>,
    pub updates_after: Vec<TypeId>,
    pub updates_outside_prediction: bool,
}

impl EntitySystemInfo {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            updates_before: Vec::new(),
            updates_after: Vec::new(),
            updates_outside_prediction: false,
        }
    }
}

pub trait EntitySystem: Send {
    fn info(&self) -> &EntitySystemInfo;
    fn initialize(&mut self) {}
    fn shutdown(&mut self) {}
    fn update(&mut self, _frame_time: f32) {}
    fn frame_update(&mut self, _frame_time: f32) {}
}
