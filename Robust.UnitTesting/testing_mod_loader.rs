use std::path::Path;
use robust_shared::content_pack::{Mod, ModLoader, ModLoaderInternal, ResourcePath};

pub struct TestingModLoader {
    pub mods: Vec<Box<dyn Mod>>,
}

impl TestingModLoader {
    pub fn new() -> Self {
        Self { mods: Vec::new() }
    }
}

impl ModLoader for TestingModLoader {
    fn try_load_modules_from(&mut self, _mount_path: &ResourcePath, _filter_prefix: &str) -> bool {
        for m in &self.mods {
            self.init_mod(m);
        }
        true
    }

    fn load_game_assembly(&mut self, _assembly: &[u8], _symbols: Option<&[u8]>, _skip_verify: bool) {
        panic!("Not supported in testing mod loader");
    }

    fn load_game_assembly_from_disk(&mut self, _disk_path: &Path, _skip_verify: bool) {
        panic!("Not supported in testing mod loader");
    }

    fn try_load_assembly(&mut self, _assembly_name: &str) -> bool {
        panic!("Not supported in testing mod loader");
    }

    fn set_use_load_context(&mut self, _use_load_context: bool) {}

    fn set_enable_sandboxing(&mut self, _sandboxing: bool) {}
}

impl ModLoaderInternal for TestingModLoader {
    fn init_mod(&mut self, m: &dyn Mod) {
        // Initialization logic for test mods
    }

    fn extra_module_loaders(&self) -> Option<&[Box<dyn Fn(&str) -> Option<Vec<u8>>>]> {
        None
    }
}
