use crate::Component;
use std::collections::HashMap;
use std::sync::Arc;

pub type AppearanceValue = Arc<dyn std::any::Any + Send + Sync>;

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
        T: Clone + PartialEq + Send + Sync + 'static,
    {
        let key = key.into();
        if let Some(existing) = self.appearance_data.get(&key) {
            if let Some(existing) = existing.downcast_ref::<T>() {
                if *existing == value {
                    return;
                }
            }
        }

        self.appearance_data.insert(key, Arc::new(value));
        self.appearance_dirty = true;
    }

    pub fn get_data<T>(&self, key: &str) -> Option<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        self.appearance_data.get(key)?.downcast_ref::<T>().cloned()
    }

    pub fn clear_dirty(&mut self) {
        self.appearance_dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::AppearanceComponent;

    #[test]
    fn appearance_component_tracks_changes_and_reads_typed_values() {
        let mut component = AppearanceComponent::new();
        component.set_data("state", 5u32);
        assert!(component.appearance_dirty);
        assert_eq!(component.get_data::<u32>("state"), Some(5));
        component.clear_dirty();
        component.set_data("state", 5u32);
        assert!(!component.appearance_dirty);
    }
}
