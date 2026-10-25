use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;

#[derive(Serialize)]
struct Ping {
    app: String,
    engine: &'static str,
    flavor: &'static str,
}

/// Connection check: proves the webview can call into Rust and that the engine crate is linked.
#[tauri::command]
fn ping(app: AppHandle) -> Ping {
    Ping {
        app: app.package_info().version.to_string(),
        engine: pulsar_engine::VERSION,
        flavor: if cfg!(debug_assertions) {
            "dev"
        } else {
            "release"
        },
    }
}

/// Returns the newer version on GitHub Releases, if any. Dev builds never update.
#[tauri::command]
async fn check_update(app: AppHandle) -> Result<Option<String>, String> {
    if cfg!(debug_assertions) {
        return Ok(None);
    }
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    Ok(update.map(|u| u.version))
}

#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    if cfg!(debug_assertions) {
        return Ok(());
    }
    let Some(update) = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    app.restart()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be registered first. Dev and release have different identifiers, so they don't block each other.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![ping, check_update, install_update])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
