use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::db::{open_db, get_file_conflict_rules, get_profile_mods, ConflictingMod, FileConflict};

/// Scans the staging directory of all enabled mods in a profile and resolves conflicts
pub fn scan_and_resolve_conflicts(
    profile_id: &str,
    staging_base: &Path,
) -> Result<Vec<FileConflict>, String> {
    let conn = open_db().map_err(|e| e.to_string())?;
    let mods = get_profile_mods(&conn, profile_id).map_err(|e| e.to_string())?;
    let rules = get_file_conflict_rules(&conn, profile_id).map_err(|e| e.to_string())?;

    let rules_map: HashMap<String, String> = rules
        .into_iter()
        .map(|r| (r.relative_path, r.winning_mod_id))
        .collect();

    // Map: relative_path -> Vec<ConflictingMod>
    let mut file_providers: HashMap<String, Vec<ConflictingMod>> = HashMap::new();

    for pmod in &mods {
        if !pmod.is_enabled {
            continue;
        }

        // Determine mod staging path
        let mod_dir = if let Some(ref sp) = pmod.staged_path {
            PathBuf::from(sp)
        } else {
            staging_base.join(&pmod.id)
        };

        if !mod_dir.is_dir() {
            continue;
        }

        let files = collect_relative_files(&mod_dir)?;
        for rel_file in files {
            file_providers
                .entry(rel_file)
                .or_default()
                .push(ConflictingMod {
                    mod_id: pmod.id.clone(),
                    mod_name: pmod.name.clone(),
                    load_priority: pmod.load_priority,
                });
        }
    }

    let mut result = Vec::new();

    for (rel_path, mut claimants) in file_providers {
        // We only consider it a conflict if more than 1 mod provides the same file
        if claimants.len() > 1 {
            // Sort by priority descending: highest priority first
            claimants.sort_by(|a, b| b.load_priority.cmp(&a.load_priority));

            let winner_id = if let Some(explicit_winner) = rules_map.get(&rel_path) {
                if claimants.iter().any(|c| &c.mod_id == explicit_winner) {
                    explicit_winner.clone()
                } else {
                    claimants[0].mod_id.clone()
                }
            } else {
                claimants[0].mod_id.clone()
            };

            result.push(FileConflict {
                relative_path: rel_path,
                conflicting_mods: claimants,
                current_winner_mod_id: winner_id,
            });
        }
    }

    // Sort conflicts alphabetically by path
    result.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    Ok(result)
}

/// Recursively scans a directory and collects all relative file paths
pub fn collect_relative_files(root_dir: &Path) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    collect_files_recursive(root_dir, Path::new(""), &mut files)
        .map_err(|e| format!("Failed to collect files in {}: {}", root_dir.display(), e))?;
    Ok(files)
}

fn collect_files_recursive(
    base: &Path,
    current_rel: &Path,
    acc: &mut Vec<String>,
) -> std::io::Result<()> {
    let dir = base.join(current_rel);
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let file_name = entry.file_name();
        let next_rel = current_rel.join(file_name);

        if file_type.is_dir() {
            // Skip FOMOD meta folder from conflict detection
            let fname_str = next_rel.to_string_lossy().to_lowercase();
            if fname_str == "fomod" || fname_str.starts_with("fomod/") || fname_str.starts_with("fomod\\") {
                continue;
            }
            collect_files_recursive(base, &next_rel, acc)?;
        } else if file_type.is_file() || file_type.is_symlink() {
            acc.push(next_rel.to_string_lossy().to_string());
        }
    }

    Ok(())
}
