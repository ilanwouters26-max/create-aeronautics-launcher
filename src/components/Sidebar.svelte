<script lang="ts">
  import Icon from "./Icon.svelte";
  import { app, navigate } from "../lib/stores.svelte";
  import { bytes } from "../lib/format";
  import type { ServerStatus } from "../lib/types";

  const route = $derived(app.route);
  const servers = $derived(app.config?.servers ?? []);

  function statusClass(id: string): ServerStatus {
    return app.states[id]?.status ?? "stopped";
  }

  function players(id: string): string {
    const s = app.states[id];
    if (!s) return "";
    if (s.status !== "running") return "";
    return `${s.players.length}${s.players_max ? "/" + s.players_max : ""}`;
  }

  const totalAlerts = $derived(Object.values(app.states).reduce((n, s) => n + s.counts.bugs_open + s.counts.suspicious_open, 0));
</script>

<aside class="sidebar">
  <div class="brand">
    <span class="logo"><Icon name="cube" size={18} /></span>
    <div>
      <div class="brand-name">Telek Panel</div>
      <div class="brand-sub">Serveurs Minecraft</div>
    </div>
  </div>

  <nav>
    <button class:active={route.page === "dashboard"} onclick={() => navigate("dashboard")}>
      <Icon name="home" /> <span>Vue d'ensemble</span>
      {#if totalAlerts > 0}<span class="pill alert">{totalAlerts}</span>{/if}
    </button>

    <div class="section">Serveurs</div>
    {#each servers as s (s.id)}
      <button class:active={route.page === "server" && route.id === s.id} onclick={() => navigate("server", s.id, route.page === "server" && route.id === s.id ? route.tab : "console")}>
        <span class="dot {statusClass(s.id)}"></span>
        <span class="ellipsis">{s.name}</span>
        {#if players(s.id)}<span class="pill">{players(s.id)}</span>{/if}
      </button>
    {/each}
    {#if servers.length === 0}
      <div class="hint">Aucun serveur. Ajoute-en un depuis la vue d'ensemble.</div>
    {/if}

    <div class="section">Modpack</div>
    <button class:active={route.page === "client"} onclick={() => navigate("client")}>
      <Icon name="layers" /> <span>Client</span>
    </button>
    <button class:active={route.page === "launcher"} onclick={() => navigate("launcher")}>
      <Icon name="upload" /> <span>Launcher</span>
    </button>

    <div class="section">PC</div>
    <button class:active={route.page === "system"} onclick={() => navigate("system")}>
      <Icon name="monitor" /> <span>Système</span>
      {#if app.shutdown?.plan}<span class="pill warn"><Icon name="power" size={11} /></span>{/if}
    </button>
  </nav>

  <div class="foot">
    <div class="row between small muted">
      <span>Panel</span>
      <span class="mono">{app.appMetrics.cpu.toFixed(1)} % · {bytes(app.appMetrics.mem)}</span>
    </div>
  </div>
</aside>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    background: var(--bg-elev);
    border-right: 1px solid var(--border);
    height: 100vh;
    overflow: hidden;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 16px 16px 12px;
  }

  .logo {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    background: var(--accent);
    color: var(--accent-text);
    display: grid;
    place-items: center;
  }

  .brand-name {
    font-weight: 600;
  }

  .brand-sub {
    font-size: 11.5px;
    color: var(--muted);
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 4px 10px;
    overflow: auto;
    flex: 1;
  }

  nav button {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    text-align: left;
    padding: 7px 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    cursor: pointer;
  }

  nav button:hover {
    background: var(--surface);
    color: var(--text);
  }

  nav button.active {
    background: var(--surface-2);
    color: var(--text);
  }

  nav button span.ellipsis {
    flex: 1;
    min-width: 0;
  }

  nav button > span:first-child:not(.dot) {
    flex: 1;
  }

  .section {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
    padding: 14px 10px 4px;
  }

  .hint {
    font-size: 12px;
    color: var(--muted);
    padding: 4px 10px;
  }

  .pill {
    margin-left: auto;
    font-size: 11px;
    background: var(--surface-3);
    border-radius: 999px;
    padding: 1px 7px;
    color: var(--text-2);
    display: inline-flex;
    align-items: center;
  }

  .pill.alert {
    background: rgba(240, 112, 106, 0.18);
    color: var(--err);
  }

  .pill.warn {
    background: rgba(242, 184, 75, 0.18);
    color: var(--warn);
  }

  .foot {
    padding: 10px 16px;
    border-top: 1px solid var(--border);
  }
</style>
