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
    <span class="logo" aria-hidden="true"></span>
    <div>
      <div class="brand-name">Telek Panel</div>
      <div class="brand-sub">Contrôle serveurs</div>
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
    background: linear-gradient(180deg, rgba(8, 16, 30, 0.96), rgba(4, 8, 16, 0.98));
    border-right: 1px solid var(--border);
    height: 100vh;
    overflow: hidden;
    position: relative;
  }

  .sidebar::after {
    content: "";
    position: absolute;
    top: 0;
    right: -1px;
    width: 1px;
    height: 220px;
    background: linear-gradient(180deg, var(--accent), transparent);
    box-shadow: var(--glow);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 18px 16px 14px;
  }

  .logo {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    flex: 0 0 auto;
    background: radial-gradient(
      circle,
      #eafdff 0 13%,
      rgba(34, 200, 255, 0.95) 15% 28%,
      rgba(34, 200, 255, 0.12) 30% 46%,
      rgba(111, 220, 255, 0.85) 48% 54%,
      rgba(34, 200, 255, 0.1) 56% 72%,
      transparent 74%
    );
    box-shadow: 0 0 18px rgba(34, 200, 255, 0.55), inset 0 0 10px rgba(34, 200, 255, 0.5);
  }

  .brand-name {
    font-family: var(--font-display);
    font-weight: 700;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    font-size: 13px;
    text-shadow: 0 0 12px rgba(34, 200, 255, 0.45);
  }

  .brand-sub {
    font-size: 10.5px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
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
    border-left: 2px solid transparent;
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
    background: transparent;
    color: var(--text-2);
    cursor: pointer;
  }

  nav button:hover {
    background: rgba(34, 200, 255, 0.05);
    color: var(--text);
  }

  nav button.active {
    background: linear-gradient(90deg, rgba(34, 200, 255, 0.16), rgba(34, 200, 255, 0.03));
    border-left-color: var(--accent);
    color: var(--text);
    box-shadow: inset 12px 0 18px -14px rgba(34, 200, 255, 0.6);
  }

  nav button span.ellipsis {
    flex: 1;
    min-width: 0;
  }

  nav button > span:first-child:not(.dot) {
    flex: 1;
  }

  .section {
    font-family: var(--font-display);
    font-size: 10.5px;
    text-transform: uppercase;
    letter-spacing: 0.18em;
    color: #4f7a9c;
    padding: 16px 10px 5px;
  }

  .hint {
    font-size: 12px;
    color: var(--muted);
    padding: 4px 10px;
  }

  .pill {
    margin-left: auto;
    font-family: var(--mono);
    font-size: 10.5px;
    background: rgba(34, 200, 255, 0.1);
    border: 1px solid var(--border);
    border-radius: 2px;
    padding: 0 6px;
    color: var(--text-2);
    display: inline-flex;
    align-items: center;
  }

  .pill.alert {
    background: rgba(255, 84, 104, 0.16);
    border-color: rgba(255, 84, 104, 0.4);
    color: var(--err);
  }

  .pill.warn {
    background: rgba(255, 181, 71, 0.14);
    border-color: rgba(255, 181, 71, 0.4);
    color: var(--warn);
  }

  .foot {
    padding: 10px 16px;
    border-top: 1px solid var(--border);
    letter-spacing: 0.04em;
  }
</style>
