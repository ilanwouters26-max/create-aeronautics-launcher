mod commands;
mod loops;
mod servers;
mod state;
mod tray;

use panel_core::history::History;
use serde::Serialize;
use state::{AppPaths, AppState};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};

#[derive(Serialize, Clone)]
struct QuitRequested {
    running: Vec<String>,
}

/// Quitte directement si rien ne tourne, sinon laisse le frontend demander quoi faire des serveurs.
pub fn request_quit(app: &AppHandle) {
    let running = servers::running_server_names(app);
    if running.is_empty() {
        app.state::<AppState>().quitting.store(true, Ordering::SeqCst);
        app.exit(0);
    } else {
        tray::show_main(app);
        let _ = app.emit("app:quit-requested", &QuitRequested { running });
    }
}

fn resolve_paths(app: &AppHandle) -> tauri::Result<AppPaths> {
    let config_dir = app.path().app_config_dir()?;
    let data_dir = app.path().app_data_dir()?;
    Ok(AppPaths { config_file: config_dir.join("config.json"), history_db: data_dir.join("history.db"), data_dir })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .setup(|app| {
            let handle = app.handle().clone();
            let paths = resolve_paths(&handle)?;
            let mut config = panel_core::config::load(&paths.config_file).unwrap_or_else(|e| {
                eprintln!("{e}");
                panel_core::config::AppConfig::default()
            });
            if let Some(plan) = &config.shutdown {
                if panel_core::schedule::shutdown_expired(chrono::Local::now(), plan) {
                    config.shutdown = None;
                }
            }
            let history = History::open(&paths.history_db)?;
            let start_minimized = config.settings.start_minimized || std::env::args().any(|a| a == "--minimized");
            app.manage(AppState::new(paths, config, history));
            tray::build(&handle)?;
            loops::spawn_all(handle.clone());
            if !start_minimized {
                tray::show_main(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let close_to_tray = app.state::<AppState>().config.lock().map(|c| c.settings.close_to_tray).unwrap_or(true);
                api.prevent_close();
                if close_to_tray {
                    let _ = window.hide();
                } else {
                    request_quit(app);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::get_paths,
            commands::save_config,
            commands::list_server_states,
            commands::server_start,
            commands::server_stop,
            commands::server_restart,
            commands::server_kill,
            commands::server_command,
            commands::server_log,
            commands::server_clear_log,
            commands::server_ping,
            commands::server_detect_launch,
            commands::server_players,
            commands::server_scan_stats,
            commands::server_properties,
            commands::metrics_watch,
            commands::metrics_history,
            commands::playit_start,
            commands::playit_stop,
            commands::mods_list,
            commands::mods_set_enabled,
            commands::mods_delete,
            commands::mods_import,
            commands::mods_diff,
            commands::open_path,
            commands::instances_detect,
            commands::instance_inspect,
            commands::launcher_scan,
            commands::launcher_publish,
            commands::bugs_list,
            commands::bug_resolve,
            commands::bug_delete,
            commands::suspicious_list,
            commands::suspicious_ack,
            commands::suspicious_delete,
            commands::history_clear,
            commands::crash_report_read,
            commands::system_info,
            commands::shutdown_plan,
            commands::shutdown_cancel,
            commands::shutdown_status,
            commands::running_servers,
            commands::app_quit,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri n'a pas pu démarrer");
}
