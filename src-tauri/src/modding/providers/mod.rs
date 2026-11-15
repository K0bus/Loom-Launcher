pub mod curseforge;
pub mod modrinth;

use std::path::{Path, PathBuf};
use async_trait::async_trait;

use super::adapter::{ModVersionInfo, ProviderKind, RemoteDependency, UnifiedModSearchResult};

#[async_trait]
pub trait ModProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;

    async fn search(
        &self,
        query: &str,
        game_id: &str,
        game_version: Option<&str>,
        loader: Option<&str>,
        limit: usize,
    ) -> Result<Vec<UnifiedModSearchResult>, String>;

    async fn get_versions(
        &self,
        remote_mod_id: &str,
        game_version: Option<&str>,
        loader: Option<&str>,
    ) -> Result<Vec<ModVersionInfo>, String>;

    async fn resolve_dependencies(
        &self,
        remote_mod_id: &str,
        remote_version_id: &str,
    ) -> Result<Vec<RemoteDependency>, String>;

    async fn download_file(
        &self,
        url: &str,
        destination_dir: &Path,
        expected_filename: &str,
    ) -> Result<PathBuf, String>;
}
