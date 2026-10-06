//! Commandes exposées au frontend. Chaque commande délègue au cœur ou à `servers`.

use crate::servers::{self, StatsScan};
use crate::state::{AppPaths, AppState, MetricPoint, ServerState, ShutdownStatus};
use crate::loops;
use panel_core::config::{AppConfig, PlannedShutdown, ServerKind, CONFIG_VERSION};
use panel_core::history::{BugRow, PlayerRow, PublishRow, SuspiciousRow};
use panel_core::instances::DetectedInstance;
use panel_core::logs::LogLine;
use panel_core::mods::{DuplicateGroup, ImportResult, ModDiff, ModInfo};
use panel_core::ping::PingResult;
use panel_core::process::DetectedLaunch;
use panel_core::publish::{self, FileDiff, GitInfo, Manifest, Progress, PublishOptions, PublishReport};
use panel_core::sysmon::SystemInfo;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};

type R<T> = Result<T, String>;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> R<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())
}

// ---------- Configuration ----------

#[tauri::command(rename_all = "snake_case")]
pub async fn get_config(state: State<'_, AppState>) -> R<AppConfig> {
    Ok(state.config_snapshot())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_paths(state: State<'_, AppState>) -> R<AppPaths> {
    Ok(state.paths.clone())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn save_config(app: AppHandle, state: State<'_, AppState>, config: AppConfig) -> R<AppConfig> {
    let mut incoming = config;
    {
        let mut current = state.config.lock().map_err(|_| "Config verrouillée")?;
        // Le plan d'extinction appartient au backend, le frontend ne le réécrit pas.
        incoming.shutdown = current.shutdown.clone();
        incoming.version = CONFIG_VERSION;
        *current = incoming.clone();
    }
    state.save_config_to_disk()?;
    if let Ok(mut servers) = state.servers.lock() {
        for s in &incoming.servers {
            if let Some(rt) = servers.get_mut(&s.id) {
                rt.tracker.set_config(s.detection.clone());
            }
        }
        servers.retain(|id, rt| incoming.server(id).is_some() || rt.is_running());
    }
    for s in &incoming.servers {
        servers::ensure_runtime(&app, &s.id);
    }
    let _ = app.emit("config:changed", &incoming);
    Ok(incoming)
}

// ---------- Serveurs ----------

#[tauri::command(rename_all = "snake_case")]
pub async fn list_server_states(app: AppHandle) -> R<Vec<ServerState>> {
    Ok(servers::all_states(&app))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn server_start(app: AppHandle, id: String) -> R<()> {
    blocking(move || servers::start_server(&app, &id)).await?
}

#[tauri::command(rename_all = "snake_case")]
pub async fn server_stop(app: AppHandle, id: String) -> R<()> {
    servers::stop_server(&app, &id, 0)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn server_restart(app: AppHandle, id: String) -> R<()> {
    servers::restart_server(&app, &id)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn server_kill(app: AppHandle, id: String) -> R<()> {
    servers::kill_server(&app, &id)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn server_command(app: AppHandle, id: String, command: String) -> R<String> {
    blocking(move || servers::send_command(&app, &id, &command)).await?
}

#[tauri::command(rename_all = "snake_case")]
pub async fn server_log(app: AppHandle, state: State<'_, AppState>, id: String) -> R<Vec<LogLine>> {
    servers::ensure_runtime(&app, &id);
    let servers = state.servers.lock().map_err(|_| "État verrouillé")?;
    Ok(servers.get(&id).map(|rt| rt.log.iter().cloned().collect()).unwrap_or_default())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn server_clear_log(state: State<'_, AppState>, id: String) -> R<()> {
    if let Some(rt) = state.servers.lock().map_err(|_| "État verrouillé")?.get_mut(&id) {
        rt.log.clear();
    }
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn server_ping(app: AppHandle, id: String) -> R<PingResult> {
    blocking(move || servers::ping_server(&app, &id)).await?
}

#[tauri::command(rename_all = "snake_case")]
pub async fn server_detect_launch(dir: String) -> R<DetectedLaunch> {
    blocking(move || panel_core::process::detect_launch(Path::new(&dir))).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn server_players(state: State<'_, AppState>, id: String) -> R<Vec<PlayerRow>> {
    state.history.lock().map_err(|_| "Historique verrouillé")?.list_players(&id, 300)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn server_scan_stats(app: AppHandle, id: String) -> R<StatsScan> {
    blocking(move || servers::scan_stats(&app, &id)).await?
}

#[tauri::command(rename_all = "snake_case")]
pub async fn server_properties(state: State<'_, AppState>, id: String) -> R<HashMap<String, String>> {
    let cfg = state.server_config(&id).ok_or("Serveur inconnu")?;
    match &cfg.kind {
        ServerKind::Local(l) => Ok(panel_core::properties::read(Path::new(&l.dir))),
        ServerKind::Remote(_) => Err("Pas d'accès aux fichiers d'un serveur hébergé".into()),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn metrics_watch(state: State<'_, AppState>, id: String, watch: bool) -> R<()> {
    let mut m = state.monitor.lock().map_err(|_| "Moniteur verrouillé")?;
    if watch {
        m.watchers.insert(id);
    } else {
        m.watchers.remove(&id);
    }
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn metrics_history(state: State<'_, AppState>, id: String) -> R<Vec<MetricPoint>> {
    let servers = state.servers.lock().map_err(|_| "État verrouillé")?;
    Ok(servers.get(&id).map(|rt| rt.metrics.iter().cloned().collect()).unwrap_or_default())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn playit_start(app: AppHandle, id: String) -> R<()> {
    blocking(move || servers::start_playit(&app, &id)).await?
}

#[tauri::command(rename_all = "snake_case")]
pub async fn playit_stop(app: AppHandle, id: String) -> R<()> {
    servers::stop_playit(&app, &id)
}

// ---------- Mods et instances ----------

#[derive(Serialize)]
pub struct ModsListing {
    pub dir: String,
    pub dir_exists: bool,
    pub mods: Vec<ModInfo>,
    pub duplicates: Vec<DuplicateGroup>,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mods_list(dir: String) -> R<ModsListing> {
    blocking(move || {
        let path = PathBuf::from(&dir);
        let mods = panel_core::mods::scan_mods(&path);
        let duplicates = panel_core::mods::duplicates(&mods);
        ModsListing { dir, dir_exists: path.is_dir(), mods, duplicates }
    })
    .await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mods_set_enabled(path: String, enabled: bool) -> R<String> {
    blocking(move || panel_core::mods::set_enabled(Path::new(&path), enabled).map(|p| p.to_string_lossy().to_string())).await?
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mods_delete(path: String) -> R<()> {
    blocking(move || panel_core::mods::delete_mod(Path::new(&path))).await?
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mods_import(dir: String, files: Vec<String>) -> R<Vec<ImportResult>> {
    blocking(move || {
        let paths: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
        panel_core::mods::import_mods(Path::new(&dir), &paths)
    })
    .await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mods_diff(dir_a: String, dir_b: String) -> R<ModDiff> {
    blocking(move || {
        let a = panel_core::mods::scan_mods(Path::new(&dir_a));
        let b = panel_core::mods::scan_mods(Path::new(&dir_b));
        panel_core::mods::diff(&a, &b)
    })
    .await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn open_path(path: String) -> R<()> {
    panel_core::system::open_in_file_manager(Path::new(&path))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn instances_detect() -> R<Vec<DetectedInstance>> {
    blocking(panel_core::instances::detect).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn instance_inspect(dir: String) -> R<DetectedInstance> {
    blocking(move || panel_core::instances::inspect_dir(Path::new(&dir))).await
}

// ---------- Launcher / publication ----------

#[derive(Serialize)]
pub struct LauncherScan {
    pub files: Vec<FileDiff>,
    pub git: GitInfo,
    pub manifest: Option<Manifest>,
    pub history: Vec<PublishRow>,
    pub release_dir: String,
    pub manifest_file: String,
    pub publishing: bool,
    pub error: String,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn launcher_scan(app: AppHandle, id: String) -> R<LauncherScan> {
    blocking(move || {
        let state = app.state::<AppState>();
        let cfg = state.config_snapshot().launcher(&id).cloned().ok_or("Launcher inconnu")?;
        let release_dir = publish::release_dir(&cfg);
        let manifest_file = publish::manifest_file(&cfg);
        let manifest = publish::read_manifest(&manifest_file);
        let (files, error) = {
            let mut cache = state.sha_cache.lock().map_err(|_| "Cache verrouillé")?;
            match publish::scan_release_dir(&release_dir, &cfg.ignore_suffixes, &mut cache) {
                Ok(files) => (publish::compare(manifest.as_ref(), &files), String::new()),
                Err(e) => (Vec::new(), e),
            }
        };
        let git = publish::git_info(Path::new(&cfg.project_dir));
        let history = state.history.lock().ok().and_then(|h| h.list_publishes(&id, 20).ok()).unwrap_or_default();
        let publishing = state.publishing.lock().map(|p| p.contains(&id)).unwrap_or(false);
        Ok(LauncherScan {
            files,
            git,
            manifest,
            history,
            release_dir: release_dir.to_string_lossy().to_string(),
            manifest_file: manifest_file.to_string_lossy().to_string(),
            publishing,
            error,
        })
    })
    .await?
}

#[derive(Serialize, Clone)]
struct PublishProgress {
    launcher_id: String,
    progress: Progress,
}

#[derive(Serialize, Clone)]
struct PublishDone {
    launcher_id: String,
    report: Option<PublishReport>,
    error: String,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn launcher_publish(app: AppHandle, state: State<'_, AppState>, id: String, version: Option<String>, push: bool) -> R<()> {
    let cfg = state.config_snapshot().launcher(&id).cloned().ok_or("Launcher inconnu")?;
    {
        let mut publishing = state.publishing.lock().map_err(|_| "État verrouillé")?;
        if publishing.contains(&id) {
            return Err("Une publication est déjà en cours pour ce launcher".into());
        }
        publishing.insert(id.clone());
    }
    let version = version.filter(|v| !v.trim().is_empty()).unwrap_or_else(|| chrono::Local::now().format("%Y.%m.%d-%H%M").to_string());
    let opts = PublishOptions { version, push, excluded: cfg.excluded.iter().cloned().collect::<HashSet<_>>() };
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let launcher_id = id.clone();
        let on = |p: Progress| {
            let _ = app.emit("publish:progress", &PublishProgress { launcher_id: launcher_id.clone(), progress: p });
        };
        let result = match state.sha_cache.lock() {
            Ok(mut cache) => publish::publish(&cfg, &opts, &mut cache, &on),
            Err(_) => Err("Cache verrouillé".to_string()),
        };
        let ts = panel_core::now_rfc3339();
        if let Ok(history) = state.history.lock() {
            match &result {
                Ok(report) => {
                    let _ = history.record_publish(&id, report, true, &ts);
                }
                Err(e) => {
                    let failed = PublishReport { version: opts.version.clone(), message: e.clone(), ..Default::default() };
                    let _ = history.record_publish(&id, &failed, false, &ts);
                }
            }
        }
        if let Ok(mut publishing) = state.publishing.lock() {
            publishing.remove(&id);
        }
        let done = match result {
            Ok(report) => PublishDone { launcher_id: id, report: Some(report), error: String::new() },
            Err(e) => PublishDone { launcher_id: id, report: None, error: e },
        };
        let _ = app.emit("publish:done", &done);
    });
    Ok(())
}

// ---------- Historique ----------

#[tauri::command(rename_all = "snake_case")]
pub async fn bugs_list(state: State<'_, AppState>, server_id: Option<String>, include_resolved: bool) -> R<Vec<BugRow>> {
    state.history.lock().map_err(|_| "Historique verrouillé")?.list_bugs(server_id.as_deref(), include_resolved, 500)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn bug_resolve(app: AppHandle, state: State<'_, AppState>, id: i64, server_id: String, resolved: bool) -> R<()> {
    state.history.lock().map_err(|_| "Historique verrouillé")?.set_bug_resolved(id, resolved)?;
    servers::refresh_counts(&app, &server_id);
    servers::emit_state(&app, &server_id);
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn bug_delete(app: AppHandle, state: State<'_, AppState>, id: i64, server_id: String) -> R<()> {
    state.history.lock().map_err(|_| "Historique verrouillé")?.delete_bug(id)?;
    servers::refresh_counts(&app, &server_id);
    servers::emit_state(&app, &server_id);
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn suspicious_list(state: State<'_, AppState>, server_id: Option<String>) -> R<Vec<SuspiciousRow>> {
    state.history.lock().map_err(|_| "Historique verrouillé")?.list_suspicious(server_id.as_deref(), 500)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn suspicious_ack(app: AppHandle, state: State<'_, AppState>, id: i64, server_id: String, ack: bool) -> R<()> {
    state.history.lock().map_err(|_| "Historique verrouillé")?.ack_suspicious(id, ack)?;
    servers::refresh_counts(&app, &server_id);
    servers::emit_state(&app, &server_id);
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn suspicious_delete(app: AppHandle, state: State<'_, AppState>, id: i64, server_id: String) -> R<()> {
    state.history.lock().map_err(|_| "Historique verrouillé")?.delete_suspicious(id)?;
    servers::refresh_counts(&app, &server_id);
    servers::emit_state(&app, &server_id);
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn history_clear(app: AppHandle, state: State<'_, AppState>, server_id: String, bugs: bool, suspicious: bool) -> R<()> {
    state.history.lock().map_err(|_| "Historique verrouillé")?.clear_server(&server_id, bugs, suspicious)?;
    servers::refresh_counts(&app, &server_id);
    servers::emit_state(&app, &server_id);
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn crash_report_read(path: String) -> R<String> {
    blocking(move || {
        let bytes = std::fs::read(&path).map_err(|e| format!("Lecture : {e}"))?;
        let cut = bytes.len().min(200 * 1024);
        let mut text = String::from_utf8_lossy(&bytes[..cut]).to_string();
        if cut < bytes.len() {
            text.push_str("\n… (tronqué)");
        }
        Ok(text)
    })
    .await?
}

// ---------- Système ----------

#[derive(Serialize)]
pub struct SystemOverview {
    pub info: SystemInfo,
    pub app_pid: u32,
    pub paths: AppPaths,
    pub is_windows: bool,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn system_info(state: State<'_, AppState>) -> R<SystemOverview> {
    let info = state.monitor.lock().map_err(|_| "Moniteur verrouillé")?.monitor.info();
    Ok(SystemOverview { info, app_pid: std::process::id(), paths: state.paths.clone(), is_windows: cfg!(windows) })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn shutdown_plan(app: AppHandle, state: State<'_, AppState>, plan: PlannedShutdown) -> R<ShutdownStatus> {
    let at = panel_core::schedule::parse_plan_time(&plan).ok_or("Heure d'extinction invalide")?;
    if at <= chrono::Local::now() + chrono::Duration::seconds(30) {
        return Err("L'heure d'extinction doit être dans le futur (au moins 30 s)".into());
    }
    let mut plan = plan;
    plan.created_at = panel_core::now_rfc3339();
    plan.grace_s = plan.grace_s.clamp(10, 3600);
    {
        let mut cfg = state.config.lock().map_err(|_| "Config verrouillée")?;
        cfg.shutdown = Some(plan);
    }
    state.save_config_to_disk()?;
    if let Ok(mut s) = state.schedule.lock() {
        s.reset_shutdown();
    }
    loops::emit_shutdown_state(&app);
    Ok(loops::shutdown_status(&app))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn shutdown_cancel(app: AppHandle, state: State<'_, AppState>) -> R<ShutdownStatus> {
    if state.os_shutdown_armed.swap(false, Ordering::SeqCst) {
        panel_core::system::cancel_os_shutdown()?;
    }
    {
        let mut cfg = state.config.lock().map_err(|_| "Config verrouillée")?;
        cfg.shutdown = None;
    }
    state.save_config_to_disk()?;
    if let Ok(mut s) = state.schedule.lock() {
        s.reset_shutdown();
    }
    loops::emit_shutdown_state(&app);
    Ok(loops::shutdown_status(&app))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn shutdown_status(app: AppHandle) -> R<ShutdownStatus> {
    Ok(loops::shutdown_status(&app))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn running_servers(app: AppHandle) -> R<Vec<String>> {
    Ok(servers::running_server_names(&app))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn app_quit(app: AppHandle, state: State<'_, AppState>, stop_servers: bool) -> R<()> {
    state.quitting.store(true, Ordering::SeqCst);
    std::thread::spawn(move || {
        if stop_servers {
            servers::stop_all_blocking(&app);
        }
        app.exit(0);
    });
    Ok(())
}
