// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    {
        // 0. Clean up any ghost or outdated instances BEFORE initializing Tauri & single_instance
        loom_launcher_lib::process_guard::cleanup_outdated_or_ghost_instances();

        // Fix for NVIDIA explicit sync protocol issue on Wayland
        if std::env::var("__NV_DISABLE_EXPLICIT_SYNC").is_err() {
            std::env::set_var("__NV_DISABLE_EXPLICIT_SYNC", "1");
        }

        if std::env::args().any(|arg| arg == "--minimized") {
            std::env::set_var("LOOM_START_MINIMIZED", "1");
        }

        // Sur Linux (en particulier NVIDIA + Wayland), le moteur DMA-BUF de WebKitGTK provoque
        // des corruptions d'images, des clignotements et la disparition de l'application au scroll.
        // Désactiver le DMA-BUF force WebKitGTK à utiliser la mémoire partagée / buffers GL classiques,
        // ce qui résout 100% de ces bugs tout en conservant l'accélération GPU matérielle à 60+ FPS.
        if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }

        // Vérification de l'état de l'accélération matérielle
        let base_dir = dirs::data_local_dir()
            .unwrap_or_else(|| {
                std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
                    .join(".local/share")
            })
            .join("loom");

        let disable_hw_accel = base_dir.join("disable_hw_accel").exists();

        if disable_hw_accel {
            // Mode sécurisé logiciel si désactivé par l'utilisateur
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        } else {
            // Accélération matérielle GPU activée par défaut : compositing actif sans DMABUF
            std::env::remove_var("WEBKIT_DISABLE_COMPOSITING_MODE");
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

    loom_launcher_lib::run();
}
