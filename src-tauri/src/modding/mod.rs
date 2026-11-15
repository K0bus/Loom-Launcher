pub mod adapter;
pub mod providers;
pub mod registry;
pub mod symlink;

pub use adapter::{
    GameAdapter, GameModdingManifest, LaunchExecutable, ModFileInfo, ModVersionInfo, ProviderKind,
    RemoteDependency, UnifiedModSearchResult,
};
pub use providers::curseforge::CurseForgeProvider;
pub use providers::modrinth::ModrinthProvider;
pub use providers::ModProvider;
pub use registry::{create_shared_registry, GameRegistry, SharedGameRegistry};
