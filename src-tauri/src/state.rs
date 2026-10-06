//! État partagé de l'application : configuration, runtimes serveurs, historique.

use panel_core::bugs::ErrorCollector;
use panel_core::config::{AppConfig, DetectionConfig, ServerConfig, ServerKind};
use panel_core::history::{Counts, History};
use panel_core::logs::LogLine;
use panel_core::ping::PingResult;
use panel_core::process::ManagedProcess;
use panel_core::publish::ShaCache;
use panel_core::schedule::{NextEvent, ScheduleState};
use panel_core::suspicious::Tracker;
use panel_core::sysmon::Monitor;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Serialize, Clone, Debug)]
pub struct AppPaths {
    pub config_file: PathBuf,
    pub history_db: PathBuf,
    pub data_dir: PathBuf,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServerStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Crashed,
    /// Serveur distant : état déduit du ping.
    Unknown,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct MetricPoint {
    pub ts: i64,
    pub cpu: f32,
    pub mem: u64,
    pub sys_cpu: f32,
    pub sys_mem_used: u64,
    pub sys_mem_total: u64,
    pub rx_bps: f64,
    pub tx_bps: f64,
}

#[derive(Serialize, Clone, Debug)]
pub struct OnlinePlayer {
    pub name: String,
    pub since: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct ServerState {
    pub id: String,
    pub status: ServerStatus,
    pub remote: bool,
    pub players: Vec<OnlinePlayer>,
    pub players_max: u32,
    pub uptime_s: u64,
    pub pid: Option<u32>,
    pub start_progress: u8,
    pub last_ping: Option<PingResult>,
    pub counts: Counts,
    pub next_event: Option<NextEvent>,
    pub playit_running: bool,
    pub last_error: String,
    pub log_lines: usize,
}

pub struct ServerRuntime {
    pub status: ServerStatus,
    pub process: Option<Arc<ManagedProcess>>,
    pub playit: Option<Arc<ManagedProcess>>,
    pub started_at: Option<Instant>,
    pub online: BTreeMap<String, chrono::DateTime<chrono::Local>>,
    pub players_max: u32,
    pub log: VecDeque<LogLine>,
    pub collector: ErrorCollector,
    pub tracker: Tracker,
    pub last_ping: Option<PingResult>,
    pub last_ping_at: Option<Instant>,
    pub metrics: VecDeque<MetricPoint>,
    pub last_error: String,
    pub start_progress: u8,
    pub stop_requested: bool,
    pub counts: Counts,
    pub restarts: VecDeque<Instant>,
    pub last_stats_scan: Option<Instant>,
}

impl ServerRuntime {
    pub fn new(detection: DetectionConfig, first_joins: HashMap<String, chrono::DateTime<chrono::Local>>, remote: bool) -> Self {
        Self {
            status: if remote { ServerStatus::Unknown } else { ServerStatus::Stopped },
            process: None,
            playit: None,
            started_at: None,
            online: BTreeMap::new(),
            players_max: 0,
            log: VecDeque::new(),
            collector: ErrorCollector::new(),
            tracker: Tracker::new(detection, first_joins),
            last_ping: None,
            last_ping_at: None,
            metrics: VecDeque::new(),
            last_error: String::new(),
            start_progress: 0,
            stop_requested: false,
            counts: Counts::default(),
            restarts: VecDeque::new(),
            last_stats_scan: None,
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self.status, ServerStatus::Starting | ServerStatus::Running | ServerStatus::Stopping)
    }

    pub fn push_log(&mut self, line: LogLine, cap: usize) {
        if self.log.len() >= cap.max(100) {
            self.log.pop_front();
        }
        self.log.push_back(line);
    }

    pub fn snapshot(&self, cfg: &ServerConfig) -> ServerState {
        ServerState {
            id: cfg.id.clone(),
            status: self.status,
            remote: matches!(cfg.kind, ServerKind::Remote(_)),
            players: self
                .online
                .iter()
                .map(|(n, t)| OnlinePlayer { name: n.clone(), since: t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true) })
                .collect(),
            players_max: self.players_max,
            uptime_s: self.started_at.map(|t| t.elapsed().as_secs()).unwrap_or(0),
            pid: self.process.as_ref().filter(|p| p.is_running()).map(|p| p.pid),
            start_progress: self.start_progress,
            last_ping: self.last_ping.clone(),
            counts: self.counts.clone(),
            next_event: panel_core::schedule::next_event(chrono::Local::now(), &cfg.schedule),
            playit_running: self.playit.as_ref().map(|p| p.is_running()).unwrap_or(false),
            last_error: self.last_error.clone(),
            log_lines: self.log.len(),
        }
    }
}

pub struct MonitorState {
    pub monitor: Monitor,
    /// Identifiants de serveurs (ou "app") dont quelqu'un regarde les métriques.
    pub watchers: HashSet<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct ShutdownStatus {
    pub plan: Option<panel_core::config::PlannedShutdown>,
    pub os_armed: bool,
    pub seconds_left: i64,
}

pub struct AppState {
    pub paths: AppPaths,
    pub config: Mutex<AppConfig>,
    pub history: Mutex<History>,
    pub servers: Mutex<HashMap<String, ServerRuntime>>,
    pub pending_logs: Mutex<Vec<(String, LogLine)>>,
    pub monitor: Mutex<MonitorState>,
    pub sha_cache: Mutex<ShaCache>,
    pub schedule: Mutex<ScheduleState>,
    pub publishing: Mutex<HashSet<String>>,
    pub os_shutdown_armed: AtomicBool,
    pub quitting: AtomicBool,
}

impl AppState {
    pub fn new(paths: AppPaths, config: AppConfig, history: History) -> Self {
        Self {
            paths,
            config: Mutex::new(config),
            history: Mutex::new(history),
            servers: Mutex::new(HashMap::new()),
            pending_logs: Mutex::new(Vec::new()),
            monitor: Mutex::new(MonitorState { monitor: Monitor::new(), watchers: HashSet::new() }),
            sha_cache: Mutex::new(ShaCache::default()),
            schedule: Mutex::new(ScheduleState::default()),
            publishing: Mutex::new(HashSet::new()),
            os_shutdown_armed: AtomicBool::new(false),
            quitting: AtomicBool::new(false),
        }
    }

    pub fn config_snapshot(&self) -> AppConfig {
        self.config.lock().map(|c| c.clone()).unwrap_or_default()
    }

    pub fn server_config(&self, id: &str) -> Option<ServerConfig> {
        self.config.lock().ok().and_then(|c| c.server(id).cloned())
    }

    pub fn save_config_to_disk(&self) -> Result<(), String> {
        let cfg = self.config_snapshot();
        panel_core::config::save(&self.paths.config_file, &cfg)
    }
}
