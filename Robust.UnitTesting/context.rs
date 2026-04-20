use std::collections::HashSet;
use robust_shared::configuration::{ConfigurationManager, ConfigurationManagerInternal};
use robust_shared::content_pack::ResourcePath;
use robust_shared::entity::{ComponentFactory, EntityManager};
use robust_shared::map::MapManager;
use robust_shared::physics::components::{
    BroadphaseComponent, FixturesComponent, PhysicsMapComponent,
};
use robust_shared::prototypes::PrototypeManager;
use robust_shared::reflection::ReflectionManager;
use robust_shared::system::{EntitySystemManager, TransformSystem, EntityLookupSystem};
use robust_server::container::ContainerSystem;
use robust_server::game_objects::MetaDataComponent;
use robust_server::physics::EntityLookupComponent;
use crate::mod_loader::testing_mod_loader::TestingModLoader;

pub enum UnitTestProject {
    Server,
    Client,
}

pub struct RobustTestContext {
    pub config_manager: ConfigurationManager,
    pub entity_manager: EntityManager,
    pub map_manager: MapManager,
    pub system_manager: EntitySystemManager,
    pub component_factory: ComponentFactory,
    pub reflection_manager: ReflectionManager,
    pub prototype_manager: PrototypeManager,
    pub mod_loader: TestingModLoader,
}

impl RobustTestContext {
    pub fn new(project: UnitTestProject) -> Self {
        let mut config_manager = ConfigurationManager::new();
        config_manager.initialize(matches!(project, UnitTestProject::Server));

        let assemblies = match project {
            UnitTestProject::Client => vec!["Robust.Client"],
            UnitTestProject::Server => vec!["Robust.Server"],
        };

        let shared_assemblies = vec!["Robust.Shared", "Robust.UnitTesting"];
        let content_assemblies = Self::get_content_assemblies();

        let all_assemblies: HashSet<_> = assemblies
            .iter()
            .chain(shared_assemblies.iter())
            .chain(content_assemblies.iter())
            .cloned()
            .collect();

        for assembly in &all_assemblies {
            config_manager.load_cvars_from_assembly(assembly);
        }

        let mut system_manager = EntitySystemManager::new();
        system_manager.load_extra_system::<ContainerSystem>();
        system_manager.load_extra_system::<TransformSystem>();
        system_manager.load_extra_system::<EntityLookupSystem>();

        let mut entity_manager = EntityManager::new();
        let mut map_manager = MapManager::new();

        entity_manager.initialize();
        map_manager.initialize();
        system_manager.initialize();

        let mut reflection_manager = ReflectionManager::new();
        reflection_manager.load_assemblies(all_assemblies.into_iter().collect());

        let mut mod_loader = TestingModLoader::new();
        mod_loader.try_load_modules_from(&ResourcePath::root(), "");

        let mut component_factory = ComponentFactory::new();
        if !component_factory.all_registered_types().contains(&"MetaDataComponent") {
            component_factory.register_class::<MetaDataComponent>();
        }
        if !component_factory.all_registered_types().contains(&"EntityLookupComponent") {
            component_factory.register_class::<EntityLookupComponent>();
        }
        if !component_factory.all_registered_types().contains(&"SharedPhysicsMapComponent") {
            component_factory.register_class::<PhysicsMapComponent>();
        }
        if !component_factory.all_registered_types().contains(&"BroadphaseComponent") {
            component_factory.register_class::<BroadphaseComponent>();
        }
        if !component_factory.all_registered_types().contains(&"FixturesComponent") {
            component_factory.register_class::<FixturesComponent>();
        }

        entity_manager.startup();
        map_manager.startup();

        Self {
            config_manager,
            entity_manager,
            map_manager,
            system_manager,
            component_factory,
            reflection_manager,
            prototype_manager: PrototypeManager::new(),
            mod_loader,
        }
    }

    fn get_content_assemblies() -> Vec<&'static str> {
        vec![]
    }
}

impl Drop for RobustTestContext {
    fn drop(&mut self) {
        // Limpeza equivalente a `IoCManager.Clear()`.
        // Em Rust, os recursos são liberados automaticamente quando o struct é descartado.
        // Se houver necessidade de shutdown manual, pode ser implementado aqui.
    }
}
