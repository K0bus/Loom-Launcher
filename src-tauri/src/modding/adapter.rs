use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderKind {
    #[serde(alias = "modrinth")]
    Modrinth,
    #[serde(alias = "curseforge")]
    CurseForge,
    #[serde(alias = "nexus", alias = "nexusmods")]
    NexusMods,
    #[serde(alias = "steam", alias = "steamworkshop")]
    SteamWorkshop,
    #[serde(alias = "wago")]
    Wago,
    #[serde(alias = "local", alias = "localarchive")]
    LocalArchive,
}

impl ProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderKind::Modrinth => "modrinth",
            ProviderKind::CurseForge => "curseforge",
            ProviderKind::NexusMods => "nexusmods",
            ProviderKind::SteamWorkshop => "steamworkshop",
            ProviderKind::Wago => "wago",
            ProviderKind::LocalArchive => "local",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().replace('_', "").replace('-', "").as_str() {
            "modrinth" => ProviderKind::Modrinth,
            "curseforge" => ProviderKind::CurseForge,
            "nexus" | "nexusmods" => ProviderKind::NexusMods,
            "steam" | "steamworkshop" => ProviderKind::SteamWorkshop,
            "wago" => ProviderKind::Wago,
            _ => ProviderKind::LocalArchive,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchExecutable {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub label: String,
    pub relative_path: PathBuf,
    pub is_default: bool,
    pub requires_steamworks: bool,
}

impl LaunchExecutable {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        relative_path: impl Into<PathBuf>,
        is_default: bool,
        requires_steamworks: bool,
    ) -> Self {
        let n = name.into();
        Self {
            id: id.into(),
            label: n.clone(),
            name: n,
            relative_path: relative_path.into(),
            is_default,
            requires_steamworks,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameModdingManifest {
    pub game_id: String,
    pub display_name: String,
    #[serde(default)]
    pub game_name: String,
    pub supported_providers: Vec<ProviderKind>,
    pub allows_load_order: bool,
    #[serde(default)]
    pub supports_load_order: bool,
    pub allows_conflict_override: bool,
    #[serde(default)]
    pub supports_conflict_resolution: bool,
    pub supports_modpacks: bool,
    #[serde(default)]
    pub mods_directory: String,
    #[serde(default)]
    pub file_extensions: Vec<String>,
    pub executables: Vec<LaunchExecutable>,
}

impl GameModdingManifest {
    pub fn new(
        game_id: impl Into<String>,
        display_name: impl Into<String>,
        supported_providers: Vec<ProviderKind>,
        allows_load_order: bool,
        allows_conflict_override: bool,
        supports_modpacks: bool,
        executables: Vec<LaunchExecutable>,
    ) -> Self {
        let gid = game_id.into();
        let dn = display_name.into();
        Self {
            game_id: gid,
            display_name: dn.clone(),
            game_name: dn,
            supported_providers,
            allows_load_order,
            supports_load_order: allows_load_order,
            allows_conflict_override,
            supports_conflict_resolution: allows_conflict_override,
            supports_modpacks,
            mods_directory: "mods".to_string(),
            file_extensions: vec![".jar".to_string(), ".zip".to_string()],
            executables,
        }
    }
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
    fn manifest(&self) -> GameModdingManifest;

    /// Répertoire où les archives sont décompressées avant activation
    fn get_staging_directory(&self, profile_id: &str) -> PathBuf;

    /// Répertoire cible réel du jeu (ex: Data/ ou Interface/AddOns/)
    fn get_deployment_directory(&self, profile_id: &str) -> PathBuf;

    /// Normalise le contenu d'une archive téléchargée selon la structure du jeu
    fn normalize_staged_mod(&self, mod_staged_path: &PathBuf) -> Result<(), String>;

    /// Déploie l'ensemble des symlinks du profil actif vers le dossier cible
    fn deploy_profile(&self, profile_id: &str) -> Result<(), String>;

    /// Nettoie les symlinks déployés sans altérer les fichiers vanilla
    fn undeploy_profile(&self, profile_id: &str) -> Result<(), String>;
}
