// Backend simulé pour `npm run dev` dans un navigateur : données d'exemple,
// logs et métriques animés. Jamais utilisé dans l'application Tauri.

import type * as T from "./types";

type Handler = (payload: unknown) => void;
const listeners = new Map<string, Set<Handler>>();

function emit(event: string, payload: unknown): void {
  listeners.get(event)?.forEach((h) => h(payload));
}

export async function mockListen(event: string, handler: Handler): Promise<() => void> {
  if (!listeners.has(event)) listeners.set(event, new Set());
  listeners.get(event)!.add(handler);
  return () => listeners.get(event)?.delete(handler);
}

const defaultDetection = (): T.DetectionConfig => ({
  enabled: true,
  gamemode: true,
  give: true,
  op: true,
  teleport: true,
  other_admin: false,
  early_advancements: true,
  early_rules: [
    { title: "Diamonds!", max_minutes: 20 },
    { title: "Cover Me with Diamonds", max_minutes: 60 },
  ],
  mining: true,
  mining_rules: [
    { label: "Diamants", blocks: ["minecraft:diamond_ore", "minecraft:deepslate_diamond_ore"], per_hour: 40 },
    { label: "Débris antiques", blocks: ["minecraft:ancient_debris"], per_hour: 15 },
  ],
  mining_min_playtime_min: 30,
});

const config: T.AppConfig = {
  version: 1,
  settings: {
    close_to_tray: true,
    start_minimized: false,
    metrics_interval_visible_ms: 2000,
    metrics_interval_hidden_ms: 30000,
    ping_interval_s: 30,
    stats_scan_interval_min: 5,
    log_buffer_lines: 2000,
  },
  servers: [
    {
      id: "zevent",
      name: "Zevent (Mineral Contest)",
      kind: {
        type: "local",
        dir: "C:\\TelekGamer\\Zevent",
        java_path: "java",
        jvm_args: "-Xms2G -Xmx6G",
        launch: { mode: "jar", path: "server.jar" },
        extra_args: "nogui",
        port: 25565,
        auto_restart: true,
        stop_timeout_s: 90,
        stop_warning_s: 300,
      },
      loader: "forge",
      mc_version: "1.20.1",
      schedule: { enabled: true, open_time: "06:00", close_time: "22:00", warning_s: 300 },
      detection: { ...defaultDetection(), mining: false, early_advancements: false },
      playit: { enabled: true, exe_path: "C:\\Program Files\\playit_gg\\bin\\playit.exe", args: "", with_server: true },
      notes: "Event Mineral Contest, mod OreClash.",
    },
    {
      id: "adrien",
      name: "Serv Adrien V1",
      kind: {
        type: "remote",
        host: "game4.onpowered.net",
        port: 25582,
        rcon: { enabled: true, host: "game4.onpowered.net", port: 25585, password: "" },
      },
      loader: "fabric",
      mc_version: "26.1.2",
      schedule: { enabled: false, open_time: "06:00", close_time: "22:00", warning_s: 300 },
      detection: defaultDetection(),
      playit: { enabled: false, exe_path: "", args: "", with_server: true },
      notes: "Hébergé chez onpowered.",
    },
    {
      id: "aero",
      name: "Create Aeronautics",
      kind: {
        type: "local",
        dir: "C:\\TelekGamer\\Aeronautics",
        java_path: "C:\\Program Files\\Java\\jdk-21\\bin\\java.exe",
        jvm_args: "-Xms4G -Xmx12G",
        launch: { mode: "argsfile", path: "libraries/net/neoforged/neoforge/21.1.72/win_args.txt" },
        extra_args: "nogui",
        port: 25566,
        auto_restart: false,
        stop_timeout_s: 120,
        stop_warning_s: 300,
      },
      loader: "neoforge",
      mc_version: "1.21.1",
      schedule: { enabled: false, open_time: "06:00", close_time: "22:00", warning_s: 300 },
      detection: defaultDetection(),
      playit: { enabled: false, exe_path: "", args: "", with_server: true },
      notes: "",
    },
  ],
  instances: [
    { id: "i1", name: "Create Aeronautics", dir: "C:\\Users\\Ilan\\curseforge\\minecraft\\Instances\\Create Aeronautics", source: "curseforge", loader: "neoforge", mc_version: "1.21.1" },
  ],
  launchers: [
    {
      id: "l1",
      name: "Launcher Create",
      project_dir: "C:\\TelekGamer\\Launcher Create",
      release_dir: "release files",
      dist_dir: "dist release",
      manifest_path: "manifest.json",
      branch: "main",
      base_url: "https://raw.githubusercontent.com/ilanwouters26-max/launcher-create/main/release files",
      commit_message: "Publication modpack {date}",
      excluded: ["mods/optional-shaders.jar"],
      ignore_suffixes: [".disabled", ".bak"],
    },
    {
      id: "l2",
      name: "Launcher Adrien",
      project_dir: "C:\\TelekGamer\\Launcher Adrien",
      release_dir: "release files",
      dist_dir: "dist release",
      manifest_path: "manifest.json",
      branch: "main",
      base_url: "",
      commit_message: "Publication modpack {date}",
      excluded: [],
      ignore_suffixes: [".disabled"],
    },
  ],
  shutdown: null,
};

const states: Record<string, T.ServerState> = {};
const logs: Record<string, T.LogLine[]> = {};
const timers: Record<string, number> = {};
let bugSeq = 10;
let susSeq = 10;

const bugs: T.BugRow[] = [
  { id: 1, server_id: "zevent", category: "error", level: "error", title: "Exception ticking world", exception: "NullPointerException", details: "[21:03:11] [Server thread/ERROR]: Exception ticking world\njava.lang.NullPointerException: Cannot invoke \"net.minecraft.world.entity.Entity.level()\"\n\tat net.minecraft.world.level.Level.tick(Level.java:120)\n\tat net.minecraft.server.MinecraftServer.tickChildren(MinecraftServer.java:980)", count: 7, first_seen: "2026-10-05T21:03:11+02:00", last_seen: "2026-10-06T19:41:02+02:00", resolved: false },
  { id: 2, server_id: "zevent", category: "lag", level: "warn", title: "Le serveur ne suit plus (Can't keep up)", exception: "", details: "2534 ms de retard, 50 ticks", count: 42, first_seen: "2026-10-04T18:00:00+02:00", last_seen: "2026-10-06T20:12:40+02:00", resolved: false },
  { id: 3, server_id: "aero", category: "crash", level: "fatal", title: "Crash du serveur", exception: "", details: "Rapport : C:\\TelekGamer\\Aeronautics\\crash-reports\\crash-2026-10-05_22.14.03-server.txt", count: 1, first_seen: "2026-10-05T22:14:03+02:00", last_seen: "2026-10-05T22:14:03+02:00", resolved: true },
];

const suspicious: T.SuspiciousRow[] = [
  { id: 1, server_id: "zevent", ts: "2026-10-06T20:02:10+02:00", player: "Kevin_Mine", category: "gamemode", severity: "high", title: "Kevin_Mine a passé lui-même en Creative", details: "Set own game mode to Creative Mode", acknowledged: false },
  { id: 2, server_id: "zevent", ts: "2026-10-06T19:48:33+02:00", player: "xXDiamondXx", category: "early_advancement", severity: "warn", title: "xXDiamondXx : « Diamonds! » 6 min après sa première connexion", details: "Seuil : 20 min.", acknowledged: false },
  { id: 3, server_id: "aero", ts: "2026-10-05T21:10:00+02:00", player: "Thomas", category: "give", severity: "high", title: "Thomas s'est give 64 × Netherite Ingot (pour Thomas)", details: "Gave 64 [Netherite Ingot] to Thomas", acknowledged: true },
];

function initState(s: T.ServerConfig): T.ServerState {
  return {
    id: s.id,
    status: s.kind.type === "remote" ? "running" : "stopped",
    remote: s.kind.type === "remote",
    players: s.kind.type === "remote" ? [{ name: "Adrien", since: new Date(Date.now() - 3600e3).toISOString() }, { name: "Lola", since: new Date(Date.now() - 600e3).toISOString() }] : [],
    players_max: 20,
    uptime_s: 0,
    pid: null,
    start_progress: 0,
    last_ping: s.kind.type === "remote" ? { online: true, players_online: 2, players_max: 20, version: "Fabric 26.1.2", protocol: 800, motd: "Serv Adrien V1", latency_ms: 31, sample: ["Adrien", "Lola"], error: "" } : null,
    counts: { bugs_open: bugs.filter((b) => b.server_id === s.id && !b.resolved).length, suspicious_open: suspicious.filter((x) => x.server_id === s.id && !x.acknowledged).length },
    next_event: s.schedule.enabled ? { kind: "close", at: new Date(Date.now() + 5400e3).toISOString(), in_seconds: 5400 } : null,
    playit_running: false,
    last_error: "",
    log_lines: 0,
  };
}

for (const s of config.servers) {
  states[s.id] = initState(s);
  logs[s.id] = [];
}

function line(level: T.LogLevel, message: string, thread = "Server thread"): T.LogLine {
  const ts = new Date().toLocaleTimeString("fr-FR", { hour12: false });
  return { ts, level, thread, logger: "", message, raw: `[${ts}] [${thread}/${level.toUpperCase()}]: ${message}`, continuation: false };
}

function pushLog(id: string, lines: T.LogLine[]): void {
  logs[id].push(...lines);
  if (logs[id].length > 2000) logs[id].splice(0, logs[id].length - 2000);
  states[id].log_lines = logs[id].length;
  emit("server:log", { server_id: id, lines });
}

function emitState(id: string): void {
  emit("server:state", { ...states[id] });
}

const chatter = [
  () => line("info", "<Kevin_Mine> quelqu'un a vu la base de Lola ?"),
  () => line("info", "xXDiamondXx has made the advancement [Diamonds!]"),
  () => line("warn", "Can't keep up! Is the server overloaded? Running 2140ms or 42 ticks behind"),
  () => line("info", "<Lola> gg"),
  () => line("info", "Thomas[/192.168.1.24:51233] logged in with entity id 433 at (120.5, 64.0, -88.2)"),
  () => line("info", "[Kevin_Mine: Set own game mode to Creative Mode]"),
  () => line("info", "Saving the game (this may take a moment!)"),
  () => line("info", "Saved the game"),
];

function startServer(id: string): void {
  const st = states[id];
  if (st.status === "running" || st.status === "starting") throw "Le serveur est déjà lancé";
  st.status = "starting";
  st.pid = 18422;
  st.start_progress = 0;
  st.last_error = "";
  emitState(id);
  pushLog(id, [line("info", "Lancement : java -Xmx6G -jar server.jar nogui", "panel"), line("info", "Starting minecraft server version 1.20.1"), line("info", "Loading properties"), line("info", 'Preparing level "world"')]);
  let pct = 0;
  const started = Date.now();
  timers[id] = window.setInterval(() => {
    if (st.status === "starting") {
      pct += 25;
      if (pct < 100) {
        st.start_progress = pct;
        pushLog(id, [line("info", `Preparing spawn area: ${pct}%`)]);
        emitState(id);
      } else {
        st.status = "running";
        st.start_progress = 100;
        st.uptime_s = 0;
        pushLog(id, [line("info", "Done (4.812s)! For help, type \"help\""), line("info", "RCON running on 0.0.0.0:25575", "RCON Listener #1")]);
        emitState(id);
      }
    } else if (st.status === "running") {
      st.uptime_s = Math.floor((Date.now() - started) / 1000);
      const l = chatter[Math.floor(Math.random() * chatter.length)]();
      pushLog(id, [l]);
      if (l.message.includes("logged in") && !st.players.some((p) => p.name === "Thomas")) st.players.push({ name: "Thomas", since: new Date().toISOString() });
      if (l.message.includes("[Kevin_Mine:")) {
        const row: T.SuspiciousRow = { id: ++susSeq, server_id: id, ts: new Date().toISOString(), player: "Kevin_Mine", category: "gamemode", severity: "high", title: "Kevin_Mine a passé lui-même en Creative", details: l.message, acknowledged: false };
        suspicious.unshift(row);
        st.counts.suspicious_open++;
        emit("history:suspicious", row);
      }
      if (l.level === "warn") {
        const b = bugs.find((x) => x.server_id === id && x.category === "lag");
        if (b) {
          b.count++;
          b.last_seen = new Date().toISOString();
          emit("history:bug", { ...b });
        }
      }
      emitState(id);
      emit("server:metrics", { server_id: id, point: metricPoint(true) });
    }
  }, 1800);
}

function stopServer(id: string): void {
  const st = states[id];
  if (st.status !== "running" && st.status !== "starting") throw "Serveur non lancé";
  st.status = "stopping";
  emitState(id);
  pushLog(id, [line("info", "Arrêt demandé (save-all puis stop)", "panel"), line("info", "Stopping the server"), line("info", "Saving players"), line("info", "Saving worlds")]);
  window.clearInterval(timers[id]);
  window.setTimeout(() => {
    st.status = "stopped";
    st.pid = null;
    st.players = [];
    st.uptime_s = 0;
    pushLog(id, [line("info", "Serveur arrêté", "panel")]);
    emitState(id);
  }, 1200);
}

function metricPoint(running: boolean): T.MetricPoint {
  return {
    ts: Date.now(),
    cpu: running ? 8 + Math.random() * 14 : 0,
    mem: running ? 5.1e9 + Math.random() * 4e8 : 0,
    sys_cpu: 12 + Math.random() * 10,
    sys_mem_used: 17.2e9 + Math.random() * 5e8,
    sys_mem_total: 32e9,
    rx_bps: 42e3 + Math.random() * 30e3,
    tx_bps: 180e3 + Math.random() * 60e3,
  };
}

const sampleMods: T.ModInfo[] = [
  { file_name: "create-1.21.1-6.0.4.jar", path: "C:\\x\\mods\\create-1.21.1-6.0.4.jar", id: "create", name: "Create", version: "6.0.4", loader: "neoforge", environment: "", description: "Technology that empowers the player", size: 15_400_000, enabled: true, modified: 1759700000 },
  { file_name: "create_aeronautics-0.9.2.jar", path: "C:\\x\\mods\\create_aeronautics-0.9.2.jar", id: "create_aeronautics", name: "Create: Aeronautics", version: "0.9.2", loader: "neoforge", environment: "", description: "Airships for Create", size: 8_100_000, enabled: true, modified: 1759700000 },
  { file_name: "sodium-neoforge-0.6.13.jar", path: "C:\\x\\mods\\sodium-neoforge-0.6.13.jar", id: "sodium", name: "Sodium", version: "0.6.13", loader: "neoforge", environment: "client", description: "Rendering engine", size: 1_200_000, enabled: true, modified: 1759700000 },
  { file_name: "jei-1.21.1-19.21.0.247.jar", path: "C:\\x\\mods\\jei-1.21.1-19.21.0.247.jar", id: "jei", name: "Just Enough Items", version: "19.21.0.247", loader: "neoforge", environment: "", description: "Recipe viewer", size: 2_300_000, enabled: true, modified: 1759700000 },
  { file_name: "xaeros_minimap_25.2.10.jar.disabled", path: "C:\\x\\mods\\xaeros_minimap_25.2.10.jar.disabled", id: "xaerominimap", name: "Xaero's Minimap", version: "25.2.10", loader: "neoforge", environment: "client", description: "", size: 3_000_000, enabled: false, modified: 1759600000 },
  { file_name: "jei-1.21.1-19.20.0.240.jar", path: "C:\\x\\mods\\jei-1.21.1-19.20.0.240.jar", id: "jei", name: "Just Enough Items", version: "19.20.0.240", loader: "neoforge", environment: "", description: "Recipe viewer", size: 2_280_000, enabled: true, modified: 1759500000 },
];

let publishTimer = 0;

export async function mockInvoke(cmd: string, args: Record<string, unknown>): Promise<unknown> {
  await new Promise((r) => setTimeout(r, 60));
  const id = args.id as string;
  switch (cmd) {
    case "get_config":
      return structuredClone(config);
    case "get_paths":
      return { config_file: "C:\\Users\\Ilan\\AppData\\Roaming\\com.telekgamer.panel\\config.json", history_db: "C:\\Users\\Ilan\\AppData\\Roaming\\com.telekgamer.panel\\history.db", data_dir: "C:\\Users\\Ilan\\AppData\\Roaming\\com.telekgamer.panel" };
    case "save_config": {
      const incoming = args.config as T.AppConfig;
      Object.assign(config, incoming, { shutdown: config.shutdown });
      for (const s of config.servers) {
        if (!states[s.id]) {
          states[s.id] = initState(s);
          logs[s.id] = [];
        }
      }
      emit("config:changed", structuredClone(config));
      return structuredClone(config);
    }
    case "list_server_states":
      return Object.values(states).map((s) => ({ ...s }));
    case "server_start":
      startServer(id);
      return null;
    case "server_stop":
      stopServer(id);
      return null;
    case "server_restart":
      stopServer(id);
      window.setTimeout(() => startServer(id), 1500);
      return null;
    case "server_kill":
      window.clearInterval(timers[id]);
      states[id].status = "stopped";
      states[id].players = [];
      emitState(id);
      return null;
    case "server_command": {
      const command = String(args.command).replace(/^\//, "");
      pushLog(id, [line("info", `> ${command}`, "console")]);
      if (states[id].remote) {
        const response = command.startsWith("list") ? "There are 2 of a max of 20 players online: Adrien, Lola" : `Commande « ${command} » exécutée`;
        pushLog(id, [line("info", response, "rcon")]);
        return response;
      }
      if (command === "list") pushLog(id, [line("info", `There are ${states[id].players.length} of a max of 20 players online: ${states[id].players.map((p) => p.name).join(", ")}`)]);
      else if (command.startsWith("say ")) pushLog(id, [line("info", `[Server] ${command.slice(4)}`)]);
      else pushLog(id, [line("info", `Commande « ${command} » exécutée`)]);
      return "";
    }
    case "server_log":
      return [...(logs[id] ?? [])];
    case "server_clear_log":
      logs[id] = [];
      return null;
    case "server_ping":
      return states[id].last_ping ?? { online: states[id].status === "running", players_online: states[id].players.length, players_max: 20, version: "1.20.1", protocol: 763, motd: "Zevent", latency_ms: 2, sample: [], error: states[id].status === "running" ? "" : "Connexion refusée" };
    case "server_detect_launch":
      return { launch: { mode: "argsfile", path: "libraries/net/neoforged/neoforge/21.1.72/win_args.txt" }, loader: "neoforge", mc_version: "1.21.1", jvm_args: "-Xmx12G -Xms4G", notes: [] };
    case "server_players":
      return [
        { name: "Kevin_Mine", first_seen: "2026-10-01T18:02:00+02:00", last_seen: "2026-10-06T20:02:10+02:00", sessions: 14, total_minutes: 1240 },
        { name: "Lola", first_seen: "2026-10-02T19:00:00+02:00", last_seen: "2026-10-06T19:50:00+02:00", sessions: 9, total_minutes: 610 },
        { name: "xXDiamondXx", first_seen: "2026-10-06T19:42:00+02:00", last_seen: "2026-10-06T19:48:33+02:00", sessions: 1, total_minutes: 7 },
      ];
    case "server_scan_stats":
      return {
        stats: [
          { uuid: "a", name: "Kevin_Mine", play_hours: 20.6, mined: { "minecraft:diamond_ore": 44, "minecraft:deepslate_diamond_ore": 310 }, deaths: 12, player_kills: 3 },
          { uuid: "b", name: "Lola", play_hours: 10.1, mined: { "minecraft:deepslate_diamond_ore": 90 }, deaths: 4, player_kills: 0 },
          { uuid: "c", name: "xXDiamondXx", play_hours: 0.9, mined: { "minecraft:deepslate_diamond_ore": 120 }, deaths: 0, player_kills: 0 },
        ],
        alerts: [{ player: "xXDiamondXx", rule: "Diamants", count: 120, per_hour: 133.3, threshold: 40, play_hours: 0.9 }],
        recorded: 1,
      };
    case "server_properties":
      return { "level-name": "world", "server-port": "25565", motd: "Zevent", "max-players": "20", "enable-rcon": "false", "view-distance": "10" };
    case "metrics_watch":
      return null;
    case "metrics_history": {
      const running = states[id]?.status === "running";
      return Array.from({ length: 60 }, (_, i) => ({ ...metricPoint(running), ts: Date.now() - (60 - i) * 2000 }));
    }
    case "playit_start":
      states[id].playit_running = true;
      pushLog(id, [line("info", "agent lancé", "playit"), line("info", "tunnel udp+tcp 25565 → zevent.playit.gg", "playit")]);
      emitState(id);
      return null;
    case "playit_stop":
      states[id].playit_running = false;
      emitState(id);
      return null;
    case "mods_list": {
      const dir = String(args.dir);
      const mods = sampleMods.map((m) => ({ ...m, path: `${dir}\\${m.file_name}` }));
      return { dir, dir_exists: true, mods, duplicates: [{ key: "jei", files: ["jei-1.21.1-19.21.0.247.jar", "jei-1.21.1-19.20.0.240.jar"] }] };
    }
    case "mods_set_enabled": {
      const path = String(args.path);
      const enabled = Boolean(args.enabled);
      const m = sampleMods.find((x) => path.endsWith(x.file_name));
      if (m) {
        m.enabled = enabled;
        m.file_name = enabled ? m.file_name.replace(/\.disabled$/, "") : `${m.file_name}.disabled`;
      }
      return enabled ? path.replace(/\.disabled$/, "") : `${path}.disabled`;
    }
    case "mods_delete": {
      const i = sampleMods.findIndex((x) => String(args.path).endsWith(x.file_name));
      if (i >= 0) sampleMods.splice(i, 1);
      return null;
    }
    case "mods_import":
      return (args.files as string[]).map((f) => ({ file: f.split(/[\\/]/).pop() ?? f, ok: f.endsWith(".jar"), message: f.endsWith(".jar") ? "Copié" : "Pas un .jar, ignoré" }));
    case "mods_diff":
      return { only_a: [sampleMods[2], sampleMods[4]], only_b: [], version_mismatch: [[sampleMods[3], { ...sampleMods[3], version: "19.20.0.240", file_name: "jei-1.21.1-19.20.0.240.jar" }]], same: 3 };
    case "open_path":
      return null;
    case "instances_detect":
      return [
        { source: "curseforge", name: "Create Aeronautics", dir: "C:\\Users\\Ilan\\curseforge\\minecraft\\Instances\\Create Aeronautics", mc_version: "1.21.1", loader: "neoforge", mods_count: 184 },
        { source: "curseforge", name: "Serv Adrien", dir: "C:\\Users\\Ilan\\curseforge\\minecraft\\Instances\\Serv Adrien", mc_version: "26.1.2", loader: "fabric", mods_count: 12 },
        { source: "official", name: "Minecraft Launcher (.minecraft)", dir: "C:\\Users\\Ilan\\AppData\\Roaming\\.minecraft", mc_version: "", loader: "", mods_count: 0 },
      ];
    case "instance_inspect":
      return { source: "manual", name: String(args.dir).split(/[\\/]/).pop(), dir: args.dir, mc_version: "", loader: "", mods_count: 3 };
    case "launcher_scan":
      return {
        files: [
          { path: "mods/create-1.21.1-6.0.4.jar", name: "create-1.21.1-6.0.4.jar", size: 15_400_000, sha1: "7e240de74fb1ed08fa08d38063f6a6a91462a815", status: "changed" },
          { path: "mods/create_aeronautics-0.9.2.jar", name: "create_aeronautics-0.9.2.jar", size: 8_100_000, sha1: "a9993e364706816aba3e25717850c26c9cd0d89d", status: "new" },
          { path: "mods/jei-1.21.1-19.21.0.247.jar", name: "jei-1.21.1-19.21.0.247.jar", size: 2_300_000, sha1: "bbb", status: "unchanged" },
          { path: "mods/optional-shaders.jar", name: "optional-shaders.jar", size: 900_000, sha1: "ccc", status: "unchanged" },
          { path: "config/create-common.toml", name: "create-common.toml", size: 4_200, sha1: "ddd", status: "unchanged" },
          { path: "mods/old-mod-1.0.jar", name: "old-mod-1.0.jar", size: 300_000, sha1: "eee", status: "removed" },
        ],
        git: { available: true, is_repo: true, branch: "main", last_commit: "a1b2c3d Publication modpack 05/10/2026 (il y a 1 jour)", remote: "https://github.com/ilanwouters26-max/launcher-create.git", dirty_count: 2, error: "" },
        manifest: { version: "2026.10.05-2130", generated_at: "2026-10-05T21:30:00+02:00", base_url: "https://raw.githubusercontent.com/ilanwouters26-max/launcher-create/main/release%20files", files: [] },
        history: [
          { id: 2, launcher_id: "l1", ts: "2026-10-05T21:30:00+02:00", version: "2026.10.05-2130", files: 185, changed: 3, ok: true, message: "Modpack publié sur GitHub" },
          { id: 1, launcher_id: "l1", ts: "2026-10-01T18:12:00+02:00", version: "2026.10.01-1812", files: 182, changed: 12, ok: false, message: "git push a échoué (code 128) : vérifie les identifiants et la branche" },
        ],
        release_dir: "C:\\TelekGamer\\Launcher Create\\release files",
        manifest_file: "C:\\TelekGamer\\Launcher Create\\manifest.json",
        publishing: publishTimer !== 0,
        error: "",
      };
    case "launcher_publish": {
      const launcher_id = id;
      const steps = ["Analyse des fichiers de release", "Écriture du manifest", "git add", "git commit", "git push"];
      const lines = [["185 fichiers, 2 modifiés depuis le dernier manifest"], ["184 entrées → manifest.json"], [], ["[main 9f8e7d6] Publication modpack 06/10/2026 21:14", " 3 files changed, 12 insertions(+), 4 deletions(-)"], ["Enumerating objects: 9, done.", "Counting objects: 100% (9/9), done.", "Compressing objects: 100% (5/5), done.", "Writing objects: 100% (5/5), 15.2 MiB | 4.1 MiB/s, done.", "To https://github.com/ilanwouters26-max/launcher-create.git", "   a1b2c3d..9f8e7d6  main -> main"]];
      let i = 0;
      publishTimer = window.setInterval(() => {
        if (i < steps.length) {
          emit("publish:progress", { launcher_id, progress: { kind: "step", index: i + 1, total: steps.length, label: steps[i] } });
          for (const l of lines[i]) emit("publish:progress", { launcher_id, progress: { kind: "line", text: l } });
          i++;
        } else {
          window.clearInterval(publishTimer);
          publishTimer = 0;
          emit("publish:progress", { launcher_id, progress: { kind: "done", ok: true, message: "Modpack publié sur GitHub" } });
          emit("publish:done", { launcher_id, report: { version: "2026.10.06-2114", files_published: 184, files_changed: 2, committed: true, pushed: true, message: "Modpack publié sur GitHub" }, error: "" });
        }
      }, 700);
      return null;
    }
    case "bugs_list":
      return bugs.filter((b) => (!args.server_id || b.server_id === args.server_id) && (args.include_resolved || !b.resolved)).map((b) => ({ ...b }));
    case "bug_resolve": {
      const b = bugs.find((x) => x.id === args.id);
      if (b) b.resolved = Boolean(args.resolved);
      return null;
    }
    case "bug_delete": {
      const i = bugs.findIndex((x) => x.id === args.id);
      if (i >= 0) bugs.splice(i, 1);
      return null;
    }
    case "suspicious_list":
      return suspicious.filter((s) => !args.server_id || s.server_id === args.server_id).map((s) => ({ ...s }));
    case "suspicious_ack": {
      const s = suspicious.find((x) => x.id === args.id);
      if (s) s.acknowledged = Boolean(args.ack);
      return null;
    }
    case "suspicious_delete": {
      const i = suspicious.findIndex((x) => x.id === args.id);
      if (i >= 0) suspicious.splice(i, 1);
      return null;
    }
    case "history_clear":
      return null;
    case "crash_report_read":
      return "---- Minecraft Crash Report ----\n// Why did you do that?\n\nTime: 2026-10-05 22:14:03\nDescription: Exception in server tick loop\n\njava.lang.IllegalStateException: Ticking entity\n\tat net.minecraft.server.MinecraftServer.tickChildren(MinecraftServer.java:980)";
    case "system_info":
      return { info: { os: "Windows 11 (24H2)", cpu_brand: "13th Gen Intel(R) Core(TM) i7-13700", cpu_count: 24, mem_total: 32e9 }, app_pid: 4242, paths: { config_file: "C:\\Users\\Ilan\\AppData\\Roaming\\com.telekgamer.panel\\config.json", history_db: "C:\\Users\\Ilan\\AppData\\Roaming\\com.telekgamer.panel\\history.db", data_dir: "C:\\Users\\Ilan\\AppData\\Roaming\\com.telekgamer.panel" }, is_windows: true };
    case "shutdown_plan": {
      const plan = { ...(args.plan as T.PlannedShutdown), created_at: new Date().toISOString() };
      config.shutdown = plan;
      const status = { plan, os_armed: false, seconds_left: Math.round((new Date(plan.at).getTime() - Date.now()) / 1000) };
      emit("shutdown:state", status);
      return status;
    }
    case "shutdown_cancel":
      config.shutdown = null;
      emit("shutdown:state", { plan: null, os_armed: false, seconds_left: 0 });
      return { plan: null, os_armed: false, seconds_left: 0 };
    case "shutdown_status":
      return { plan: config.shutdown, os_armed: false, seconds_left: config.shutdown ? Math.round((new Date(config.shutdown.at).getTime() - Date.now()) / 1000) : 0 };
    case "running_servers":
      return Object.entries(states).filter(([, s]) => !s.remote && s.status === "running").map(([k]) => config.servers.find((c) => c.id === k)?.name ?? k);
    case "app_quit":
      return null;
    default:
      throw `Commande inconnue (mock) : ${cmd}`;
  }
}

window.setInterval(() => emit("app:metrics", { cpu: 0.3 + Math.random() * 0.4, mem: 48e6 + Math.random() * 4e6 }), 3000);
