//! Monte les modules Battle.net de `src-tauri` sans les copier (aucune duplication, aucune
//! modification du cœur métier). `mod.rs` (commandes Tauri) et `wow.rs` ne sont pas montés.
#[path = "../../src-tauri/src/battlenet/auth.rs"]
pub mod auth;
#[path = "../../src-tauri/src/battlenet/detector.rs"]
pub mod detector;
#[path = "../../src-tauri/src/battlenet/launcher.rs"]
pub mod launcher;
#[path = "../../src-tauri/src/battlenet/wow.rs"]
pub mod wow;
