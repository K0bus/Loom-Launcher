use flate2::read::GzDecoder;
use reqwest::Client;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::Command;
use tar::Archive;

pub fn get_runtimes_base_dir() -> PathBuf {
    let base = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/share"))
        .join("loom")
        .join("runtimes");
    let _ = fs::create_dir_all(&base);
    base
}

/// Returns the expected binary path for a given Java major version (e.g. 8, 17, 21)
pub fn get_java_binary_path(major_version: u32) -> PathBuf {
    get_runtimes_base_dir()
        .join(format!("java-{}", major_version))
        .join("bin")
        .join("java")
}

/// Checks if an existing java binary is valid by testing `java -version`
pub fn validate_java_binary(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(path, perms);
        }
    }

    let status = Command::new(path).arg("-version").output();
    match status {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

/// Finds any available system Java matching or compatible with the requested major version
pub fn find_system_java(major_version: u32) -> Option<PathBuf> {
    // Check SDKMAN
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let sdkman_path = PathBuf::from(&home).join(".sdkman/candidates/java/current/bin/java");
    if validate_java_binary(&sdkman_path) {
        return Some(sdkman_path);
    }

    // Check standard system paths
    let candidates = [
        format!("/usr/lib/jvm/java-{}-openjdk/bin/java", major_version),
        format!("/usr/lib64/jvm/java-{}-openjdk/bin/java", major_version),
        format!("/usr/lib/jvm/jre-{}/bin/java", major_version),
        "/usr/bin/java".to_string(),
    ];

    for c in candidates {
        let p = PathBuf::from(c);
        if validate_java_binary(&p) {
            return Some(p);
        }
    }

    None
}

/// Autonomous Java provisioning manager:
/// Detects, downloads, and unpacks Eclipse Adoptium (Temurin) JRE for the target major version.
/// Guarantees that Minecraft can run without requiring the host OS to have Java installed.
pub async fn ensure_java_runtime(client: &Client, major_version: u32) -> Result<PathBuf, String> {
    let target_java = get_java_binary_path(major_version);
    if validate_java_binary(&target_java) {
        return Ok(target_java);
    }

    let runtime_dir = get_runtimes_base_dir().join(format!("java-{}", major_version));
    let _ = fs::create_dir_all(&runtime_dir);

    println!(
        "[Loom::Runtime] Provisioning autonomous Java {} for Linux x64...",
        major_version
    );

    // Adoptium Temurin latest release API
    let adoptium_url = format!(
        "https://api.adoptium.net/v3/binary/latest/{}/ga/linux/x64/jdk/hotspot/normal/eclipse",
        major_version
    );

    let resp = client
        .get(&adoptium_url)
        .send()
        .await
        .map_err(|e| format!("Failed to initiate Adoptium download for Java {}: {}", major_version, e))?;

    if !resp.status().is_success() {
        // Fallback to system Java if available
        if let Some(sys_java) = find_system_java(major_version) {
            println!(
                "[Loom::Runtime] Adoptium download returned {}, using system fallback: {}",
                resp.status(),
                sys_java.display()
            );
            return Ok(sys_java);
        }
        return Err(format!(
            "Adoptium API error status {} downloading Java {}",
            resp.status(),
            major_version
        ));
    }

    let temp_archive_path = runtime_dir.join(format!("temurin_jdk_{}.tar.gz", major_version));

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Error downloading JDK bytes: {}", e))?;

    fs::write(&temp_archive_path, &bytes)
        .map_err(|e| format!("Error saving JDK archive: {}", e))?;

    // Extract tar.gz into temporary unpack directory
    let unpack_dir = runtime_dir.join("unpack");
    let _ = fs::create_dir_all(&unpack_dir);

    let tar_file = File::open(&temp_archive_path)
        .map_err(|e| format!("Failed to open downloaded archive: {}", e))?;
    let tar = GzDecoder::new(tar_file);
    let mut archive = Archive::new(tar);

    archive
        .unpack(&unpack_dir)
        .map_err(|e| format!("Failed to extract JDK archive: {}", e))?;

    // Find extracted root folder containing bin/java
    let mut found_java_bin = None;
    if let Ok(entries) = fs::read_dir(&unpack_dir) {
        for entry in entries.flatten() {
            let candidate_bin = entry.path().join("bin").join("java");
            if candidate_bin.exists() {
                found_java_bin = Some(entry.path());
                break;
            }
        }
    }

    if let Some(jdk_root) = found_java_bin {
        // Move contents from jdk_root to runtime_dir
        if let Ok(entries) = fs::read_dir(&jdk_root) {
            for entry in entries.flatten() {
                let dest = runtime_dir.join(entry.file_name());
                let _ = fs::rename(entry.path(), dest);
            }
        }
    }

    // Clean up temporary files
    let _ = fs::remove_file(&temp_archive_path);
    let _ = fs::remove_dir_all(&unpack_dir);

    // Validate the provisioned binary
    if validate_java_binary(&target_java) {
        println!(
            "[Loom::Runtime] Java {} provisioned successfully at {}",
            major_version,
            target_java.display()
        );
        Ok(target_java)
    } else if let Some(sys) = find_system_java(major_version) {
        Ok(sys)
    } else {
        Err(format!(
            "Java {} provisioned but validation failed at {}",
            major_version,
            target_java.display()
        ))
    }
}
