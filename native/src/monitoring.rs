//! Monte le collecteur de métriques de `src-tauri` (indépendant de Tauri) sans le copier.
//! `commands.rs` (commandes Tauri) n'est pas monté ; `kill_process` est reproduit dans `mon.rs`.
#[path = "../../src-tauri/src/monitoring/models.rs"]
pub mod models;
#[path = "../../src-tauri/src/monitoring/collector.rs"]
pub mod collector;
pub use collector::*;
pub use models::*;
