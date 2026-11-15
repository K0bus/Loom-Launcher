pub mod adapter;
pub mod bethesda;
pub mod conflicts;
pub mod fomod;
pub mod nexus;
pub mod plugins;
pub mod providers;
pub mod registry;
pub mod symlink;
pub mod wow_adapter;

pub use adapter::{
    GameAdapter, GameModdingManifest, LaunchExecutable, ModFileInfo, ModVersionInfo, ProviderKind,
    RemoteDependency, UnifiedModSearchResult,
};
pub use bethesda::BethesdaAdapter;
pub use providers::curseforge::CurseForgeProvider;
pub use providers::modrinth::ModrinthProvider;
pub use providers::ModProvider;
pub use registry::{create_shared_registry, GameRegistry, SharedGameRegistry};
pub use wow_adapter::WowAdapter;
