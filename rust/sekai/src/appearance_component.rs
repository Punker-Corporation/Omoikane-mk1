use crate::Component;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AppearanceValue {
    Bool(bool),
    Int(i32),
    UInt(u32),
    Float(f32),
    Text(String),
}

pub trait IntoAppearanceValue {
    fn into_appearance_value(self) -> AppearanceValue;
}

pub trait FromAppearanceValue: Sized {
    fn from_appearance_value(value: &AppearanceValue) -> Option<Self>;
}

impl IntoAppearanceValue for bool {
    fn into_appearance_value(self) -> AppearanceValue {
        AppearanceValue::Bool(self)
    }
}

impl FromAppearanceValue for bool {
    fn from_appearance_value(value: &AppearanceValue) -> Option<Self> {
        match value {
            AppearanceValue::Bool(value) => Some(*value),
            _ => None,
        }
    }
}

impl IntoAppearanceValue for i32 {
    fn into_appearance_value(self) -> AppearanceValue {
        AppearanceValue::Int(self)
    }
}

impl FromAppearanceValue for i32 {
    fn from_appearance_value(value: &AppearanceValue) -> Option<Self> {
        match value {
            AppearanceValue::Int(value) => Some(*value),
            _ => None,
        }
    }
}

impl IntoAppearanceValue for u32 {
    fn into_appearance_value(self) -> AppearanceValue {
        AppearanceValue::UInt(self)
    }
}

impl FromAppearanceValue for u32 {
    fn from_appearance_value(value: &AppearanceValue) -> Option<Self> {
        match value {
            AppearanceValue::UInt(value) => Some(*value),
            _ => None,
        }
    }
}

impl IntoAppearanceValue for u8 {
    fn into_appearance_value(self) -> AppearanceValue {
        AppearanceValue::UInt(self as u32)
    }
}

impl FromAppearanceValue for u8 {
    fn from_appearance_value(value: &AppearanceValue) -> Option<Self> {
        match value {
            AppearanceValue::UInt(value) => u8::try_from(*value).ok(),
            _ => None,
        }
    }
}

impl IntoAppearanceValue for f32 {
    fn into_appearance_value(self) -> AppearanceValue {
        AppearanceValue::Float(self)
    }
}

impl FromAppearanceValue for f32 {
    fn from_appearance_value(value: &AppearanceValue) -> Option<Self> {
        match value {
            AppearanceValue::Float(value) => Some(*value),
            _ => None,
        }
    }
}

impl IntoAppearanceValue for String {
    fn into_appearance_value(self) -> AppearanceValue {
        AppearanceValue::Text(self)
    }
}

impl IntoAppearanceValue for &str {
    fn into_appearance_value(self) -> AppearanceValue {
        AppearanceValue::Text(self.to_string())
    }
}

impl FromAppearanceValue for String {
    fn from_appearance_value(value: &AppearanceValue) -> Option<Self> {
        match value {
            AppearanceValue::Text(value) => Some(value.clone()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AppearanceComponentState {
    pub data: HashMap<String, AppearanceValue>,
}

#[derive(Debug, Clone)]
pub struct AppearanceComponent {
    pub base: Component,
    pub appearance_dirty: bool,
    pub appearance_data: HashMap<String, AppearanceValue>,
}

impl AppearanceComponent {
    pub fn new() -> Self {
        Self {
            base: Component::new("Appearance"),
            appearance_dirty: false,
            appearance_data: HashMap::new(),
        }
    }

    pub fn set_data<T>(&mut self, key: impl Into<String>, value: T)
    where
        T: IntoAppearanceValue,
    {
        let key = key.into();
        let value = value.into_appearance_value();
        if self.appearance_data.get(&key) == Some(&value) {
            return;
        }

        self.appearance_data.insert(key, value);
        self.appearance_dirty = true;
    }

    pub fn get_data<T>(&self, key: &str) -> Option<T>
    where
        T: FromAppearanceValue,
    {
        T::from_appearance_value(self.appearance_data.get(key)?)
    }

    pub fn get_component_state(&self) -> AppearanceComponentState {
        AppearanceComponentState {
            data: self.appearance_data.clone(),
        }
    }

    pub fn handle_component_state(&mut self, state: AppearanceComponentState) {
        self.appearance_data = state.data;
        self.appearance_dirty = true;
    }

    pub fn clear_dirty(&mut self) {
        self.appearance_dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::{AppearanceComponent, AppearanceComponentState, AppearanceValue};
    use std::collections::HashMap;

    #[test]
    fn appearance_component_tracks_changes_and_reads_typed_values() {
        let mut component = AppearanceComponent::new();
        component.set_data("state", 5u32);
        component.set_data("name", "omoikane");
        assert!(component.appearance_dirty);
        assert_eq!(component.get_data::<u32>("state"), Some(5));
        assert_eq!(component.get_data::<String>("name").as_deref(), Some("omoikane"));
        component.clear_dirty();
        component.set_data("state", 5u32);
        assert!(!component.appearance_dirty);
        component.handle_component_state(AppearanceComponentState {
            data: HashMap::from([(String::from("enabled"), AppearanceValue::Bool(true))]),
        });
        assert_eq!(component.get_data::<bool>("enabled"), Some(true));
    }
}
