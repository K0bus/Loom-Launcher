use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use super::adapter::{GameAdapter, GameModdingManifest};

#[derive(Default)]
pub struct GameRegistry {
    adapters: HashMap<String, Arc<dyn GameAdapter + Send + Sync>>,
}

impl GameRegistry {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    pub fn register(&mut self, adapter: Arc<dyn GameAdapter + Send + Sync>) {
        let manifest = adapter.manifest();
        self.adapters.insert(manifest.game_id, adapter);
    }

    pub fn get(&self, game_id: &str) -> Option<Arc<dyn GameAdapter + Send + Sync>> {
        self.adapters.get(game_id).cloned()
    }

    pub fn list_manifests(&self) -> Vec<GameModdingManifest> {
        self.adapters.values().map(|a| a.manifest()).collect()
    }
}

pub type SharedGameRegistry = Arc<RwLock<GameRegistry>>;

pub fn create_shared_registry() -> SharedGameRegistry {
    Arc::new(RwLock::new(GameRegistry::new()))
}
