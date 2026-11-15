use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FomodOption {
    pub name: String,
    pub description: Option<String>,
    pub image_path: Option<String>,
    pub source_files: Vec<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FomodGroup {
    pub name: String,
    pub group_type: String, // "SelectExactlyOne", "SelectAtLeastOne", "SelectAny"
    pub options: Vec<FomodOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FomodStep {
    pub name: String,
    pub groups: Vec<FomodGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FomodConfig {
    pub module_name: String,
    pub module_image: Option<String>,
    pub has_installer: bool,
    pub required_files: Vec<String>,
    pub steps: Vec<FomodStep>,
}

/// Checks whether a given directory contains a FOMOD installer (`fomod/ModuleConfig.xml`)
pub fn find_fomod_dir(base_dir: &Path) -> Option<PathBuf> {
    if !base_dir.is_dir() {
        return None;
    }

    // Direct check: fomod or FOMOD
    let candidates = ["fomod", "FOMOD", "Fomod"];
    for cand in &candidates {
        let p = base_dir.join(cand);
        if p.is_dir() {
            let xml_candidates = ["ModuleConfig.xml", "moduleconfig.xml", "Moduleconfig.xml"];
            for xml in &xml_candidates {
                let xml_path = p.join(xml);
                if xml_path.is_file() {
                    return Some(p);
                }
            }
        }
    }

    // Check one level deep
    if let Ok(entries) = fs::read_dir(base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                for cand in &candidates {
                    let p = path.join(cand);
                    if p.is_dir() {
                        let xml_candidates = ["ModuleConfig.xml", "moduleconfig.xml", "Moduleconfig.xml"];
                        for xml in &xml_candidates {
                            if p.join(xml).is_file() {
                                return Some(p);
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Lightweight parser for FOMOD ModuleConfig.xml and info.xml
pub fn inspect_fomod(mod_path: &Path) -> Result<Option<FomodConfig>, String> {
    let fomod_dir = match find_fomod_dir(mod_path) {
        Some(d) => d,
        None => return Ok(None),
    };

    let xml_candidates = ["ModuleConfig.xml", "moduleconfig.xml", "Moduleconfig.xml"];
    let mut config_file = None;
    for cand in &xml_candidates {
        let p = fomod_dir.join(cand);
        if p.is_file() {
            config_file = Some(p);
            break;
        }
    }

    let config_path = match config_file {
        Some(p) => p,
        None => return Ok(None),
    };

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read FOMOD config: {}", e))?;

    // Extract module name
    let module_name = extract_tag_content(&content, "moduleName")
        .or_else(|| extract_tag_content(&content, "ModuleName"))
        .unwrap_or_else(|| {
            mod_path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown Mod".to_string())
        });

    let mut required_files = Vec::new();
    // Scan for <file source="..." /> or <folder source="..." />
    for line in content.lines() {
        let trimmed = line.trim();
        if (trimmed.starts_with("<file") || trimmed.starts_with("<folder")) && trimmed.contains("source=\"") {
            if let Some(src) = extract_attribute(trimmed, "source") {
                if !src.is_empty() {
                    required_files.push(src);
                }
            }
        }
    }

    let has_steps = content.contains("<installStep") || content.contains("<step");

    Ok(Some(FomodConfig {
        module_name,
        module_image: None,
        has_installer: has_steps,
        required_files,
        steps: Vec::new(),
    }))
}

fn extract_tag_content(xml: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{}>", tag);
    let close_tag = format!("</{}>", tag);
    if let Some(start) = xml.find(&open_tag) {
        let content_start = start + open_tag.len();
        if let Some(end) = xml[content_start..].find(&close_tag) {
            return Some(xml[content_start..content_start + end].trim().to_string());
        }
    }
    None
}

fn extract_attribute(line: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    if let Some(start) = line.find(&pattern) {
        let val_start = start + pattern.len();
        if let Some(end) = line[val_start..].find('"') {
            return Some(line[val_start..val_start + end].to_string());
        }
    }
    None
}
