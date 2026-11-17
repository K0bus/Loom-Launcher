//! Configuration du runner par jeu (runner, GameMode, MangoHud, variables d'environnement).
//! Stockée dans `~/.local/share/loom/native_game_config.json` (le Vue la garde dans son store).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub const RUNNERS: [&str; 4] = ["proton-ge", "proton-experimental", "wine-staging", "native"];
pub const RUNNER_LABELS: [&str; 4] = ["GE-Proton 9-20 (Recommandé)", "Proton Experimental", "Wine Staging (Système)", "Linux Natif"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameConfig {
    pub runner: String,
    pub gamemode: bool,
    pub mangohud: bool,
    pub env: String,
}

impl Default for GameConfig {
    fn default() -> Self {
        let s = crate::settings::load();
        Self { runner: crate::settings::RUNNERS[s.runner.min(3)].to_string(), gamemode: s.gamemode, mangohud: s.mangohud, env: String::new() }
    }
}

impl GameConfig {
    pub fn runner_index(&self) -> usize { RUNNERS.iter().position(|r| *r == self.runner).unwrap_or(0) }
    pub fn runner_label(&self) -> &'static str { RUNNER_LABELS[self.runner_index()] }

    /// Chaîne à coller dans « Options de lancement » de Steam (ou à utiliser comme préfixe de commande).
    pub fn launch_options(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        for kv in self.env.split_whitespace().filter(|kv| kv.contains('=')) { parts.push(kv.to_string()); }
        if self.mangohud && !parts.iter().any(|p| p.starts_with("MANGOHUD=")) { parts.push("MANGOHUD=1".into()); }
        if self.gamemode { parts.push("gamemoderun".into()); }
        if self.mangohud { parts.push("mangohud".into()); }
        parts.push("%command%".into());
        parts.join(" ")
    }
}

fn path() -> PathBuf {
    let d = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("loom");
    let _ = std::fs::create_dir_all(&d);
    d.join("native_game_config.json")
}

fn read_all() -> HashMap<String, GameConfig> {
    std::fs::read_to_string(path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn load(game_id: &str) -> GameConfig { read_all().remove(game_id).unwrap_or_default() }

pub fn save(game_id: &str, cfg: &GameConfig) -> Result<(), String> {
    let mut all = read_all();
    all.insert(game_id.to_string(), cfg.clone());
    std::fs::write(path(), serde_json::to_string_pretty(&all).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

/// Battle.net n'a qu'une configuration (préfixe Wine global) : on y reporte GameMode / MangoHud / env.
pub fn apply_to_battlenet(cfg: &GameConfig) -> Result<(), String> {
    let conn = crate::db::open_db().map_err(|e| e.to_string())?;
    let mut b = crate::db::get_battlenet_config(&conn).map_err(|e| e.to_string())?;
    b.enable_gamemode = cfg.gamemode;
    b.enable_mangohud = cfg.mangohud;
    b.custom_env_vars = if cfg.env.trim().is_empty() { None } else { Some(cfg.env.trim().to_string()) };
    crate::db::save_battlenet_config(&conn, &b).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_options_combine_env_gamemode_and_mangohud() {
        let c = GameConfig { runner: "proton-ge".into(), gamemode: true, mangohud: true, env: "VKD3D_CONFIG=dxr".into() };
        assert_eq!(c.launch_options(), "VKD3D_CONFIG=dxr MANGOHUD=1 gamemoderun mangohud %command%");
        let none = GameConfig { runner: "native".into(), gamemode: false, mangohud: false, env: String::new() };
        assert_eq!(none.launch_options(), "%command%");
        let only_env = GameConfig { env: "A=1 pas_une_variable B=2".into(), ..none.clone() };
        assert_eq!(only_env.launch_options(), "A=1 B=2 %command%");
    }

    #[test]
    fn mangohud_variable_is_not_duplicated() {
        let c = GameConfig { runner: "x".into(), gamemode: false, mangohud: true, env: "MANGOHUD=1".into() };
        assert_eq!(c.launch_options().matches("MANGOHUD=1").count(), 1);
    }

    #[test]
    fn runner_index_and_label_fall_back_to_the_first_entry() {
        let c = GameConfig { runner: "inconnu".into(), gamemode: false, mangohud: false, env: String::new() };
        assert_eq!(c.runner_index(), 0);
        assert!(c.runner_label().contains("GE-Proton"));
        assert_eq!(GameConfig { runner: "native".into(), ..c }.runner_index(), 3);
    }

    #[test]
    fn config_is_stored_per_game() {
        crate::settings::with_temp_xdg(|_| {
            let a = GameConfig { runner: "wine-staging".into(), gamemode: false, mangohud: true, env: "X=1".into() };
            save("steam-1", &a).unwrap();
            save("steam-2", &GameConfig { runner: "native".into(), ..a.clone() }).unwrap();
            assert_eq!(load("steam-1"), a);
            assert_eq!(load("steam-2").runner, "native");
            assert_eq!(load("steam-3"), GameConfig::default(), "jeu inconnu → valeurs par défaut des Paramètres");
        });
    }
}
