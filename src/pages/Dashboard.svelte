<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../components/Icon.svelte";
  import Modal from "../components/Modal.svelte";
  import ServerForm from "../components/ServerForm.svelte";
  import { api } from "../lib/api";
  import { defaultServer } from "../lib/defaults";
  import { basenameOfDir, dateTime, duration, time } from "../lib/format";
  import { app, errorToast, navigate, saveConfig, toast } from "../lib/stores.svelte";
  import type { ServerConfig, ServerState } from "../lib/types";

  let adding = $state(false);
  let draft = $state<ServerConfig>(defaultServer());
  let saving = $state(false);

  const servers = $derived(app.config?.servers ?? []);
  const states = $derived(app.states);
  const runningCount = $derived(servers.filter((s) => states[s.id]?.status === "running").length);
  const playersTotal = $derived(servers.reduce((n, s) => n + (states[s.id]?.players.length ?? 0), 0));
  const bugsOpen = $derived(servers.reduce((n, s) => n + (states[s.id]?.counts.bugs_open ?? 0), 0));
  const suspiciousOpen = $derived(servers.reduce((n, s) => n + (states[s.id]?.counts.suspicious_open ?? 0), 0));

  onMount(() => {
    api.metricsWatch("app", true).catch(() => {});
    return () => {
      api.metricsWatch("app", false).catch(() => {});
    };
  });

  export function statusLabel(st?: ServerState): string {
    switch (st?.status) {
      case "running":
        return "En ligne";
      case "starting":
        return "Démarrage";
      case "stopping":
        return "Arrêt";
      case "crashed":
        return "Crash";
      case "unknown":
        return "Inconnu";
      default:
        return "Arrêté";
    }
  }

  function badgeClass(st?: ServerState): string {
    switch (st?.status) {
      case "running":
        return "ok";
      case "starting":
      case "stopping":
        return "warn";
      case "crashed":
        return "err";
      default:
        return "off";
    }
  }

  async function run(action: Promise<unknown>, okMessage?: string) {
    try {
      await action;
      if (okMessage) toast(okMessage, "ok");
    } catch (e) {
      errorToast(e);
    }
  }

  function openAdd() {
    draft = defaultServer();
    adding = true;
  }

  async function addServer() {
    saving = true;
    const snapshot = $state.snapshot(draft) as ServerConfig;
    try {
      await saveConfig((c) => c.servers.push(snapshot));
      adding = false;
      toast("Serveur ajouté", "ok");
      navigate("server", snapshot.id, "console");
    } catch (e) {
      errorToast(e);
    } finally {
      saving = false;
    }
  }

  function describe(s: ServerConfig): string {
    const where = s.kind.type === "local" ? basenameOfDir(s.kind.dir) || "dossier à définir" : `${s.kind.host}:${s.kind.port}`;
    const stack = [s.loader, s.mc_version].filter(Boolean).join(" ");
    return stack ? `${where} · ${stack}` : where;
  }

  function nextEventText(st?: ServerState): string {
    const n = st?.next_event;
    if (!n) return "";
    return `${n.kind === "open" ? "Ouverture" : "Fermeture"} à ${time(n.at)} (dans ${duration(n.in_seconds)})`;
  }
</script>

<div class="page-head">
  <div>
    <h1>Vue d'ensemble</h1>
    <p class="sub">{servers.length} serveur{servers.length > 1 ? "s" : ""} · {runningCount} en ligne · {playersTotal} joueur{playersTotal > 1 ? "s" : ""} connecté{playersTotal > 1 ? "s" : ""}</p>
  </div>
  <div class="actions">
    <button class="btn primary" onclick={openAdd}><Icon name="plus" size={14} /> Ajouter un serveur</button>
  </div>
</div>

{#if app.shutdown?.plan}
  <div class="notice warn row between" style="margin-bottom: 16px">
    <span><Icon name="power" size={14} /> Extinction du PC planifiée le <strong>{dateTime(app.shutdown.plan.at)}</strong> (dans {duration(app.shutdown.seconds_left)}){app.shutdown.plan.stop_servers ? ", les serveurs seront arrêtés proprement avant" : ""}.</span>
    <span class="row">
      <button class="btn sm" onclick={() => navigate("system")}>Détails</button>
      <button class="btn sm danger" onclick={() => run(api.shutdownCancel(), "Extinction annulée")}>Annuler</button>
    </span>
  </div>
{/if}

<div class="grid cols-3" style="margin-bottom: 16px; grid-template-columns: repeat(4, minmax(0, 1fr))">
  <div class="card tight stat"><span class="label">Serveurs en ligne</span><span class="value">{runningCount} <span class="muted small">/ {servers.length}</span></span></div>
  <div class="card tight stat"><span class="label">Joueurs connectés</span><span class="value">{playersTotal}</span></div>
  <div class="card tight stat"><span class="label">Bugs ouverts</span><span class="value" style:color={bugsOpen ? "var(--err)" : undefined}>{bugsOpen}</span></div>
  <div class="card tight stat"><span class="label">Alertes suspectes</span><span class="value" style:color={suspiciousOpen ? "var(--warn)" : undefined}>{suspiciousOpen}</span></div>
</div>

{#if servers.length === 0}
  <div class="empty">
    <p>Aucun serveur configuré.</p>
    <p class="small" style="margin-top: 6px">Ajoute un serveur local (lancé par le panneau) ou hébergé (ping + RCON).</p>
  </div>
{:else}
  <div class="grid auto">
    {#each servers as s (s.id)}
      {@const st = states[s.id]}
      <div
        class="card server-card"
        role="button"
        tabindex="0"
        onclick={(e) => { if (!(e.target as HTMLElement).closest(".card-actions")) navigate("server", s.id, "console"); }}
        onkeydown={(e) => { if (e.key === "Enter" && e.target === e.currentTarget) navigate("server", s.id, "console"); }}
      >
        <div class="row between">
          <div class="row" style="min-width: 0">
            <span class="dot {st?.status ?? 'stopped'}"></span>
            <strong class="ellipsis">{s.name}</strong>
          </div>
          <span class="badge {badgeClass(st)}">{statusLabel(st)}</span>
        </div>
        <div class="muted small ellipsis" style="margin-top: 4px">{describe(s)}{s.kind.type === "remote" ? " · hébergé" : ""}</div>

        {#if st?.status === "starting"}
          <div class="progress" style="margin: 12px 0 4px"><span style:width="{st.start_progress}%"></span></div>
          <div class="muted small">Préparation du monde {st.start_progress} %</div>
        {:else}
          <div class="row wrap" style="margin-top: 12px; gap: 16px">
            <div class="stat"><span class="label">Joueurs</span><span class="value sm">{st?.players.length ?? 0}{st?.players_max ? ` / ${st.players_max}` : ""}</span></div>
            <div class="stat"><span class="label">Uptime</span><span class="value sm">{st?.status === "running" && !st.remote ? duration(st.uptime_s) : "—"}</span></div>
            {#if st && (st.counts.bugs_open || st.counts.suspicious_open)}
              <div class="stat"><span class="label">Alertes</span><span class="value sm row" style="gap: 6px">
                {#if st.counts.bugs_open}<span class="badge err">{st.counts.bugs_open} bug{st.counts.bugs_open > 1 ? "s" : ""}</span>{/if}
                {#if st.counts.suspicious_open}<span class="badge warn">{st.counts.suspicious_open} suspect{st.counts.suspicious_open > 1 ? "s" : ""}</span>{/if}
              </span></div>
            {/if}
          </div>
        {/if}

        {#if st?.players.length}
          <div class="muted small ellipsis" style="margin-top: 8px">{st.players.map((p) => p.name).join(", ")}</div>
        {/if}
        {#if nextEventText(st)}
          <div class="small" style="margin-top: 8px; color: var(--text-2)"><Icon name="clock" size={12} /> {nextEventText(st)}</div>
        {/if}
        {#if st?.last_error}
          <div class="notice err small" style="margin-top: 8px">{st.last_error}</div>
        {/if}

        <div class="row card-actions">
          {#if s.kind.type === "local"}
            {#if st?.status === "running" || st?.status === "starting"}
              <button class="btn sm" onclick={() => run(api.serverStop(s.id))}><Icon name="stop" size={12} /> Arrêter</button>
              <button class="btn sm" onclick={() => run(api.serverRestart(s.id))}><Icon name="refresh" size={12} /> Redémarrer</button>
            {:else if st?.status === "stopping"}
              <span class="muted small">Arrêt en cours…</span>
            {:else}
              <button class="btn sm primary" onclick={() => run(api.serverStart(s.id))}><Icon name="play" size={12} /> Démarrer</button>
            {/if}
          {:else}
            <button class="btn sm" onclick={() => run(api.serverPing(s.id))}><Icon name="wifi" size={12} /> Ping</button>
            {#if st?.last_ping}<span class="muted small">{st.last_ping.online ? `${st.last_ping.latency_ms} ms · ${st.last_ping.version}` : st.last_ping.error}</span>{/if}
          {/if}
          <button class="btn ghost sm" style="margin-left: auto" onclick={() => navigate("server", s.id, "console")}><Icon name="terminal" size={12} /> Console</button>
        </div>
      </div>
    {/each}
  </div>
{/if}

<Modal open={adding} title="Nouveau serveur" onclose={() => (adding = false)} width={760}>
  <ServerForm bind:server={draft} onsave={addServer} oncancel={() => (adding = false)} {saving} />
</Modal>

<style>
  .server-card {
    cursor: pointer;
    transition: border-color 0.12s ease;
  }

  .server-card:hover {
    border-color: var(--border-strong);
  }

  .card-actions {
    margin-top: 14px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
    cursor: default;
  }
</style>
