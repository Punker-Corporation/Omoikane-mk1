use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use crate::context::UnitTestProject;
use crate::mod_loader::testing_mod_loader::TestingModLoader;
use robust_client::ClientIoC;
use robust_server::ServerIoC;
use robust_shared::content_pack::{ModLoader, ModLoaderInternal};
use robust_shared::game_controller::DisplayMode;

pub struct IoCManager {
    singletons: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    factories: HashMap<TypeId, Box<dyn Fn(&mut Self) -> Arc<dyn Any + Send + Sync>>>,
}

impl IoCManager {
    pub fn new() -> Self {
        Self {
            singletons: HashMap::new(),
            factories: HashMap::new(),
        }
    }

    pub fn register<TInterface: 'static + Send + Sync, TImplementation: 'static + Send + Sync>(
        &mut self,
        factory: impl Fn(&mut Self) -> Arc<TImplementation> + 'static,
    ) {
        let type_id = TypeId::of::<TInterface>();
        let factory = Box::new(move |container: &mut Self| {
            let instance = factory(container);
            Arc::new(instance) as Arc<dyn Any + Send + Sync>
        });
        self.factories.insert(type_id, factory);
    }

    pub fn register_instance<TInterface: 'static + Send + Sync>(
        &mut self,
        instance: Arc<TInterface>,
    ) {
        let type_id = TypeId::of::<TInterface>();
        self.singletons.insert(type_id, instance);
    }

    pub fn resolve<T: 'static + Send + Sync>(&mut self) -> Arc<T> {
        let type_id = TypeId::of::<T>();
        if let Some(instance) = self.singletons.get(&type_id) {
            return instance.clone().downcast::<T>().unwrap();
        }
        if let Some(factory) = self.factories.remove(&type_id) {
            let instance = factory(self);
            let typed = instance.clone().downcast::<T>().unwrap();
            self.singletons.insert(type_id, instance);
            return typed;
        }
        panic!("Type not registered: {:?}", std::any::type_name::<T>());
    }

    pub fn build_graph(&mut self) {}
}

pub fn register_ioc(project: UnitTestProject) -> IoCManager {
    let mut container = IoCManager::new();

    match project {
        UnitTestProject::Client => {
            ClientIoC::register_ioc(&mut container, DisplayMode::Headless);
        }
        UnitTestProject::Server => {
            ServerIoC::register_ioc(&mut container);
        }
    }

    let mod_loader = Arc::new(TestingModLoader::new());
    container.register_instance::<dyn ModLoader>(mod_loader.clone());
    container.register_instance::<dyn ModLoaderInternal>(mod_loader.clone());
    container.register_instance::<TestingModLoader>(mod_loader);

    override_ioc(&mut container);

    container.build_graph();
    container
}

fn override_ioc(_container: &mut IoCManager) {}
