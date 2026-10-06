//! Boucles de fond : envoi groupé des logs, planification, ping, métriques, stats.

use crate::servers;
use crate::state::{AppState, MetricPoint, ServerStatus, ShutdownStatus};
use panel_core::config::ServerKind;
use panel_core::logs::LogLine;
use panel_core::schedule::{due_actions, shutdown_due, shutdown_expired, DueAction, ServerSnapshot};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const LOG_FLUSH_MS: u64 = 150;
const SCHEDULER_TICK_S: u64 = 15;
const METRIC_HISTORY: usize = 180;

#[derive(Serialize, Clone)]
struct LogBatch {
    server_id: String,
    lines: Vec<LogLine>,
}

#[derive(Serialize, Clone)]
struct MetricsPayload {
    server_id: String,
    point: MetricPoint,
}

#[derive(Serialize, Clone)]
struct AppMetrics {
    cpu: f32,
    mem: u64,
}

pub fn spawn_all(app: AppHandle) {
    spawn_log_flusher(app.clone());
    spawn_scheduler(app.clone());
    spawn_pinger(app.clone());
    spawn_metrics(app.clone());
    spawn_stats_scanner(app);
}

fn spawn_log_flusher(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(LOG_FLUSH_MS));
        let state = app.state::<AppState>();
        let pending: Vec<(String, LogLine)> = match state.pending_logs.lock() {
            Ok(mut p) if !p.is_empty() => std::mem::take(&mut *p),
            _ => Vec::new(),
        };
        if !pending.is_empty() {
            let mut by_server: HashMap<String, Vec<LogLine>> = HashMap::new();
            for (id, line) in pending {
                by_server.entry(id).or_default().push(line);
            }
            for (server_id, lines) in by_server {
                let _ = app.emit("server:log", &LogBatch { server_id, lines });
            }
        }
        servers::flush_idle_errors(&app, 800);
    });
}

pub fn shutdown_status(app: &AppHandle) -> ShutdownStatus {
    let state = app.state::<AppState>();
    let plan = state.config.lock().ok().and_then(|c| c.shutdown.clone());
    let seconds_left = plan
        .as_ref()
        .and_then(panel_core::schedule::parse_plan_time)
        .map(|at| (at - chrono::Local::now()).num_seconds())
        .unwrap_or(0);
    ShutdownStatus { plan, os_armed: state.os_shutdown_armed.load(Ordering::SeqCst), seconds_left }
}

pub fn emit_shutdown_state(app: &AppHandle) {
    let _ = app.emit("shutdown:state", &shutdown_status(app));
}

fn clear_shutdown_plan(app: &AppHandle) {
    let state = app.state::<AppState>();
    if let Ok(mut cfg) = state.config.lock() {
        cfg.shutdown = None;
    }
    let _ = state.save_config_to_disk();
    if let Ok(mut s) = state.schedule.lock() {
        s.reset_shutdown();
    }
    emit_shutdown_state(app);
}

fn scheduler_tick(app: &AppHandle) {
    let state = app.state::<AppState>();
    let cfg = state.config_snapshot();
    let running: HashMap<String, bool> = state
        .servers
        .lock()
        .map(|s| s.iter().map(|(id, rt)| (id.clone(), rt.is_running())).collect())
        .unwrap_or_default();
    let now = chrono::Local::now();
    let snapshots: Vec<ServerSnapshot> = cfg
        .servers
        .iter()
        .filter(|s| matches!(s.kind, ServerKind::Local(_)))
        .map(|s| ServerSnapshot { id: &s.id, schedule: &s.schedule, running: running.get(&s.id).copied().unwrap_or(false) })
        .collect();
    let actions = match state.schedule.lock() {
        Ok(mut st) => due_actions(now, &snapshots, &mut st),
        Err(_) => Vec::new(),
    };
    for action in actions {
        match action {
            DueAction::Open { server_id } => {
                servers::push_panel_line(app, &server_id, "Ouverture planifiée", "panel");
                if let Err(e) = servers::start_server(app, &server_id) {
                    servers::push_panel_line(app, &server_id, &format!("Ouverture planifiée impossible : {e}"), "panel");
                }
            }
            DueAction::WarnClose { server_id, seconds } => {
                let msg = if seconds >= 60 { format!("say Fermeture du serveur dans {} min", seconds / 60) } else { format!("say Fermeture du serveur dans {seconds} s") };
                let _ = servers::send_command(app, &server_id, &msg);
            }
            DueAction::Close { server_id } => {
                servers::push_panel_line(app, &server_id, "Fermeture planifiée", "panel");
                let _ = servers::stop_server(app, &server_id, 0);
            }
            DueAction::ShutdownWarn { .. } | DueAction::ShutdownNow => {}
        }
    }

    if let Some(plan) = cfg.shutdown.clone() {
        if shutdown_expired(now, &plan) {
            clear_shutdown_plan(app);
            return;
        }
        let due = state.schedule.lock().ok().and_then(|mut st| shutdown_due(now, &plan, &mut st));
        match due {
            Some(DueAction::ShutdownWarn { seconds }) => {
                let msg = if seconds >= 60 { format!("Extinction du PC dans {} min, le serveur va fermer", seconds / 60) } else { format!("Extinction du PC dans {seconds} s") };
                servers::say_all_running(app, &msg);
            }
            Some(DueAction::ShutdownNow) => {
                if !state.os_shutdown_armed.swap(true, Ordering::SeqCst) {
                    let app2 = app.clone();
                    thread::spawn(move || {
                        if plan.stop_servers {
                            servers::stop_all_blocking(&app2);
                        }
                        let result = panel_core::system::schedule_os_shutdown(plan.grace_s, "Telek Panel : extinction planifiée");
                        if result.is_err() {
                            app2.state::<AppState>().os_shutdown_armed.store(false, Ordering::SeqCst);
                        }
                        clear_shutdown_plan(&app2);
                        let _ = app2.emit("shutdown:result", &result.err().unwrap_or_default());
                    });
                }
            }
            _ => {}
        }
    }
}

fn spawn_scheduler(app: AppHandle) {
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(3));
        loop {
            scheduler_tick(&app);
            thread::sleep(Duration::from_secs(SCHEDULER_TICK_S));
        }
    });
}

fn spawn_pinger(app: AppHandle) {
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(2));
        loop {
            let state = app.state::<AppState>();
            let cfg = state.config_snapshot();
            let interval = cfg.settings.ping_interval_s.max(10);
            let running: HashMap<String, ServerStatus> = state
                .servers
                .lock()
                .map(|s| s.iter().map(|(id, rt)| (id.clone(), rt.status)).collect())
                .unwrap_or_default();
            let targets: Vec<String> = cfg
                .servers
                .iter()
                .filter(|s| match &s.kind {
                    ServerKind::Remote(r) => !r.host.trim().is_empty(),
                    ServerKind::Local(_) => matches!(running.get(&s.id), Some(ServerStatus::Running)),
                })
                .map(|s| s.id.clone())
                .collect();
            for id in targets {
                let _ = servers::ping_server(&app, &id);
            }
            thread::sleep(Duration::from_secs(interval));
        }
    });
}

fn spawn_metrics(app: AppHandle) {
    thread::spawn(move || loop {
        let state = app.state::<AppState>();
        let (visible_ms, hidden_ms) = state
            .config
            .lock()
            .map(|c| (c.settings.metrics_interval_visible_ms.max(500), c.settings.metrics_interval_hidden_ms))
            .unwrap_or((2000, 30_000));
        let watchers: Vec<String> = state.monitor.lock().map(|m| m.watchers.iter().cloned().collect()).unwrap_or_default();
        let watching = !watchers.is_empty();
        if !watching && hidden_ms == 0 {
            thread::sleep(Duration::from_secs(5));
            continue;
        }
        let pids: Vec<(String, u32, bool)> = {
            let cfg = state.config_snapshot();
            state
                .servers
                .lock()
                .map(|s| {
                    s.iter()
                        .filter_map(|(id, rt)| {
                            let p = rt.process.as_ref().filter(|p| p.is_running())?;
                            let script = cfg
                                .server(id)
                                .map(|c| matches!(&c.kind, ServerKind::Local(l) if matches!(l.launch, panel_core::config::LaunchTarget::Script { .. })))
                                .unwrap_or(false);
                            Some((id.clone(), p.pid, script))
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        let self_pid = std::process::id();
        let mut all_pids: Vec<u32> = pids.iter().map(|(_, pid, _)| *pid).collect();
        all_pids.push(self_pid);
        let include_children = pids.iter().any(|(_, _, script)| *script);
        let sample = match state.monitor.lock() {
            Ok(mut m) => m.monitor.sample(&all_pids, include_children),
            Err(_) => {
                thread::sleep(Duration::from_millis(visible_ms));
                continue;
            }
        };
        for (id, pid, _) in &pids {
            let proc_ = sample.processes.get(pid).cloned().unwrap_or_default();
            let point = MetricPoint {
                ts: sample.ts,
                cpu: proc_.cpu,
                mem: proc_.mem,
                sys_cpu: sample.cpu_total,
                sys_mem_used: sample.mem_used,
                sys_mem_total: sample.mem_total,
                rx_bps: sample.net_rx_bps,
                tx_bps: sample.net_tx_bps,
            };
            if let Ok(mut servers) = state.servers.lock() {
                if let Some(rt) = servers.get_mut(id) {
                    if rt.metrics.len() >= METRIC_HISTORY {
                        rt.metrics.pop_front();
                    }
                    rt.metrics.push_back(point.clone());
                }
            }
            if watchers.iter().any(|w| w == id) {
                let _ = app.emit("server:metrics", &MetricsPayload { server_id: id.clone(), point });
            }
        }
        if watchers.iter().any(|w| w == "app") {
            let me = sample.processes.get(&self_pid).cloned().unwrap_or_default();
            let _ = app.emit("app:metrics", &AppMetrics { cpu: me.cpu, mem: me.mem });
        }
        thread::sleep(Duration::from_millis(if watching { visible_ms } else { hidden_ms.max(5000) }));
    });
}

fn spawn_stats_scanner(app: AppHandle) {
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(20));
        loop {
            let state = app.state::<AppState>();
            let cfg = state.config_snapshot();
            let interval = Duration::from_secs(cfg.settings.stats_scan_interval_min.max(1) * 60);
            let due: Vec<String> = state
                .servers
                .lock()
                .map(|s| {
                    s.iter()
                        .filter(|(id, rt)| {
                            rt.status == ServerStatus::Running
                                && cfg.server(id).map(|c| c.detection.enabled && c.detection.mining && matches!(c.kind, ServerKind::Local(_))).unwrap_or(false)
                                && rt.last_stats_scan.map(|t| t.elapsed() >= interval).unwrap_or(true)
                        })
                        .map(|(id, _)| id.clone())
                        .collect()
                })
                .unwrap_or_default();
            for id in due {
                let _ = servers::scan_stats(&app, &id);
            }
            thread::sleep(Duration::from_secs(60));
        }
    });
}
