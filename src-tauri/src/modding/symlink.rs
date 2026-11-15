use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Returns the shared central cache directory where downloaded mod binaries are stored:
/// `~/.local/share/loom/cache/mods/`
pub fn get_mods_cache_dir() -> PathBuf {
    let base = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
        .join("loom")
        .join("cache")
        .join("mods");
    let _ = fs::create_dir_all(&base);
    base
}

/// Atomically creates a symlink pointing `link_target` -> `source_path`.
///
/// Under Unix, creates a temporary symlink in the target's parent directory and renames it over `link_target`
/// to guarantee atomic replacement.
pub fn create_atomic_symlink(source_path: &Path, link_target: &Path) -> io::Result<()> {
    if let Some(parent) = link_target.parent() {
        fs::create_dir_all(parent)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let parent = link_target.parent().unwrap_or_else(|| Path::new("."));
        let temp_link = parent.join(format!(".tmp_symlink_{}", Uuid::new_v4()));

        // Create temporary symlink pointing to source
        symlink(source_path, &temp_link)?;

        // Rename atomically replaces link_target if it already exists
        if let Err(err) = fs::rename(&temp_link, link_target) {
            let _ = fs::remove_file(&temp_link);
            return Err(err);
        }
        Ok(())
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::symlink_file;

        if link_target.exists() || link_target.is_symlink() {
            let _ = fs::remove_file(link_target);
        }

        // On Windows without Developer Mode, symlinks may fail; fallback to hard link or copy
        if let Err(_) = symlink_file(source_path, link_target) {
            if let Err(_) = fs::hard_link(source_path, link_target) {
                fs::copy(source_path, link_target)?;
            }
        }
        Ok(())
    }

    #[cfg(not(any(unix, windows)))]
    {
        if link_target.exists() {
            let _ = fs::remove_file(link_target);
        }
        fs::copy(source_path, link_target)?;
        Ok(())
    }
}

/// Removes a symlink or file safely
pub fn remove_symlink(link_target: &Path) -> io::Result<()> {
    if link_target.is_symlink() || link_target.exists() {
        fs::remove_file(link_target)
    } else {
        Ok(())
    }
}

/// Cleans all symlinks in a directory (leaving actual non-symlink user files intact if desired,
/// or removing all managed mod symlinks)
pub fn clear_symlinks_in_dir(dir: &Path) -> io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_symlink() {
            let _ = fs::remove_file(&path);
        }
    }
    Ok(())
}
