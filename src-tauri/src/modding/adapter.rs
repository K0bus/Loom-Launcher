use crate::db::ProfileModRecord;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Modrinth,
    CurseForge,
    Local,
    Nexus,
}

impl ProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderKind::Modrinth => "modrinth",
            ProviderKind::CurseForge => "curseforge",
            ProviderKind::Local => "local",
            ProviderKind::Nexus => "nexus",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "modrinth" => ProviderKind::Modrinth,
            "curseforge" => ProviderKind::CurseForge,
            "nexus" => ProviderKind::Nexus,
            _ => ProviderKind::Local,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchExecutable {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameModdingManifest {
    pub game_id: String,
    pub game_name: String,
    pub supported_providers: Vec<ProviderKind>,
    pub supports_load_order: bool,
    pub supports_conflict_resolution: bool,
    pub mods_directory: String,
    pub file_extensions: Vec<String>,
    pub executables: Vec<LaunchExecutable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedModSearchResult {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub summary: String,
    pub author: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub provider: ProviderKind,
    pub latest_version: Option<String>,
    pub categories: Vec<String>,
    pub loaders: Vec<String>,
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub project_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModFileInfo {
    pub id: String,
    pub filename: String,
    pub url: String,
    pub size_bytes: u64,
    pub sha1: Option<String>,
    pub sha512: Option<String>,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteDependency {
    pub remote_mod_id: String,
    pub remote_version_id: Option<String>,
    pub name: Option<String>,
    pub is_required: bool,
    pub dependency_type: String, // "required", "optional", "incompatible"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModVersionInfo {
    pub id: String,
    pub mod_id: String,
    pub name: String,
    pub version_number: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub files: Vec<ModFileInfo>,
    pub dependencies: Vec<RemoteDependency>,
}

pub trait GameAdapter: Send + Sync {
    /// Return the game's manifest (identifier, supported providers, load order, collision rules, executables)
    fn manifest(&self) -> GameModdingManifest;

    /// Returns the target mods folder for a given profile
    fn get_profile_mods_path(&self, profile_id: &str) -> PathBuf;

    /// Deploys symlinks from cache into the profile's mods directory for all enabled mods
    fn deploy_profile(&self, profile_id: &str, mods: &[ProfileModRecord]) -> Result<(), String>;

    /// Cleans up symlinks when changing or disabling profile mods
    fn undeploy_profile(&self, profile_id: &str) -> Result<(), String>;
}
