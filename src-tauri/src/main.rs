// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
#[repr(C)]
struct GLogField {
    key: *const libc::c_char,
    value: *const libc::c_void,
    length: libc::ssize_t,
}

#[cfg(target_os = "linux")]
unsafe extern "C" fn glib_log_writer(
    log_level: libc::c_int,
    fields: *const GLogField,
    n_fields: libc::size_t,
    user_data: *mut libc::c_void,
) -> libc::c_int {
    if !fields.is_null() {
        for i in 0..n_fields {
            let field = &*fields.add(i);
            if !field.key.is_null() && !field.value.is_null() {
                let key_str = std::ffi::CStr::from_ptr(field.key).to_string_lossy();
                if key_str == "MESSAGE" {
                    let msg = if field.length > 0 {
                        let slice = std::slice::from_raw_parts(field.value as *const u8, field.length as usize);
                        String::from_utf8_lossy(slice).to_string()
                    } else {
                        std::ffi::CStr::from_ptr(field.value as *const libc::c_char).to_string_lossy().to_string()
                    };

                    // Suppress known cosmetic/deprecated upstream library logs
                    if msg.contains("gtk_window_set_titlebar")
                        || msg.contains("libayatana-appindicator is deprecated")
                    {
                        return 0; // G_LOG_WRITER_HANDLED
                    }
                }
            }
        }
    }

    type GLogWriterDefault = unsafe extern "C" fn(
        libc::c_int,
        *const GLogField,
        libc::size_t,
        *mut libc::c_void,
    ) -> libc::c_int;

    let handle = libc::dlopen(b"libglib-2.0.so.0\0".as_ptr() as *const _, libc::RTLD_LAZY);
    if !handle.is_null() {
        let sym = libc::dlsym(handle, b"g_log_writer_default\0".as_ptr() as *const _);
        if !sym.is_null() {
            let default_writer: GLogWriterDefault = std::mem::transmute(sym);
            return default_writer(log_level, fields, n_fields, user_data);
        }
    }

    1 // G_LOG_WRITER_UNHANDLED
}

#[cfg(target_os = "linux")]
unsafe extern "C" fn glib_log_default_handler(
    log_domain: *const libc::c_char,
    log_level: libc::c_int,
    message: *const libc::c_char,
    user_data: *mut libc::c_void,
) {
    if !message.is_null() {
        let msg = std::ffi::CStr::from_ptr(message).to_string_lossy();
        if msg.contains("gtk_window_set_titlebar")
            || msg.contains("libayatana-appindicator is deprecated")
        {
            return;
        }
    }

    type GLogFunc = unsafe extern "C" fn(
        *const libc::c_char,
        libc::c_int,
        *const libc::c_char,
        *mut libc::c_void,
    );
    let handle = libc::dlopen(b"libglib-2.0.so.0\0".as_ptr() as *const _, libc::RTLD_LAZY);
    if !handle.is_null() {
        let sym = libc::dlsym(handle, b"g_log_default_handler\0".as_ptr() as *const _);
        if !sym.is_null() {
            let default_handler: GLogFunc = std::mem::transmute(sym);
            default_handler(log_domain, log_level, message, user_data);
        }
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    {
        // Suppress known cosmetic/deprecated upstream GTK & Ayatana warnings in terminal
        unsafe {
            let handle = libc::dlopen(b"libglib-2.0.so.0\0".as_ptr() as *const _, libc::RTLD_LAZY);
            if !handle.is_null() {
                let set_writer = libc::dlsym(handle, b"g_log_set_writer_func\0".as_ptr() as *const _);
                if !set_writer.is_null() {
                    let set_writer_fn: unsafe extern "C" fn(
                        unsafe extern "C" fn(libc::c_int, *const GLogField, libc::size_t, *mut libc::c_void) -> libc::c_int,
                        *mut libc::c_void,
                        Option<unsafe extern "C" fn(*mut libc::c_void)>,
                    ) = std::mem::transmute(set_writer);
                    set_writer_fn(glib_log_writer, std::ptr::null_mut(), None);
                }

                let set_default = libc::dlsym(handle, b"g_log_set_default_handler\0".as_ptr() as *const _);
                if !set_default.is_null() {
                    let set_default_fn: unsafe extern "C" fn(
                        unsafe extern "C" fn(*const libc::c_char, libc::c_int, *const libc::c_char, *mut libc::c_void),
                        *mut libc::c_void,
                    ) = std::mem::transmute(set_default);
                    set_default_fn(glib_log_default_handler, std::ptr::null_mut());
                }
            }
        }

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
