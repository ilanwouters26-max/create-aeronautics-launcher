// Types miroirs des structures Rust (panel-core et src-tauri).

export interface AppSettings {
  close_to_tray: boolean;
  start_minimized: boolean;
  metrics_interval_visible_ms: number;
  metrics_interval_hidden_ms: number;
  ping_interval_s: number;
  stats_scan_interval_min: number;
  log_buffer_lines: number;
}

export type LaunchTarget =
  | { mode: "jar"; path: string }
  | { mode: "argsfile"; path: string }
  | { mode: "script"; path: string };

export interface LocalServer {
  dir: string;
  java_path: string;
  jvm_args: string;
  launch: LaunchTarget;
  extra_args: string;
  port: number;
  auto_restart: boolean;
  stop_timeout_s: number;
  stop_warning_s: number;
}

export interface RconConfig {
  enabled: boolean;
  host: string;
  port: number;
  password: string;
}

export interface RemoteServer {
  host: string;
  port: number;
  rcon: RconConfig;
}

export type ServerKind = ({ type: "local" } & LocalServer) | ({ type: "remote" } & RemoteServer);

export interface ScheduleConfig {
  enabled: boolean;
  open_time: string;
  close_time: string;
  warning_s: number;
}

export interface EarlyAdvancementRule {
  title: string;
  max_minutes: number;
}

export interface MiningRule {
  label: string;
  blocks: string[];
  per_hour: number;
}

export interface DetectionConfig {
  enabled: boolean;
  gamemode: boolean;
  give: boolean;
  op: boolean;
  teleport: boolean;
  other_admin: boolean;
  early_advancements: boolean;
  early_rules: EarlyAdvancementRule[];
  mining: boolean;
  mining_rules: MiningRule[];
  mining_min_playtime_min: number;
}

export interface PlayitConfig {
  enabled: boolean;
  exe_path: string;
  args: string;
  with_server: boolean;
}

export interface ServerConfig {
  id: string;
  name: string;
  kind: ServerKind;
  loader: string;
  mc_version: string;
  schedule: ScheduleConfig;
  detection: DetectionConfig;
  playit: PlayitConfig;
  notes: string;
}

export interface InstanceConfig {
  id: string;
  name: string;
  dir: string;
  source: string;
  loader: string;
  mc_version: string;
}

export interface LauncherConfig {
  id: string;
  name: string;
  project_dir: string;
  release_dir: string;
  dist_dir: string;
  manifest_path: string;
  branch: string;
  base_url: string;
  commit_message: string;
  excluded: string[];
  ignore_suffixes: string[];
}

export interface PlannedShutdown {
  at: string;
  stop_servers: boolean;
  grace_s: number;
  warning_s: number;
  created_at: string;
}

export interface AppConfig {
  version: number;
  settings: AppSettings;
  servers: ServerConfig[];
  instances: InstanceConfig[];
  launchers: LauncherConfig[];
  shutdown: PlannedShutdown | null;
}

export type ServerStatus = "stopped" | "starting" | "running" | "stopping" | "crashed" | "unknown";

export interface PingResult {
  online: boolean;
  players_online: number;
  players_max: number;
  version: string;
  protocol: number;
  motd: string;
  latency_ms: number;
  sample: string[];
  error: string;
}

export interface Counts {
  bugs_open: number;
  suspicious_open: number;
}

export interface NextEvent {
  kind: "open" | "close";
  at: string;
  in_seconds: number;
}

export interface OnlinePlayer {
  name: string;
  since: string;
}

export interface ServerState {
  id: string;
  status: ServerStatus;
  remote: boolean;
  players: OnlinePlayer[];
  players_max: number;
  uptime_s: number;
  pid: number | null;
  start_progress: number;
  last_ping: PingResult | null;
  counts: Counts;
  next_event: NextEvent | null;
  playit_running: boolean;
  last_error: string;
  log_lines: number;
}

export type LogLevel = "trace" | "debug" | "info" | "warn" | "error" | "fatal" | "unknown";

export interface LogLine {
  ts: string;
  level: LogLevel;
  thread: string;
  logger: string;
  message: string;
  raw: string;
  continuation: boolean;
}

export interface MetricPoint {
  ts: number;
  cpu: number;
  mem: number;
  sys_cpu: number;
  sys_mem_used: number;
  sys_mem_total: number;
  rx_bps: number;
  tx_bps: number;
}

export interface DetectedLaunch {
  launch: LaunchTarget | null;
  loader: string;
  mc_version: string;
  jvm_args: string;
  notes: string[];
}

export interface PlayerRow {
  name: string;
  first_seen: string;
  last_seen: string;
  sessions: number;
  total_minutes: number;
}

export interface PlayerStats {
  uuid: string;
  name: string;
  play_hours: number;
  mined: Record<string, number>;
  deaths: number;
  player_kills: number;
}

export interface MiningAlert {
  player: string;
  rule: string;
  count: number;
  per_hour: number;
  threshold: number;
  play_hours: number;
}

export interface StatsScan {
  stats: PlayerStats[];
  alerts: MiningAlert[];
  recorded: number;
}

export interface ModInfo {
  file_name: string;
  path: string;
  id: string;
  name: string;
  version: string;
  loader: string;
  environment: string;
  description: string;
  size: number;
  enabled: boolean;
  modified: number;
}

export interface DuplicateGroup {
  key: string;
  files: string[];
}

export interface ModsListing {
  dir: string;
  dir_exists: boolean;
  mods: ModInfo[];
  duplicates: DuplicateGroup[];
}

export interface ImportResult {
  file: string;
  ok: boolean;
  message: string;
}

export interface ModDiff {
  only_a: ModInfo[];
  only_b: ModInfo[];
  version_mismatch: [ModInfo, ModInfo][];
  same: number;
}

export interface DetectedInstance {
  source: string;
  name: string;
  dir: string;
  mc_version: string;
  loader: string;
  mods_count: number;
}

export type FileStatus = "new" | "changed" | "unchanged" | "removed";

export interface FileDiff {
  path: string;
  name: string;
  size: number;
  sha1: string;
  status: FileStatus;
}

export interface GitInfo {
  available: boolean;
  is_repo: boolean;
  branch: string;
  last_commit: string;
  remote: string;
  dirty_count: number;
  error: string;
}

export interface ManifestEntry {
  path: string;
  size: number;
  sha1: string;
  url: string;
}

export interface Manifest {
  version: string;
  generated_at: string;
  base_url: string;
  files: ManifestEntry[];
}

export interface PublishRow {
  id: number;
  launcher_id: string;
  ts: string;
  version: string;
  files: number;
  changed: number;
  ok: boolean;
  message: string;
}

export interface LauncherScan {
  files: FileDiff[];
  git: GitInfo;
  manifest: Manifest | null;
  history: PublishRow[];
  release_dir: string;
  manifest_file: string;
  publishing: boolean;
  error: string;
}

export type Progress =
  | { kind: "step"; index: number; total: number; label: string }
  | { kind: "line"; text: string }
  | { kind: "done"; ok: boolean; message: string };

export interface PublishReport {
  version: string;
  files_published: number;
  files_changed: number;
  committed: boolean;
  pushed: boolean;
  message: string;
}

export interface BugRow {
  id: number;
  server_id: string;
  category: string;
  level: string;
  title: string;
  exception: string;
  details: string;
  count: number;
  first_seen: string;
  last_seen: string;
  resolved: boolean;
}

export interface SuspiciousRow {
  id: number;
  server_id: string;
  ts: string;
  player: string;
  category: string;
  severity: "info" | "warn" | "high";
  title: string;
  details: string;
  acknowledged: boolean;
}

export interface SystemInfo {
  os: string;
  cpu_brand: string;
  cpu_count: number;
  mem_total: number;
}

export interface AppPaths {
  config_file: string;
  history_db: string;
  data_dir: string;
}

export interface SystemOverview {
  info: SystemInfo;
  app_pid: number;
  paths: AppPaths;
  is_windows: boolean;
}

export interface ShutdownStatus {
  plan: PlannedShutdown | null;
  os_armed: boolean;
  seconds_left: number;
}

export interface AppMetrics {
  cpu: number;
  mem: number;
}
