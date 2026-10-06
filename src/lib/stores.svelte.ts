// État global de l'interface (runes Svelte 5) et chargement initial.

import { api, on } from "./api";
import type { AppConfig, BugRow, ServerConfig, ServerState, ShutdownStatus, SuspiciousRow, SystemOverview } from "./types";

export type Page = "dashboard" | "server" | "client" | "launcher" | "system";

export interface Route {
  page: Page;
  id: string;
  tab: string;
}

export interface Toast {
  id: number;
  kind: "info" | "ok" | "warn" | "err";
  message: string;
}

export const app = $state({
  ready: false,
  error: "",
  config: null as AppConfig | null,
  states: {} as Record<string, ServerState>,
  route: { page: "dashboard", id: "", tab: "" } as Route,
  toasts: [] as Toast[],
  shutdown: null as ShutdownStatus | null,
  quitPrompt: null as string[] | null,
  system: null as SystemOverview | null,
  appMetrics: { cpu: 0, mem: 0 },
});

let toastSeq = 0;

export function toast(message: string, kind: Toast["kind"] = "info", ms = 4500): void {
  const id = ++toastSeq;
  app.toasts.push({ id, kind, message });
  setTimeout(() => {
    const i = app.toasts.findIndex((t) => t.id === id);
    if (i >= 0) app.toasts.splice(i, 1);
  }, ms);
}

export function errorToast(e: unknown): void {
  toast(typeof e === "string" ? e : e instanceof Error ? e.message : String(e), "err", 7000);
}

export function navigate(page: Page, id = "", tab = ""): void {
  app.route = { page, id, tab };
  try {
    localStorage.setItem("route", JSON.stringify(app.route));
  } catch {
    // stockage indisponible : on garde seulement l'état en mémoire
  }
}

export function serverCfg(id: string): ServerConfig | undefined {
  return app.config?.servers.find((s) => s.id === id);
}

export function serverName(id: string): string {
  return serverCfg(id)?.name ?? "Serveur";
}

/** Modifie une copie de la config, l'envoie au backend, puis adopte la version sauvegardée. */
export async function saveConfig(mutate: (cfg: AppConfig) => void): Promise<void> {
  if (!app.config) return;
  const copy = structuredClone($state.snapshot(app.config)) as AppConfig;
  mutate(copy);
  app.config = await api.saveConfig(copy);
}

export async function refreshStates(): Promise<void> {
  const list = await api.listServerStates();
  const next: Record<string, ServerState> = {};
  for (const s of list) next[s.id] = s;
  app.states = next;
}

export async function loadAll(): Promise<void> {
  try {
    app.config = await api.getConfig();
    await refreshStates();
    app.shutdown = await api.shutdownStatus();
    app.system = await api.systemInfo();
    app.ready = true;
  } catch (e) {
    app.error = typeof e === "string" ? e : String(e);
  }
}

export async function subscribeEvents(): Promise<void> {
  await on<ServerState>("server:state", (s) => {
    app.states[s.id] = s;
  });
  await on<AppConfig>("config:changed", (c) => {
    app.config = c;
  });
  await on<ShutdownStatus>("shutdown:state", (s) => {
    app.shutdown = s;
  });
  await on<string>("shutdown:result", (err) => {
    if (err) toast(`Extinction : ${err}`, "err", 10000);
    else toast("Extinction de Windows programmée", "warn", 10000);
  });
  await on<{ running: string[] }>("app:quit-requested", (p) => {
    app.quitPrompt = p.running;
  });
  await on<BugRow>("history:bug", (b) => {
    if (b.count === 1 && b.category !== "lag") toast(`${serverName(b.server_id)} : ${b.title}`, "err", 6000);
  });
  await on<SuspiciousRow>("history:suspicious", (s) => {
    toast(`${serverName(s.server_id)} : ${s.title}`, s.severity === "high" ? "err" : "warn", 7000);
  });
  await on<{ cpu: number; mem: number }>("app:metrics", (m) => {
    app.appMetrics = m;
  });
}

export function restoreRoute(): void {
  try {
    const raw = localStorage.getItem("route");
    if (raw) {
      const r = JSON.parse(raw) as Route;
      if (r && r.page) app.route = r;
    }
  } catch {
    // route par défaut
  }
}
