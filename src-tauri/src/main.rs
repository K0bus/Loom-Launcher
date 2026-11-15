// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    {
        // 0. Clean up any ghost or outdated instances BEFORE initializing Tauri & single_instance
        loom_launcher_lib::process_guard::cleanup_outdated_or_ghost_instances();

        if std::env::args().any(|arg| arg == "--minimized") {
            std::env::set_var("LOOM_START_MINIMIZED", "1");
        }

        // Vérification de l'état de configuration de l'accélération matérielle
        let base_dir = dirs::data_local_dir()
            .unwrap_or_else(|| {
                std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
                    .join(".local/share")
            })
            .join("loom");

        let disable_hw_accel = base_dir.join("disable_hw_accel").exists();

        // Nettoyage préalable complet pour garantir un état prévisible (notamment après un redémarrage)
        std::env::remove_var("LIBGL_ALWAYS_SOFTWARE");
        std::env::remove_var("GALLIUM_DRIVER");
        std::env::remove_var("WEBKIT_DISABLE_DMABUF_RENDERER");
        std::env::remove_var("WEBKIT_DISABLE_COMPOSITING_MODE");
        std::env::remove_var("WEBKIT_FORCE_COMPOSITING_MODE");

        if disable_hw_accel {
            // MODE SÉCURISÉ LOGICIEL (Software Safe Fallback)
            // Force WebKitGTK et Mesa à router 100% du rendu sur le rasterizer CPU (llvmpipe).
            // Bypass absolu du GPU matériel en cas d'artefacts, de crash ou de pilotes instables.
            eprintln!("[Loom::GPU] Démarrage en Mode Rendu Logiciel Sécurisé (llvmpipe / Safe Mode)");
            std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");
            std::env::set_var("GALLIUM_DRIVER", "llvmpipe");
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        } else {
            // MODE ACCÉLÉRATION MATÉRIELLE (Hardware GPU Compositing)
            // Laisse le GPU piloter le rendu via EGL / DMA-BUF / Mesa / Skia à pleine vitesse (VSync native / 60+ FPS).
            // Le protocole DMA-BUF zero-copy reste actif sur Wayland et X11.
            eprintln!("[Loom::GPU] Démarrage avec Accélération Matérielle GPU (EGL / DMA-BUF / Native Compositor)");
            std::env::set_var("WEBKIT_FORCE_COMPOSITING_MODE", "1");

            // Sur NVIDIA sous Wayland, explicit sync provoque l'Erreur de Protocole Wayland 71
            std::env::set_var("__NV_DISABLE_EXPLICIT_SYNC", "1");
        }
    }

    loom_launcher_lib::run();
}
