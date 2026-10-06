<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../../components/Icon.svelte";
  import Sparkline from "../../components/Sparkline.svelte";
  import { api, on } from "../../lib/api";
  import { bitrate, bytes, percent } from "../../lib/format";
  import { app, errorToast } from "../../lib/stores.svelte";
  import type { MetricPoint, ServerConfig } from "../../lib/types";

  let { id, cfg }: { id: string; cfg: ServerConfig } = $props();

  let points = $state<MetricPoint[]>([]);
  let serverProps = $state<Record<string, string> | null>(null);

  const st = $derived(app.states[id]);
  const running = $derived(st?.status === "running" || st?.status === "starting");
  const last = $derived(points.at(-1));
  const xmx = $derived.by(() => {
    if (cfg.kind.type !== "local") return 0;
    const m = cfg.kind.jvm_args.match(/-Xmx(\d+)([gGmM])/);
    if (!m) return 0;
    const n = parseInt(m[1], 10);
    return m[2].toLowerCase() === "g" ? n * 1024 ** 3 : n * 1024 ** 2;
  });

  onMount(() => {
    api.metricsWatch(id, true).catch(() => {});
    api.metricsHistory(id).then((p) => (points = p)).catch(errorToast);
    if (cfg.kind.type === "local") api.serverProperties(id).then((p) => (serverProps = p)).catch(() => (serverProps = null));
    let unlisten: (() => void) | undefined;
    on<{ server_id: string; point: MetricPoint }>("server:metrics", (m) => {
      if (m.server_id !== id) return;
      points.push(m.point);
      if (points.length > 180) points.splice(0, points.length - 180);
    }).then((u) => (unlisten = u));
    return () => {
      unlisten?.();
      api.metricsWatch(id, false).catch(() => {});
    };
  });

  async function playit(start: boolean) {
    try {
      if (start) await api.playitStart(id);
      else await api.playitStop(id);
    } catch (e) {
      errorToast(e);
    }
  }

  const PROPS = [
    ["max-players", "Joueurs max"],
    ["view-distance", "Distance de rendu"],
    ["simulation-distance", "Distance de simulation"],
    ["difficulty", "Difficulté"],
    ["online-mode", "Comptes officiels"],
    ["enable-rcon", "RCON"],
    ["level-name", "Monde"],
    ["motd", "MOTD"],
  ];
</script>

<div class="stack">
  {#if !running}
    <div class="notice">Serveur arrêté : les mesures reprennent au démarrage. L'échantillonnage ne tourne que quand cet onglet est ouvert.</div>
  {/if}

  <div class="grid cols-3">
    <div class="card tight">
      <div class="stat"><span class="label">CPU serveur</span><span class="value">{last && running ? percent(last.cpu) : "—"}</span></div>
      <Sparkline values={points.map((p) => p.cpu)} max={100} />
      <div class="muted small">Part de la machine entière ({app.system?.info.cpu_count ?? "?"} cœurs = 100 %)</div>
    </div>
    <div class="card tight">
      <div class="stat"><span class="label">RAM serveur</span><span class="value">{last && running ? bytes(last.mem) : "—"}</span></div>
      <Sparkline values={points.map((p) => p.mem)} max={xmx || 0} color="var(--blue)" />
      <div class="muted small">{xmx ? `Plafond JVM (-Xmx) : ${bytes(xmx)}` : "Pas de -Xmx dans les arguments JVM"}</div>
    </div>
    <div class="card tight">
      <div class="stat"><span class="label">Réseau (toute la machine)</span><span class="value sm">↓ {last ? bitrate(last.rx_bps) : "—"} · ↑ {last ? bitrate(last.tx_bps) : "—"}</span></div>
      <Sparkline values={points.map((p) => p.rx_bps + p.tx_bps)} color="var(--warn)" />
      <div class="muted small">Entrée + sortie, toutes interfaces (wifi inclus)</div>
    </div>
  </div>

  <div class="grid cols-3">
    <div class="card tight">
      <div class="stat"><span class="label">CPU machine</span><span class="value">{last ? percent(last.sys_cpu) : "—"}</span></div>
      <Sparkline values={points.map((p) => p.sys_cpu)} max={100} color="var(--text-2)" fill={false} />
    </div>
    <div class="card tight">
      <div class="stat"><span class="label">RAM machine</span><span class="value">{last ? `${bytes(last.sys_mem_used)} / ${bytes(last.sys_mem_total)}` : "—"}</span></div>
      <div class="progress" class:warn={last ? last.sys_mem_used / last.sys_mem_total > 0.9 : false} style="margin-top: 10px"><span style:width="{last ? (last.sys_mem_used / last.sys_mem_total) * 100 : 0}%"></span></div>
    </div>
    <div class="card tight">
      <div class="card-head" style="margin-bottom: 6px">
        <div class="stat"><span class="label">playit.gg</span><span class="value sm">{cfg.playit.enabled ? (st?.playit_running ? "Agent en marche" : "Agent arrêté") : "Non géré"}</span></div>
        {#if cfg.playit.enabled}
          {#if st?.playit_running}
            <button class="btn sm" onclick={() => playit(false)}><Icon name="stop" size={12} /> Arrêter</button>
          {:else}
            <button class="btn sm" onclick={() => playit(true)}><Icon name="play" size={12} /> Lancer</button>
          {/if}
        {/if}
      </div>
      <div class="muted small">{cfg.playit.enabled ? "Les lignes de l'agent apparaissent en violet dans la console." : "Active la gestion de l'agent dans la configuration du serveur."}</div>
    </div>
  </div>

  {#if serverProps}
    <div class="card">
      <div class="card-head">
        <h2>server.properties</h2>
        <span class="muted small">Lecture seule, modifie le fichier dans le dossier du serveur</span>
      </div>
      <div class="grid cols-3">
        {#each PROPS as [key, label] (key)}
          <div class="stat"><span class="label">{label}</span><span class="value sm mono">{serverProps[key] ?? "—"}</span></div>
        {/each}
      </div>
    </div>
  {/if}
</div>
