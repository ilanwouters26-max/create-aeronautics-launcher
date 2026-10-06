//! Cycle de vie des serveurs : lancement, console, arrêt, détection sur les logs.

use crate::state::{AppState, ServerRuntime, ServerState, ServerStatus};
use panel_core::bugs::BugReport;
use panel_core::config::{ServerConfig, ServerKind};
use panel_core::history::{BugRow, SuspiciousRow};
use panel_core::logs::{extract_event, parse_line, Level, LogEvent, LogLine};
use panel_core::ping::PingResult;
use panel_core::process::{build_launch_command, spawn, ExitHandler, LaunchCommand, LineHandler, StopOutcome};
use panel_core::stats::{MiningAlert, PlayerStats};
use panel_core::suspicious::{Severity, SuspiciousEvent};
use panel_core::{now_rfc3339, properties, rcon, stats};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

const PING_TIMEOUT: Duration = Duration::from_secs(4);
const RCON_TIMEOUT: Duration = Duration::from_secs(6);
const MAX_AUTO_RESTARTS: usize = 3;
const MINING_ALERT_COOLDOWN_H: i64 = 6;

#[derive(Serialize, Clone, Debug)]
pub struct StatsScan {
    pub stats: Vec<PlayerStats>,
    pub alerts: Vec<MiningAlert>,
    pub recorded: usize,
}

/// Crée le runtime d'un serveur s'il n'existe pas encore.
pub fn ensure_runtime(app: &AppHandle, id: &str) {
    let state = app.state::<AppState>();
    if state.servers.lock().map(|s| s.contains_key(id)).unwrap_or(true) {
        return;
    }
    let Some(cfg) = state.server_config(id) else { return };
    let first_joins = state.history.lock().ok().and_then(|h| h.first_joins(id).ok()).unwrap_or_default();
    let counts = state.history.lock().ok().and_then(|h| h.counts(id).ok()).unwrap_or_default();
    let remote = matches!(cfg.kind, ServerKind::Remote(_));
    let mut rt = ServerRuntime::new(cfg.detection.clone(), first_joins, remote);
    rt.counts = counts;
    let mut servers = match state.servers.lock() {
        Ok(s) => s,
        Err(_) => return,
    };
    servers.entry(id.to_string()).or_insert(rt);
}

pub fn snapshot(app: &AppHandle, id: &str) -> Option<ServerState> {
    let state = app.state::<AppState>();
    let cfg = state.server_config(id)?;
    let servers = state.servers.lock().ok()?;
    servers.get(id).map(|rt| rt.snapshot(&cfg))
}

pub fn emit_state(app: &AppHandle, id: &str) {
    if let Some(s) = snapshot(app, id) {
        let _ = app.emit("server:state", &s);
    }
}

pub fn all_states(app: &AppHandle) -> Vec<ServerState> {
    let cfg = app.state::<AppState>().config_snapshot();
    let ids: Vec<String> = cfg.servers.iter().map(|s| s.id.clone()).collect();
    for id in &ids {
        ensure_runtime(app, id);
    }
    let state = app.state::<AppState>();
    let servers = match state.servers.lock() {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    cfg.servers.iter().filter_map(|s| servers.get(&s.id).map(|rt| rt.snapshot(s))).collect()
}

/// Ligne synthétique du panneau (pas issue du serveur), affichée dans la console.
pub fn panel_line(message: &str, logger: &str) -> LogLine {
    LogLine {
        ts: chrono::Local::now().format("%H:%M:%S").to_string(),
        level: Level::Info,
        thread: String::new(),
        logger: logger.to_string(),
        message: message.to_string(),
        raw: message.to_string(),
        continuation: false,
    }
}

pub fn push_panel_line(app: &AppHandle, id: &str, message: &str, logger: &str) {
    let state = app.state::<AppState>();
    let cap = state.config.lock().map(|c| c.settings.log_buffer_lines).unwrap_or(2000);
    let line = panel_line(message, logger);
    if let Ok(mut servers) = state.servers.lock() {
        if let Some(rt) = servers.get_mut(id) {
            rt.push_log(line.clone(), cap);
        }
    }
    let mut pending = match state.pending_logs.lock() {
        Ok(p) => p,
        Err(_) => return,
    };
    pending.push((id.to_string(), line));
}

fn record_bug(app: &AppHandle, id: &str, bug: &BugReport, ts: &str) {
    let state = app.state::<AppState>();
    let row: Option<BugRow> = state.history.lock().ok().and_then(|h| h.record_bug(id, bug, ts).ok());
    if let Some(row) = row {
        let _ = app.emit("history:bug", &row);
        refresh_counts(app, id);
    }
}

fn record_suspicious(app: &AppHandle, id: &str, ev: &SuspiciousEvent, ts: &str) {
    let state = app.state::<AppState>();
    let inserted = state.history.lock().ok().and_then(|h| h.record_suspicious(id, ev, ts).ok());
    if let Some(row_id) = inserted {
        let row = SuspiciousRow {
            id: row_id,
            server_id: id.to_string(),
            ts: ts.to_string(),
            player: ev.player.clone(),
            category: ev.category.clone(),
            severity: ev.severity.as_str().to_string(),
            title: ev.title.clone(),
            details: ev.details.clone(),
            acknowledged: false,
        };
        let _ = app.emit("history:suspicious", &row);
        refresh_counts(app, id);
    }
}

pub fn refresh_counts(app: &AppHandle, id: &str) {
    let state = app.state::<AppState>();
    let counts = state.history.lock().ok().and_then(|h| h.counts(id).ok());
    if let Some(c) = counts {
        if let Ok(mut servers) = state.servers.lock() {
            if let Some(rt) = servers.get_mut(id) {
                rt.counts = c;
            }
        }
    }
}

/// Pipeline d'une ligne de log serveur : état, joueurs, bugs, actions suspectes.
pub fn handle_line(app: &AppHandle, id: &str, raw: String) {
    let state = app.state::<AppState>();
    let cap = state.config.lock().map(|c| c.settings.log_buffer_lines).unwrap_or(2000);
    let mut line = parse_line(&raw);
    if line.ts.is_empty() && !line.continuation {
        line.ts = chrono::Local::now().format("%H:%M:%S").to_string();
    }
    let event = extract_event(&line);
    let now = chrono::Local::now();
    let ts = now_rfc3339();
    let mut state_changed = false;
    let mut joined: Option<String> = None;
    let mut left: Option<String> = None;
    let mut list_sync: Vec<String> = Vec::new();
    let bug;
    let suspicious;
    {
        let mut servers = match state.servers.lock() {
            Ok(s) => s,
            Err(_) => return,
        };
        let Some(rt) = servers.get_mut(id) else { return };
        if let Some(ev) = &event {
            match ev {
                LogEvent::Started { .. } => {
                    if rt.status == ServerStatus::Starting {
                        rt.status = ServerStatus::Running;
                        rt.start_progress = 100;
                        state_changed = true;
                    }
                }
                LogEvent::Progress { percent } => {
                    rt.start_progress = *percent;
                    state_changed = true;
                }
                LogEvent::Stopping => {
                    if matches!(rt.status, ServerStatus::Running | ServerStatus::Starting) {
                        rt.status = ServerStatus::Stopping;
                        state_changed = true;
                    }
                }
                LogEvent::Joined { name, .. } => {
                    if !rt.online.contains_key(name) {
                        rt.online.insert(name.clone(), now);
                        joined = Some(name.clone());
                        state_changed = true;
                    }
                }
                LogEvent::Left { name, .. } => {
                    if rt.online.remove(name).is_some() {
                        left = Some(name.clone());
                        state_changed = true;
                    }
                }
                LogEvent::List { max, names, .. } => {
                    rt.players_max = *max;
                    for n in names {
                        if !rt.online.contains_key(n) {
                            rt.online.insert(n.clone(), now);
                            list_sync.push(n.clone());
                        }
                    }
                    rt.online.retain(|n, _| names.contains(n));
                    state_changed = true;
                }
                _ => {}
            }
            suspicious = rt.tracker.on_event(ev, now);
        } else {
            suspicious = None;
        }
        bug = rt.collector.feed(&line, event.as_ref());
        rt.push_log(line.clone(), cap);
    }
    if let Ok(history) = state.history.lock() {
        if let Some(n) = &joined {
            let _ = history.player_joined(id, n, &ts);
        }
        for n in &list_sync {
            let _ = history.player_joined(id, n, &ts);
        }
        if let Some(n) = &left {
            let _ = history.player_left(id, n, &ts);
        }
    }
    if let Some(b) = &bug {
        record_bug(app, id, b, &ts);
    }
    if let Some(s) = &suspicious {
        record_suspicious(app, id, s, &ts);
    }
    if let Ok(mut pending) = state.pending_logs.lock() {
        pending.push((id.to_string(), line));
    }
    if state_changed {
        emit_state(app, id);
    }
}

/// Clôt les erreurs en attente sans nouvelle ligne depuis `idle_ms`.
pub fn flush_idle_errors(app: &AppHandle, idle_ms: u128) {
    let state = app.state::<AppState>();
    let mut found: Vec<(String, BugReport)> = Vec::new();
    if let Ok(mut servers) = state.servers.lock() {
        for (id, rt) in servers.iter_mut() {
            if let Some(b) = rt.collector.flush_if_idle(idle_ms) {
                found.push((id.clone(), b));
            }
        }
    }
    let ts = now_rfc3339();
    for (id, b) in found {
        record_bug(app, &id, &b, &ts);
    }
}

pub fn on_exit(app: &AppHandle, id: &str, code: Option<i32>) {
    let state = app.state::<AppState>();
    let ts = now_rfc3339();
    let crashed;
    let pending_bug;
    let playit_to_stop;
    {
        let Ok(mut servers) = state.servers.lock() else { return };
        let Some(rt) = servers.get_mut(id) else { return };
        crashed = !rt.stop_requested && code != Some(0);
        rt.process = None;
        rt.started_at = None;
        rt.online.clear();
        rt.start_progress = 0;
        rt.status = if crashed { ServerStatus::Crashed } else { ServerStatus::Stopped };
        rt.stop_requested = false;
        pending_bug = rt.collector.finish();
        if crashed {
            rt.last_error = match code {
                Some(c) => format!("Le serveur s'est arrêté tout seul (code {c})"),
                None => "Le serveur s'est arrêté tout seul".to_string(),
            };
        }
        playit_to_stop = rt.playit.clone();
    }
    if let Ok(history) = state.history.lock() {
        let _ = history.close_sessions(id, &ts);
    }
    if let Some(b) = pending_bug {
        record_bug(app, id, &b, &ts);
    }
    let cfg = state.server_config(id);
    let local = cfg.as_ref().and_then(|c| match &c.kind {
        ServerKind::Local(l) => Some(l.clone()),
        ServerKind::Remote(_) => None,
    });
    if crashed {
        let bug = BugReport {
            category: "crash".into(),
            level: "fatal".into(),
            title: "Arrêt inattendu du serveur".into(),
            exception: String::new(),
            details: format!("Code de sortie : {}", code.map(|c| c.to_string()).unwrap_or_else(|| "inconnu".into())),
            fingerprint: "crash|exit".into(),
        };
        record_bug(app, id, &bug, &ts);
        push_panel_line(app, id, &format!("Arrêt inattendu (code {:?})", code), "panel");
    } else {
        push_panel_line(app, id, "Serveur arrêté", "panel");
    }
    let with_server = cfg.as_ref().map(|c| c.playit.with_server).unwrap_or(true);
    if with_server {
        if let Some(p) = playit_to_stop {
            p.kill();
            if let Ok(mut servers) = state.servers.lock() {
                if let Some(rt) = servers.get_mut(id) {
                    rt.playit = None;
                }
            }
        }
    }
    emit_state(app, id);

    let auto_restart = crashed && local.map(|l| l.auto_restart).unwrap_or(false) && !state.quitting.load(std::sync::atomic::Ordering::SeqCst);
    if auto_restart {
        let allowed = {
            let Ok(mut servers) = state.servers.lock() else { return };
            let Some(rt) = servers.get_mut(id) else { return };
            let cutoff = Instant::now() - Duration::from_secs(600);
            rt.restarts.retain(|t| *t > cutoff);
            if rt.restarts.len() < MAX_AUTO_RESTARTS {
                rt.restarts.push_back(Instant::now());
                true
            } else {
                rt.last_error = "Trop de redémarrages automatiques en 10 min, relance manuelle nécessaire".into();
                false
            }
        };
        if allowed {
            push_panel_line(app, id, "Redémarrage automatique dans 5 s", "panel");
            let app = app.clone();
            let id = id.to_string();
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(5));
                if let Err(e) = start_server(&app, &id) {
                    push_panel_line(&app, &id, &format!("Redémarrage automatique impossible : {e}"), "panel");
                }
            });
        } else {
            emit_state(app, id);
        }
    }
}

fn local_cfg(cfg: &ServerConfig) -> Result<panel_core::config::LocalServer, String> {
    match &cfg.kind {
        ServerKind::Local(l) => Ok(l.clone()),
        ServerKind::Remote(_) => Err("Serveur hébergé : démarrage et arrêt passent par le panel de l'hébergeur".into()),
    }
}

pub fn start_server(app: &AppHandle, id: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    let cfg = state.server_config(id).ok_or("Serveur inconnu")?;
    let local = local_cfg(&cfg)?;
    ensure_runtime(app, id);
    let cmd: LaunchCommand = build_launch_command(&local)?;
    {
        let mut servers = state.servers.lock().map_err(|_| "État verrouillé")?;
        let rt = servers.get_mut(id).ok_or("Runtime absent")?;
        if rt.is_running() {
            return Err("Le serveur est déjà lancé".into());
        }
        rt.status = ServerStatus::Starting;
        rt.stop_requested = false;
        rt.start_progress = 0;
        rt.last_error.clear();
        rt.started_at = Some(Instant::now());
        rt.collector = panel_core::bugs::ErrorCollector::new();
    }
    push_panel_line(app, id, &format!("Lancement : {}", cmd.display()), "panel");
    let (app_l, id_l) = (app.clone(), id.to_string());
    let on_line: LineHandler = Arc::new(move |raw| handle_line(&app_l, &id_l, raw));
    let (app_e, id_e) = (app.clone(), id.to_string());
    let on_exit_cb: ExitHandler = Box::new(move |code| on_exit(&app_e, &id_e, code));
    match spawn(&cmd, on_line, on_exit_cb) {
        Ok(p) => {
            if let Ok(mut servers) = state.servers.lock() {
                if let Some(rt) = servers.get_mut(id) {
                    rt.process = Some(p);
                }
            }
        }
        Err(e) => {
            if let Ok(mut servers) = state.servers.lock() {
                if let Some(rt) = servers.get_mut(id) {
                    rt.status = ServerStatus::Stopped;
                    rt.started_at = None;
                    rt.last_error = e.clone();
                }
            }
            push_panel_line(app, id, &format!("Échec du lancement : {e}"), "panel");
            emit_state(app, id);
            return Err(e);
        }
    }
    if cfg.playit.enabled && cfg.playit.with_server {
        if let Err(e) = start_playit(app, id) {
            push_panel_line(app, id, &format!("playit : {e}"), "playit");
        }
    }
    emit_state(app, id);
    Ok(())
}

fn countdown_messages(proc_: &Arc<panel_core::process::ManagedProcess>, total: u64, what: &str) {
    let mut remaining = total;
    let checkpoints = [600u64, 300, 120, 60, 30, 10];
    while remaining > 0 && proc_.is_running() {
        let msg = if remaining >= 60 { format!("{what} dans {} min", remaining / 60) } else { format!("{what} dans {remaining} s") };
        let _ = proc_.write_line(&format!("say {msg}"));
        let next = checkpoints.iter().copied().filter(|c| *c < remaining).max().unwrap_or(0);
        let sleep_for = remaining - next;
        let step_end = Instant::now() + Duration::from_secs(sleep_for);
        while Instant::now() < step_end && proc_.is_running() {
            thread::sleep(Duration::from_millis(250));
        }
        remaining = next;
    }
}

/// Arrêt propre bloquant : annonce optionnelle, save-all, stop, kill de secours.
pub fn stop_server_blocking(app: &AppHandle, id: &str, warn_seconds: u64) -> Result<StopOutcome, String> {
    let state = app.state::<AppState>();
    let cfg = state.server_config(id).ok_or("Serveur inconnu")?;
    let local = local_cfg(&cfg)?;
    let proc_ = {
        let mut servers = state.servers.lock().map_err(|_| "État verrouillé")?;
        let rt = servers.get_mut(id).ok_or("Serveur non lancé")?;
        let p = rt.process.clone().filter(|p| p.is_running()).ok_or("Serveur non lancé")?;
        rt.stop_requested = true;
        rt.status = ServerStatus::Stopping;
        p
    };
    emit_state(app, id);
    if warn_seconds > 0 {
        push_panel_line(app, id, &format!("Fermeture annoncée dans {warn_seconds} s"), "panel");
        countdown_messages(&proc_, warn_seconds, "Fermeture du serveur");
    }
    push_panel_line(app, id, "Arrêt demandé (save-all puis stop)", "panel");
    let _ = proc_.write_line("say Fermeture du serveur, sauvegarde en cours");
    let _ = proc_.write_line("save-all");
    thread::sleep(Duration::from_millis(800));
    let outcome = proc_.graceful_stop(Duration::from_secs(local.stop_timeout_s.max(10)));
    if outcome == StopOutcome::Killed {
        push_panel_line(app, id, "Le serveur ne répondait plus, processus tué", "panel");
    }
    if cfg.playit.with_server {
        let _ = stop_playit(app, id);
    }
    Ok(outcome)
}

pub fn stop_server(app: &AppHandle, id: &str, warn_seconds: u64) -> Result<(), String> {
    let state = app.state::<AppState>();
    let cfg = state.server_config(id).ok_or("Serveur inconnu")?;
    local_cfg(&cfg)?;
    let running = state
        .servers
        .lock()
        .map(|s| s.get(id).and_then(|rt| rt.process.as_ref()).map(|p| p.is_running()).unwrap_or(false))
        .unwrap_or(false);
    if !running {
        return Err("Serveur non lancé".into());
    }
    let app = app.clone();
    let id = id.to_string();
    thread::spawn(move || {
        if let Err(e) = stop_server_blocking(&app, &id, warn_seconds) {
            push_panel_line(&app, &id, &format!("Arrêt : {e}"), "panel");
        }
    });
    Ok(())
}

pub fn restart_server(app: &AppHandle, id: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    let cfg = state.server_config(id).ok_or("Serveur inconnu")?;
    local_cfg(&cfg)?;
    let app = app.clone();
    let id = id.to_string();
    thread::spawn(move || {
        let _ = stop_server_blocking(&app, &id, 0);
        // Laisse le thread de surveillance passer le statut à "arrêté".
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let still = app
                .state::<AppState>()
                .servers
                .lock()
                .map(|s| s.get(&id).map(|rt| rt.is_running()).unwrap_or(false))
                .unwrap_or(false);
            if !still || Instant::now() > deadline {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        if let Err(e) = start_server(&app, &id) {
            push_panel_line(&app, &id, &format!("Redémarrage : {e}"), "panel");
        }
    });
    Ok(())
}

pub fn kill_server(app: &AppHandle, id: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    let proc_ = {
        let mut servers = state.servers.lock().map_err(|_| "État verrouillé")?;
        let rt = servers.get_mut(id).ok_or("Serveur non lancé")?;
        rt.stop_requested = true;
        rt.process.clone().ok_or("Serveur non lancé")?
    };
    push_panel_line(app, id, "Arrêt forcé", "panel");
    proc_.kill();
    Ok(())
}

pub fn send_command(app: &AppHandle, id: &str, command: &str) -> Result<String, String> {
    let state = app.state::<AppState>();
    let cfg = state.server_config(id).ok_or("Serveur inconnu")?;
    let command = command.trim().trim_start_matches('/').to_string();
    if command.is_empty() {
        return Ok(String::new());
    }
    match &cfg.kind {
        ServerKind::Local(_) => {
            let proc_ = state
                .servers
                .lock()
                .ok()
                .and_then(|s| s.get(id).and_then(|rt| rt.process.clone()))
                .filter(|p| p.is_running())
                .ok_or("Serveur non lancé")?;
            push_panel_line(app, id, &format!("> {command}"), "console");
            proc_.write_line(&command)?;
            Ok(String::new())
        }
        ServerKind::Remote(r) => {
            if !r.rcon.enabled {
                return Err("RCON désactivé pour ce serveur (voir sa configuration)".into());
            }
            let host = if r.rcon.host.trim().is_empty() { r.host.clone() } else { r.rcon.host.clone() };
            ensure_runtime(app, id);
            push_panel_line(app, id, &format!("> {command}"), "console");
            let mut client = rcon::RconClient::connect(&host, r.rcon.port, &r.rcon.password, RCON_TIMEOUT)?;
            let response = client.command(&command)?;
            for l in response.lines().filter(|l| !l.trim().is_empty()) {
                push_panel_line(app, id, l, "rcon");
            }
            if response.trim().is_empty() {
                push_panel_line(app, id, "(pas de réponse)", "rcon");
            }
            Ok(response)
        }
    }
}

pub fn ping_target(cfg: &ServerConfig) -> (String, u16) {
    match &cfg.kind {
        ServerKind::Local(l) => {
            let port = properties::server_port(Path::new(&l.dir)).unwrap_or(l.port);
            ("127.0.0.1".to_string(), port)
        }
        ServerKind::Remote(r) => (r.host.clone(), r.port),
    }
}

pub fn ping_server(app: &AppHandle, id: &str) -> Result<PingResult, String> {
    let state = app.state::<AppState>();
    let cfg = state.server_config(id).ok_or("Serveur inconnu")?;
    ensure_runtime(app, id);
    let (host, port) = ping_target(&cfg);
    if host.trim().is_empty() {
        return Err("Adresse du serveur non renseignée".into());
    }
    let result = panel_core::ping::ping(&host, port, PING_TIMEOUT);
    let now = chrono::Local::now();
    if let Ok(mut servers) = state.servers.lock() {
        if let Some(rt) = servers.get_mut(id) {
            rt.last_ping = Some(result.clone());
            rt.last_ping_at = Some(Instant::now());
            if result.online && result.players_max > 0 {
                rt.players_max = result.players_max;
            }
            if matches!(cfg.kind, ServerKind::Remote(_)) {
                rt.status = if result.online { ServerStatus::Running } else { ServerStatus::Stopped };
                if result.online {
                    for n in &result.sample {
                        rt.online.entry(n.clone()).or_insert(now);
                    }
                    if result.sample.len() as u32 >= result.players_online {
                        rt.online.retain(|n, _| result.sample.contains(n));
                    }
                } else {
                    rt.online.clear();
                }
            }
        }
    }
    emit_state(app, id);
    Ok(result)
}

pub fn scan_stats(app: &AppHandle, id: &str) -> Result<StatsScan, String> {
    let state = app.state::<AppState>();
    let cfg = state.server_config(id).ok_or("Serveur inconnu")?;
    let local = match &cfg.kind {
        ServerKind::Local(l) => l.clone(),
        ServerKind::Remote(_) => return Err("Statistiques joueurs indisponibles sur un serveur hébergé (pas d'accès aux fichiers)".into()),
    };
    let dir = PathBuf::from(&local.dir);
    let level = properties::level_name(&dir);
    let player_stats = stats::read_player_stats(&dir, &level);
    let alerts = stats::evaluate_mining(&player_stats, &cfg.detection);
    let ts = now_rfc3339();
    let now = chrono::Local::now();
    let mut recorded = 0;
    for a in &alerts {
        let recent = state
            .history
            .lock()
            .ok()
            .and_then(|h| h.last_mining_alert(id, &a.player, &a.rule).ok().flatten())
            .and_then(|t| chrono::DateTime::parse_from_rfc3339(&t).ok())
            .map(|t| (now - t.with_timezone(&chrono::Local)).num_hours() < MINING_ALERT_COOLDOWN_H)
            .unwrap_or(false);
        if recent {
            continue;
        }
        let ev = SuspiciousEvent {
            severity: Severity::High,
            player: a.player.clone(),
            category: "mining".into(),
            title: format!("{} : {} {}/h ({} en {} h de jeu)", a.player, a.rule, a.per_hour, a.count, a.play_hours),
            details: format!("Seuil : {} par heure. Lu dans world/stats, mis à jour à chaque sauvegarde du serveur.", a.threshold),
        };
        record_suspicious(app, id, &ev, &ts);
        recorded += 1;
    }
    if let Ok(mut servers) = state.servers.lock() {
        if let Some(rt) = servers.get_mut(id) {
            rt.last_stats_scan = Some(Instant::now());
        }
    }
    Ok(StatsScan { stats: player_stats, alerts, recorded })
}

pub fn start_playit(app: &AppHandle, id: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    let cfg = state.server_config(id).ok_or("Serveur inconnu")?;
    if cfg.playit.exe_path.trim().is_empty() {
        return Err("Chemin de l'agent playit non renseigné".into());
    }
    ensure_runtime(app, id);
    let already = state
        .servers
        .lock()
        .map(|s| s.get(id).and_then(|rt| rt.playit.as_ref()).map(|p| p.is_running()).unwrap_or(false))
        .unwrap_or(false);
    if already {
        return Err("L'agent playit tourne déjà".into());
    }
    let exe = PathBuf::from(cfg.playit.exe_path.trim());
    let cwd = exe.parent().map(|p| p.to_path_buf()).filter(|p| p.is_dir()).unwrap_or_else(std::env::temp_dir);
    let cmd = LaunchCommand { program: exe.to_string_lossy().to_string(), args: panel_core::process::split_args(&cfg.playit.args), cwd };
    let (app_l, id_l) = (app.clone(), id.to_string());
    let on_line: LineHandler = Arc::new(move |raw| push_panel_line(&app_l, &id_l, &raw, "playit"));
    let (app_e, id_e) = (app.clone(), id.to_string());
    let on_exit_cb: ExitHandler = Box::new(move |code| {
        push_panel_line(&app_e, &id_e, &format!("agent arrêté (code {:?})", code), "playit");
        if let Ok(mut servers) = app_e.state::<AppState>().servers.lock() {
            if let Some(rt) = servers.get_mut(&id_e) {
                rt.playit = None;
            }
        }
        emit_state(&app_e, &id_e);
    });
    let p = spawn(&cmd, on_line, on_exit_cb)?;
    if let Ok(mut servers) = state.servers.lock() {
        if let Some(rt) = servers.get_mut(id) {
            rt.playit = Some(p);
        }
    }
    push_panel_line(app, id, "agent lancé", "playit");
    emit_state(app, id);
    Ok(())
}

pub fn stop_playit(app: &AppHandle, id: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    let p = state.servers.lock().ok().and_then(|mut s| s.get_mut(id).and_then(|rt| rt.playit.take()));
    match p {
        Some(p) => {
            p.kill();
            emit_state(app, id);
            Ok(())
        }
        None => Err("L'agent playit n'est pas lancé".into()),
    }
}

pub fn say_all_running(app: &AppHandle, message: &str) {
    let state = app.state::<AppState>();
    let procs: Vec<Arc<panel_core::process::ManagedProcess>> = state
        .servers
        .lock()
        .map(|s| s.values().filter_map(|rt| rt.process.clone()).filter(|p| p.is_running()).collect())
        .unwrap_or_default();
    for p in procs {
        let _ = p.write_line(&format!("say {message}"));
    }
}

pub fn running_local_ids(app: &AppHandle) -> Vec<String> {
    let state = app.state::<AppState>();
    state
        .servers
        .lock()
        .map(|s| {
            s.iter()
                .filter(|(_, rt)| rt.process.as_ref().map(|p| p.is_running()).unwrap_or(false))
                .map(|(id, _)| id.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Arrête tous les serveurs locaux en parallèle et attend la fin.
pub fn stop_all_blocking(app: &AppHandle) {
    let ids = running_local_ids(app);
    let handles: Vec<_> = ids
        .into_iter()
        .map(|id| {
            let app = app.clone();
            thread::spawn(move || {
                let _ = stop_server_blocking(&app, &id, 0);
            })
        })
        .collect();
    for h in handles {
        let _ = h.join();
    }
}

pub fn running_server_names(app: &AppHandle) -> Vec<String> {
    let cfg = app.state::<AppState>().config_snapshot();
    running_local_ids(app).iter().filter_map(|id| cfg.server(id).map(|s| s.name.clone())).collect()
}
