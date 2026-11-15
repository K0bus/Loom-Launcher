use loom_launcher_lib::modding::adapter::ProviderKind;
use loom_launcher_lib::modding::bethesda::BethesdaAdapter;
use loom_launcher_lib::modding::nexus::parse_nxm_uri;
use loom_launcher_lib::modding::plugins::{format_plugins_txt, PluginType};
use loom_launcher_lib::modding::wow_adapter::WowAdapter;
use loom_launcher_lib::modding::GameAdapter;

#[test]
fn test_nxm_uri_parsing_valid() {
    let uri = "nxm://skyrimspecialedition/mods/2014/files/8542?key=abcdef123456&expires=1700000000&user_id=12345";
    let parsed = parse_nxm_uri(uri).expect("Should parse valid nxm uri");

    assert_eq!(parsed.game_domain, "skyrimspecialedition");
    assert_eq!(parsed.mod_id, 2014);
    assert_eq!(parsed.file_id, 8542);
    assert_eq!(parsed.key.as_deref(), Some("abcdef123456"));
    assert_eq!(parsed.expires, Some(1700000000));
    assert_eq!(parsed.user_id, Some(12345));
}

#[test]
fn test_nxm_uri_parsing_invalid() {
    assert!(parse_nxm_uri("http://example.com").is_err());
    assert!(parse_nxm_uri("nxm://invalid_structure").is_err());
}

#[test]
fn test_plugin_tier_sorting_and_formatting() {
    assert_eq!(PluginType::from_filename("Skyrim.esm").unwrap(), PluginType::Master);
    assert_eq!(PluginType::from_filename("Update.esm").unwrap().priority_tier(), 0);
    assert_eq!(PluginType::from_filename("LightPatch.esl").unwrap(), PluginType::LightMaster);
    assert_eq!(PluginType::from_filename("LightPatch.esl").unwrap().priority_tier(), 1);
    assert_eq!(PluginType::from_filename("SkyUI_SE.esp").unwrap(), PluginType::Standard);
    assert_eq!(PluginType::from_filename("SkyUI_SE.esp").unwrap().priority_tier(), 2);

    let dummy_plugins = vec![
        loom_launcher_lib::db::BethesdaPluginRecord {
            id: "p1".to_string(),
            profile_id: "test".to_string(),
            mod_id: None,
            filename: "SkyUI_SE.esp".to_string(),
            load_order_index: 3,
            is_active: true,
        },
        loom_launcher_lib::db::BethesdaPluginRecord {
            id: "p2".to_string(),
            profile_id: "test".to_string(),
            mod_id: None,
            filename: "Unofficial Skyrim Special Edition Patch.esp".to_string(),
            load_order_index: 2,
            is_active: true,
        },
        loom_launcher_lib::db::BethesdaPluginRecord {
            id: "p3".to_string(),
            profile_id: "test".to_string(),
            mod_id: None,
            filename: "Skyrim.esm".to_string(),
            load_order_index: 0,
            is_active: true,
        },
        loom_launcher_lib::db::BethesdaPluginRecord {
            id: "p4".to_string(),
            profile_id: "test".to_string(),
            mod_id: None,
            filename: "DisabledMod.esp".to_string(),
            load_order_index: 4,
            is_active: false,
        },
    ];

    let formatted = format_plugins_txt(&dummy_plugins);
    let lines: Vec<&str> = formatted.lines().collect();

    assert_eq!(lines[0], "*Skyrim.esm");
    assert_eq!(lines[1], "*Unofficial Skyrim Special Edition Patch.esp");
    assert_eq!(lines[2], "*SkyUI_SE.esp");
    assert_eq!(lines[3], "DisabledMod.esp");
}

#[test]
fn test_bethesda_adapter_manifest() {
    let adapter = BethesdaAdapter::new_skyrim_se();
    let manifest = adapter.manifest();

    assert_eq!(manifest.game_id, "skyrim_se");
    assert!(manifest.allows_load_order);
    assert!(manifest.allows_conflict_override);
    assert!(manifest.supported_providers.contains(&ProviderKind::NexusMods));

    let skse_exec = manifest.executables.iter().find(|e| e.id == "skse").unwrap();
    assert!(skse_exec.is_default);
    assert!(skse_exec.requires_steamworks);
}

#[test]
fn test_wow_adapter_manifest() {
    let adapter = WowAdapter::new("_retail_");
    let manifest = adapter.manifest();

    assert_eq!(manifest.game_id, "wow");
    assert!(!manifest.allows_load_order);
    assert!(!manifest.allows_conflict_override);
    assert!(manifest.supported_providers.contains(&ProviderKind::Wago));
}
