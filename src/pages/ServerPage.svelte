<script lang="ts">
  import Icon from "../components/Icon.svelte";
  import Modal from "../components/Modal.svelte";
  import Console from "../components/Console.svelte";
  import ServerForm from "../components/ServerForm.svelte";
  import ServerPlayers from "./server/ServerPlayers.svelte";
  import ServerBugs from "./server/ServerBugs.svelte";
  import ServerSuspicious from "./server/ServerSuspicious.svelte";
  import ServerMods from "./server/ServerMods.svelte";
  import ServerResources from "./server/ServerResources.svelte";
  import ServerSchedule from "./server/ServerSchedule.svelte";
  import { api } from "../lib/api";
  import { duration } from "../lib/format";
  import { app, errorToast, navigate, saveConfig, serverCfg, toast } from "../lib/stores.svelte";
  import type { ServerConfig } from "../lib/types";

  let { id }: { id: string } = $props();

  const cfg = $derived(serverCfg(id));
  const st = $derived(app.states[id]);
  const tab = $derived(app.route.tab || "console");
  const remote = $derived(cfg?.kind.type === "remote");
  const canSend = $derived(remote ? (cfg?.kind.type === "remote" && cfg.kind.rcon.enabled) : st?.status === "running" || st?.status === "starting");
  const playerNames = $derived(st?.players.map((p) => p.name) ?? []);

  const tabs = $derived(
    remote
      ? [
          { key: "console", label: "Console", icon: "terminal", count: 0 },
          { key: "players", label: "Joueurs", icon: "users", count: 0 },
          { key: "config", label: "Config", icon: "settings", count: 0 },
        ]
      : [
          { key: "console", label: "Console", icon: "terminal", count: 0 },
          { key: "players", label: "Joueurs", icon: "users", count: 0 },
          { key: "bugs", label: "Bugs", icon: "bug", count: st?.counts.bugs_open ?? 0 },
          { key: "suspicious", label: "Suspect", icon: "shield", count: st?.counts.suspicious_open ?? 0 },
          { key: "mods", label: "Mods", icon: "layers", count: 0 },
          { key: "resources", label: "Ressources", icon: "activity", count: 0 },
          { key: "schedule", label: "Planning", icon: "clock", count: 0 },
          { key: "config", label: "Config", icon: "settings", count: 0 },
        ],
  );

  let draft = $state<ServerConfig | null>(null);
  let saving = $state(false);
  let confirmDelete = $state(false);
  let confirmKill = $state(false);

  $effect(() => {
    if (tab === "config" && cfg && (!draft || draft.id !== cfg.id)) {
      draft = structuredClone($state.snapshot(cfg)) as ServerConfig;
    }
  });

  function statusLabel(): string {
    switch (st?.status) {
      case "running":
        return remote ? "En ligne" : `En ligne depuis ${duration(st.uptime_s)}`;
      case "starting":
        return `Démarrage ${st.start_progress} %`;
      case "stopping":
        return "Arrêt en cours";
      case "crashed":
        return "Crash";
      case "unknown":
        return "État inconnu";
      default:
        return remote ? "Hors ligne" : "Arrêté";
    }
  }

  function badgeClass(): string {
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

  async function saveDraft() {
    if (!draft) return;
    saving = true;
    const snapshot = $state.snapshot(draft) as ServerConfig;
    try {
      await saveConfig((c) => {
        const i = c.servers.findIndex((s) => s.id === id);
        if (i >= 0) c.servers[i] = snapshot;
      });
      toast("Configuration enregistrée", "ok");
    } catch (e) {
      errorToast(e);
    } finally {
      saving = false;
    }
  }

  async function deleteServer() {
    try {
      await saveConfig((c) => {
        c.servers = c.servers.filter((s) => s.id !== id);
      });
      confirmDelete = false;
      toast("Serveur retiré du panneau (les fichiers restent sur le disque)", "ok");
      navigate("dashboard");
    } catch (e) {
      errorToast(e);
    }
  }

  async function openDir() {
    if (cfg?.kind.type === "local") await run(api.openPath(cfg.kind.dir));
  }
</script>

{#if !cfg}
  <div class="empty">
    <p>Serveur introuvable.</p>
    <button class="btn" style="margin-top: 10px" onclick={() => navigate("dashboard")}>Retour à la vue d'ensemble</button>
  </div>
{:else}
  <div class="page-head">
    <div style="min-width: 0">
      <div class="row">
        <h1 class="ellipsis">{cfg.name}</h1>
        <span class="badge {badgeClass()}">{statusLabel()}</span>
        {#if st?.playit_running}<span class="badge info">playit</span>{/if}
      </div>
      <p class="sub">
        {#if cfg.kind.type === "local"}<span class="mono">{cfg.kind.dir || "dossier à définir"}</span>{:else}<span class="mono">{cfg.kind.host}:{cfg.kind.port}</span> · hébergé{/if}
        {#if cfg.loader || cfg.mc_version} · {[cfg.loader, cfg.mc_version].filter(Boolean).join(" ")}{/if}
        {#if st?.players.length} · {st.players.length}{st.players_max ? `/${st.players_max}` : ""} joueur{st.players.length > 1 ? "s" : ""}{/if}
      </p>
    </div>
    <div class="actions">
      {#if cfg.kind.type === "local"}
        {#if st?.status === "running" || st?.status === "starting"}
          <button class="btn" onclick={() => run(api.serverStop(id))}><Icon name="stop" size={14} /> Arrêter</button>
          <button class="btn" onclick={() => run(api.serverRestart(id))}><Icon name="refresh" size={14} /> Redémarrer</button>
          <button class="btn ghost danger" onclick={() => (confirmKill = true)} title="Tue le processus sans sauvegarder"><Icon name="ban" size={14} /></button>
        {:else if st?.status === "stopping"}
          <button class="btn" disabled><Icon name="stop" size={14} /> Arrêt…</button>
          <button class="btn ghost danger" onclick={() => (confirmKill = true)}><Icon name="ban" size={14} /> Forcer</button>
        {:else}
          <button class="btn primary" onclick={() => run(api.serverStart(id))}><Icon name="play" size={14} /> Démarrer</button>
        {/if}
        <button class="btn ghost" onclick={openDir} title="Ouvrir le dossier du serveur"><Icon name="folder" size={14} /></button>
      {:else}
        <button class="btn" onclick={() => run(api.serverPing(id))}><Icon name="wifi" size={14} /> Ping</button>
        {#if st?.last_ping}
          <span class="muted small">{st.last_ping.online ? `${st.last_ping.latency_ms} ms · ${st.last_ping.version} · ${st.last_ping.motd}` : st.last_ping.error}</span>
        {/if}
      {/if}
    </div>
  </div>

  {#if st?.last_error}
    <div class="notice err" style="margin-bottom: 14px">{st.last_error}</div>
  {/if}

  <div class="tabs">
    {#each tabs as t (t.key)}
      <button class:active={tab === t.key} onclick={() => navigate("server", id, t.key)}>
        <Icon name={t.icon} size={14} /> {t.label}
        {#if t.count}<span class="count alert">{t.count}</span>{/if}
      </button>
    {/each}
  </div>

  {#if tab === "console"}
    {#if remote && cfg.kind.type === "remote" && !cfg.kind.rcon.enabled}
      <div class="notice" style="margin-bottom: 12px">Active RCON dans la configuration pour envoyer des commandes à ce serveur hébergé. Les logs complets ne sont pas accessibles à distance, seule la réponse des commandes s'affiche.</div>
    {/if}
    <Console {id} {canSend} players={playerNames} />
  {:else if tab === "players"}
    <ServerPlayers {id} {remote} />
  {:else if tab === "bugs"}
    <ServerBugs {id} />
  {:else if tab === "suspicious"}
    <ServerSuspicious {id} />
  {:else if tab === "mods"}
    <ServerMods {cfg} />
  {:else if tab === "resources"}
    <ServerResources {id} {cfg} />
  {:else if tab === "schedule"}
    <ServerSchedule {id} />
  {:else if tab === "config" && draft}
    <div class="card">
      <ServerForm bind:server={draft} onsave={saveDraft} {saving} />
    </div>
    <div class="card" style="margin-top: 14px">
      <div class="row between">
        <div>
          <strong>Retirer ce serveur du panneau</strong>
          <div class="muted small">Les fichiers du serveur ne sont pas touchés. L'historique des bugs et alertes est conservé.</div>
        </div>
        <button class="btn danger" onclick={() => (confirmDelete = true)} disabled={st?.status === "running" || st?.status === "starting"}>Retirer</button>
      </div>
    </div>
  {/if}
{/if}

<Modal open={confirmDelete} title="Retirer le serveur ?" onclose={() => (confirmDelete = false)}>
  <p>{cfg?.name} disparaîtra du panneau. Rien n'est supprimé sur le disque.</p>
  {#snippet footer()}
    <button class="btn" onclick={() => (confirmDelete = false)}>Annuler</button>
    <button class="btn danger" onclick={deleteServer}>Retirer</button>
  {/snippet}
</Modal>

<Modal open={confirmKill} title="Forcer l'arrêt ?" onclose={() => (confirmKill = false)}>
  <p>Le processus Java sera tué immédiatement, sans sauvegarde. À réserver à un serveur bloqué.</p>
  {#snippet footer()}
    <button class="btn" onclick={() => (confirmKill = false)}>Annuler</button>
    <button class="btn danger" onclick={() => { confirmKill = false; run(api.serverKill(id)); }}>Tuer le processus</button>
  {/snippet}
</Modal>
