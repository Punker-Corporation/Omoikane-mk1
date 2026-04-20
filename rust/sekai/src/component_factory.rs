use crate::Component;
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentAvailability {
    Available,
    Ignore,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentRegistration {
    pub name: String,
    pub net_id: Option<u16>,
    pub type_name: String,
    pub references: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownComponentError(pub String);

impl fmt::Display for UnknownComponentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for UnknownComponentError {}

pub struct ComponentFactory {
    names: HashMap<String, ComponentRegistration>,
    lower_case_names: HashMap<String, String>,
    ignored_component_names: HashSet<String>,
    constructors: HashMap<String, Box<dyn Fn() -> Component + Send + Sync>>,
    networked_components: Option<Vec<ComponentRegistration>>,
}

impl ComponentFactory {
    pub fn new() -> Self {
        Self {
            names: HashMap::new(),
            lower_case_names: HashMap::new(),
            ignored_component_names: HashSet::new(),
            constructors: HashMap::new(),
            networked_components: None,
        }
    }

    pub fn register(
        &mut self,
        name: impl Into<String>,
        type_name: impl Into<String>,
        constructor: impl Fn() -> Component + Send + Sync + 'static,
    ) {
        let name = name.into();
        let lower = name.to_ascii_lowercase();
        let registration = ComponentRegistration {
            name: name.clone(),
            net_id: None,
            type_name: type_name.into(),
            references: vec![name.clone()],
        };
        self.names.insert(name.clone(), registration);
        self.lower_case_names.insert(lower, name.clone());
        self.constructors.insert(name, Box::new(constructor));
    }

    pub fn register_reference(&mut self, target: &str, reference: impl Into<String>) -> Result<(), UnknownComponentError> {
        let Some(registration) = self.names.get_mut(target) else {
            return Err(UnknownComponentError(format!("Unknown type: {target}")));
        };
        let reference = reference.into();
        if !registration.references.contains(&reference) {
            registration.references.push(reference);
        }
        Ok(())
    }

    pub fn register_ignore(&mut self, name: impl Into<String>) {
        self.ignored_component_names.insert(name.into());
    }

    pub fn get_component_availability(&self, component_name: &str, ignore_case: bool) -> ComponentAvailability {
        let key = if ignore_case {
            self.lower_case_names.get(&component_name.to_ascii_lowercase()).map(String::as_str).unwrap_or(component_name)
        } else {
            component_name
        };
        if self.names.contains_key(key) {
            ComponentAvailability::Available
        } else if self.ignored_component_names.contains(key) {
            ComponentAvailability::Ignore
        } else {
            ComponentAvailability::Unknown
        }
    }

    pub fn get_component(&self, component_name: &str, ignore_case: bool) -> Result<Component, UnknownComponentError> {
        let key = if ignore_case {
            self.lower_case_names
                .get(&component_name.to_ascii_lowercase())
                .map(String::as_str)
                .unwrap_or(component_name)
        } else {
            component_name
        };
        let Some(constructor) = self.constructors.get(key) else {
            return Err(UnknownComponentError(format!("Unknown name: {component_name}")));
        };
        Ok(constructor())
    }

    pub fn get_registration(&self, component_name: &str, ignore_case: bool) -> Result<&ComponentRegistration, UnknownComponentError> {
        let key = if ignore_case {
            self.lower_case_names
                .get(&component_name.to_ascii_lowercase())
                .map(String::as_str)
                .unwrap_or(component_name)
        } else {
            component_name
        };
        self.names.get(key).ok_or_else(|| UnknownComponentError(format!("Unknown name: {component_name}")))
    }

    pub fn try_get_registration(&self, component_name: &str, ignore_case: bool) -> Option<&ComponentRegistration> {
        self.get_registration(component_name, ignore_case).ok()
    }

    pub fn generate_net_ids(&mut self, networked_names: &[&str]) {
        let mut regs = networked_names
            .iter()
            .filter_map(|name| self.names.get(*name).cloned())
            .collect::<Vec<_>>();
        regs.sort_by(|a, b| a.name.cmp(&b.name));
        for (i, reg) in regs.iter_mut().enumerate() {
            reg.net_id = Some(i as u16);
            if let Some(existing) = self.names.get_mut(&reg.name) {
                existing.net_id = reg.net_id;
            }
        }
        self.networked_components = Some(regs);
    }

    pub fn networked_components(&self) -> Option<&[ComponentRegistration]> {
        self.networked_components.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::{ComponentAvailability, ComponentFactory};
    use crate::Component;

    #[test]
    fn component_factory_registers_lookups_and_net_ids() {
        let mut factory = ComponentFactory::new();
        factory.register("Transform", "TransformComponent", || Component::new("Transform"));
        factory.register_reference("Transform", "ITransform").unwrap();
        assert_eq!(factory.get_component_availability("transform", true), ComponentAvailability::Available);
        assert_eq!(factory.get_component("Transform", false).unwrap().name, "Transform");
        factory.generate_net_ids(&["Transform"]);
        assert_eq!(factory.networked_components().unwrap()[0].net_id, Some(0));
    }
}
