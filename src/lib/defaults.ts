// Valeurs par défaut côté interface, alignées sur celles du cœur Rust.

import type { DetectionConfig, InstanceConfig, LauncherConfig, ServerConfig } from "./types";

export function newId(): string {
  return Date.now().toString(16) + Math.floor(Math.random() * 0xffff).toString(16).padStart(4, "0");
}

export function defaultDetection(): DetectionConfig {
  return {
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
      { title: "Cover Me in Debris", max_minutes: 180 },
    ],
    mining: true,
    mining_rules: [
      { label: "Diamants", blocks: ["minecraft:diamond_ore", "minecraft:deepslate_diamond_ore"], per_hour: 40 },
      { label: "Débris antiques", blocks: ["minecraft:ancient_debris"], per_hour: 15 },
    ],
    mining_min_playtime_min: 30,
  };
}

export function miningEventDetection(): DetectionConfig {
  return { ...defaultDetection(), early_advancements: false, mining: false };
}

export function defaultServer(): ServerConfig {
  return {
    id: newId(),
    name: "Nouveau serveur",
    kind: {
      type: "local",
      dir: "",
      java_path: "java",
      jvm_args: "-Xms2G -Xmx6G -Dfile.encoding=UTF-8 -Dsun.stdout.encoding=UTF-8 -Dstdout.encoding=UTF-8",
      launch: { mode: "jar", path: "server.jar" },
      extra_args: "nogui",
      port: 25565,
      auto_restart: false,
      stop_timeout_s: 90,
      stop_warning_s: 300,
    },
    loader: "",
    mc_version: "",
    schedule: { enabled: false, open_time: "06:00", close_time: "22:00", warning_s: 300 },
    detection: defaultDetection(),
    playit: { enabled: false, exe_path: "", args: "", with_server: true },
    notes: "",
  };
}

export function defaultRemoteKind() {
  return { type: "remote" as const, host: "", port: 25565, rcon: { enabled: false, host: "", port: 25575, password: "" } };
}

export function defaultLocalKind() {
  const s = defaultServer();
  return s.kind;
}

export function defaultLauncher(): LauncherConfig {
  return {
    id: newId(),
    name: "Launcher",
    project_dir: "",
    release_dir: "release files",
    dist_dir: "dist release",
    manifest_path: "manifest.json",
    branch: "main",
    base_url: "",
    commit_message: "Publication modpack {date}",
    excluded: [],
    ignore_suffixes: [".disabled", ".bak", ".tmp"],
  };
}

export function defaultInstance(): InstanceConfig {
  return { id: newId(), name: "", dir: "", source: "manual", loader: "", mc_version: "" };
}

export const LOADERS = ["vanilla", "fabric", "forge", "neoforge", "quilt", "paper"];
