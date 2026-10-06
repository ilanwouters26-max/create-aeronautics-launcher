// Couche d'accès au backend : vraies commandes Tauri dans l'app, mock dans un navigateur.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type * as T from "./types";
import { mockInvoke, mockListen } from "./mock";

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export type Unlisten = () => void;

export async function call<R>(cmd: string, args?: Record<string, unknown>): Promise<R> {
  if (isTauri) return invoke<R>(cmd, args);
  return mockInvoke(cmd, args ?? {}) as Promise<R>;
}

export async function on<P>(event: string, handler: (payload: P) => void): Promise<Unlisten> {
  if (isTauri) return listen<P>(event, (e) => handler(e.payload));
  return mockListen(event, handler as (p: unknown) => void);
}

export const api = {
  getConfig: () => call<T.AppConfig>("get_config"),
  getPaths: () => call<T.AppPaths>("get_paths"),
  saveConfig: (config: T.AppConfig) => call<T.AppConfig>("save_config", { config }),

  listServerStates: () => call<T.ServerState[]>("list_server_states"),
  serverStart: (id: string) => call<void>("server_start", { id }),
  serverStop: (id: string) => call<void>("server_stop", { id }),
  serverRestart: (id: string) => call<void>("server_restart", { id }),
  serverKill: (id: string) => call<void>("server_kill", { id }),
  serverCommand: (id: string, command: string) => call<string>("server_command", { id, command }),
  serverLog: (id: string) => call<T.LogLine[]>("server_log", { id }),
  serverClearLog: (id: string) => call<void>("server_clear_log", { id }),
  serverPing: (id: string) => call<T.PingResult>("server_ping", { id }),
  serverDetectLaunch: (dir: string) => call<T.DetectedLaunch>("server_detect_launch", { dir }),
  serverPlayers: (id: string) => call<T.PlayerRow[]>("server_players", { id }),
  serverScanStats: (id: string) => call<T.StatsScan>("server_scan_stats", { id }),
  serverProperties: (id: string) => call<Record<string, string>>("server_properties", { id }),
  metricsWatch: (id: string, watch: boolean) => call<void>("metrics_watch", { id, watch }),
  metricsHistory: (id: string) => call<T.MetricPoint[]>("metrics_history", { id }),
  playitStart: (id: string) => call<void>("playit_start", { id }),
  playitStop: (id: string) => call<void>("playit_stop", { id }),

  modsList: (dir: string) => call<T.ModsListing>("mods_list", { dir }),
  modsSetEnabled: (path: string, enabled: boolean) => call<string>("mods_set_enabled", { path, enabled }),
  modsDelete: (path: string) => call<void>("mods_delete", { path }),
  modsImport: (dir: string, files: string[]) => call<T.ImportResult[]>("mods_import", { dir, files }),
  modsDiff: (dir_a: string, dir_b: string) => call<T.ModDiff>("mods_diff", { dir_a, dir_b }),
  openPath: (path: string) => call<void>("open_path", { path }),
  instancesDetect: () => call<T.DetectedInstance[]>("instances_detect"),
  instanceInspect: (dir: string) => call<T.DetectedInstance>("instance_inspect", { dir }),

  launcherScan: (id: string) => call<T.LauncherScan>("launcher_scan", { id }),
  launcherPublish: (id: string, version: string | null, push: boolean) => call<void>("launcher_publish", { id, version, push }),

  bugsList: (server_id: string | null, include_resolved: boolean) => call<T.BugRow[]>("bugs_list", { server_id, include_resolved }),
  bugResolve: (id: number, server_id: string, resolved: boolean) => call<void>("bug_resolve", { id, server_id, resolved }),
  bugDelete: (id: number, server_id: string) => call<void>("bug_delete", { id, server_id }),
  suspiciousList: (server_id: string | null) => call<T.SuspiciousRow[]>("suspicious_list", { server_id }),
  suspiciousAck: (id: number, server_id: string, ack: boolean) => call<void>("suspicious_ack", { id, server_id, ack }),
  suspiciousDelete: (id: number, server_id: string) => call<void>("suspicious_delete", { id, server_id }),
  historyClear: (server_id: string, bugs: boolean, suspicious: boolean) => call<void>("history_clear", { server_id, bugs, suspicious }),
  crashReportRead: (path: string) => call<string>("crash_report_read", { path }),

  systemInfo: () => call<T.SystemOverview>("system_info"),
  shutdownPlan: (plan: T.PlannedShutdown) => call<T.ShutdownStatus>("shutdown_plan", { plan }),
  shutdownCancel: () => call<T.ShutdownStatus>("shutdown_cancel"),
  shutdownStatus: () => call<T.ShutdownStatus>("shutdown_status"),
  runningServers: () => call<string[]>("running_servers"),
  appQuit: (stop_servers: boolean) => call<void>("app_quit", { stop_servers }),

  async pickFolder(title: string): Promise<string | null> {
    if (!isTauri) return window.prompt(`${title}\nChemin du dossier :`) || null;
    const { open } = await import("@tauri-apps/plugin-dialog");
    const r = await open({ directory: true, multiple: false, title });
    return typeof r === "string" ? r : null;
  },

  async pickFile(title: string, extensions: string[]): Promise<string | null> {
    if (!isTauri) return window.prompt(`${title}\nChemin du fichier :`) || null;
    const { open } = await import("@tauri-apps/plugin-dialog");
    const r = await open({ directory: false, multiple: false, title, filters: [{ name: title, extensions }] });
    return typeof r === "string" ? r : null;
  },

  async pickJars(title: string): Promise<string[]> {
    if (!isTauri) {
      const v = window.prompt(`${title}\nChemins des .jar séparés par ;`);
      return v ? v.split(";").map((s) => s.trim()).filter(Boolean) : [];
    }
    const { open } = await import("@tauri-apps/plugin-dialog");
    const r = await open({ directory: false, multiple: true, title, filters: [{ name: "Mods", extensions: ["jar"] }] });
    if (!r) return [];
    return Array.isArray(r) ? r : [r];
  },

  async autostartEnabled(): Promise<boolean> {
    if (!isTauri) return false;
    const { isEnabled } = await import("@tauri-apps/plugin-autostart");
    return isEnabled();
  },

  async setAutostart(enabled: boolean): Promise<void> {
    if (!isTauri) return;
    const { enable, disable } = await import("@tauri-apps/plugin-autostart");
    if (enabled) await enable();
    else await disable();
  },

  async hideWindow(): Promise<void> {
    if (!isTauri) return;
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().hide();
  },

  /** Glisser-déposer natif : chemins complets des fichiers déposés sur la fenêtre. */
  async onDragDrop(handler: (ev: { type: "enter" | "over" | "drop" | "leave"; paths: string[]; x: number; y: number }) => void): Promise<Unlisten> {
    if (!isTauri) return () => {};
    const { getCurrentWebview } = await import("@tauri-apps/api/webview");
    return getCurrentWebview().onDragDropEvent((event) => {
      const p = event.payload;
      const scale = window.devicePixelRatio || 1;
      if (p.type === "leave") {
        handler({ type: "leave", paths: [], x: 0, y: 0 });
      } else {
        handler({
          type: p.type,
          paths: p.type === "over" ? [] : p.paths,
          x: p.position.x / scale,
          y: p.position.y / scale,
        });
      }
    });
  },
};
