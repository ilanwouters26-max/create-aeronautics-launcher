<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../components/Icon.svelte";
  import Toggle from "../components/Toggle.svelte";
  import { api, isTauri } from "../lib/api";
  import { bytes, dateTime, duration, todayAt } from "../lib/format";
  import { app, errorToast, saveConfig, toast } from "../lib/stores.svelte";
  import type { AppSettings } from "../lib/types";

  let time = $state("23:00");
  let stopServers = $state(true);
  let warningMin = $state(5);
  let graceS = $state(60);
  let planning = $state(false);
  let settings = $state<AppSettings | null>(null);
  let autostart = $state(false);
  let savingSettings = $state(false);
  let now = $state(Date.now());

  const plan = $derived(app.shutdown?.plan ?? null);
  const secondsLeft = $derived(plan ? Math.max(0, Math.round((new Date(plan.at).getTime() - now) / 1000)) : 0);

  onMount(() => {
    settings = structuredClone($state.snapshot(app.config?.settings)) as AppSettings;
    api.autostartEnabled().then((v) => (autostart = v)).catch(() => {});
    const t = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(t);
  });

  async function planShutdown() {
    planning = true;
    try {
      app.shutdown = await api.shutdownPlan({ at: todayAt(time), stop_servers: stopServers, grace_s: graceS, warning_s: warningMin * 60, created_at: "" });
      toast("Extinction planifiée", "ok");
    } catch (e) {
      errorToast(e);
    } finally {
      planning = false;
    }
  }

  async function cancelShutdown() {
    try {
      app.shutdown = await api.shutdownCancel();
      toast("Extinction annulée", "ok");
    } catch (e) {
      errorToast(e);
    }
  }

  async function saveSettings() {
    if (!settings) return;
    savingSettings = true;
    const snapshot = $state.snapshot(settings) as AppSettings;
    try {
      await saveConfig((c) => {
        c.settings = snapshot;
      });
      toast("Réglages enregistrés", "ok");
    } catch (e) {
      errorToast(e);
    } finally {
      savingSettings = false;
    }
  }

  async function toggleAutostart(value: boolean) {
    try {
      await api.setAutostart(value);
      autostart = value;
      toast(value ? "Le panneau démarrera avec Windows" : "Démarrage automatique désactivé", "ok");
    } catch (e) {
      autostart = !value;
      errorToast(e);
    }
  }

  async function quit() {
    try {
      const running = await api.runningServers();
      if (running.length) app.quitPrompt = running;
      else await api.appQuit(false);
    } catch (e) {
      errorToast(e);
    }
  }
</script>

<div class="page-head">
  <div>
    <h1>Système</h1>
    <p class="sub">Extinction du PC, réglages du panneau, informations machine</p>
  </div>
  <div class="actions">
    <button class="btn" onclick={() => api.hideWindow()}><Icon name="moon" size={14} /> Réduire dans la barre</button>
    <button class="btn danger" onclick={quit}><Icon name="power" size={14} /> Quitter le panneau</button>
  </div>
</div>

<div class="grid cols-2">
  <div class="card">
    <div class="card-head"><h2>Extinction planifiée du PC</h2></div>
    {#if app.shutdown?.os_armed}
      <div class="notice err" style="margin-bottom: 12px">
        <strong>Windows va s'éteindre.</strong> Les serveurs ont été arrêtés. Annule tout de suite si ce n'est pas voulu.
        <div style="margin-top: 8px"><button class="btn danger sm" onclick={cancelShutdown}>Annuler l'extinction</button></div>
      </div>
    {:else if plan}
      <div class="notice warn" style="margin-bottom: 12px">
        <div class="row between">
          <div>
            <div><strong>{dateTime(plan.at)}</strong> · dans {duration(secondsLeft)}</div>
            <div class="small" style="margin-top: 4px">
              {#if plan.warning_s}Annonce in-game {Math.round(plan.warning_s / 60)} min avant. {/if}
              {plan.stop_servers ? "Arrêt propre des serveurs (save-all, stop), " : "Serveurs laissés tels quels, "}
              puis extinction Windows avec {plan.grace_s} s pour annuler.
            </div>
          </div>
          <button class="btn danger sm" onclick={cancelShutdown}>Annuler</button>
        </div>
      </div>
    {/if}
    <div class="stack" style="gap: 10px">
      <div class="form-grid">
        <div class="field">
          <label for="sd-time">Heure</label>
          <input id="sd-time" class="input" type="time" bind:value={time} />
          <span class="hint">Aujourd'hui, ou demain si l'heure est passée</span>
        </div>
        <div class="field">
          <label for="sd-warn">Annonce aux joueurs avant (min)</label>
          <input id="sd-warn" class="input" type="number" min="0" bind:value={warningMin} />
        </div>
        <div class="field">
          <label for="sd-grace">Délai d'annulation Windows (s)</label>
          <input id="sd-grace" class="input" type="number" min="10" max="3600" bind:value={graceS} />
        </div>
        <div class="field" style="justify-content: flex-end">
          <Toggle bind:checked={stopServers} label="Arrêter proprement les serveurs avant" />
        </div>
      </div>
      <div class="row between">
        <span class="muted small">{app.system?.is_windows === false ? "Disponible sous Windows uniquement." : "La séquence tourne tant que le panneau est ouvert (barre système comprise)."}</span>
        <button class="btn primary" onclick={planShutdown} disabled={planning}><Icon name="power" size={14} /> {plan ? "Remplacer le plan" : "Planifier l'extinction"}</button>
      </div>
    </div>
  </div>

  <div class="card">
    <div class="card-head"><h2>Panneau</h2></div>
    {#if settings}
      <div class="stack" style="gap: 10px">
        <Toggle bind:checked={autostart} label="Démarrer avec Windows" disabled={!isTauri} onchange={toggleAutostart} />
        <Toggle bind:checked={settings.start_minimized} label="Démarrer réduit dans la barre système" />
        <Toggle bind:checked={settings.close_to_tray} label="Fermer la fenêtre réduit dans la barre (au lieu de quitter)" />
        <hr class="sep" />
        <div class="form-grid">
          <div class="field">
            <label for="s-vis">Mesures CPU/RAM, onglet visible (ms)</label>
            <input id="s-vis" class="input" type="number" min="500" step="500" bind:value={settings.metrics_interval_visible_ms} />
          </div>
          <div class="field">
            <label for="s-hid">Mesures en arrière-plan (ms, 0 = aucune)</label>
            <input id="s-hid" class="input" type="number" min="0" step="1000" bind:value={settings.metrics_interval_hidden_ms} />
          </div>
          <div class="field">
            <label for="s-ping">Ping des serveurs (s)</label>
            <input id="s-ping" class="input" type="number" min="10" bind:value={settings.ping_interval_s} />
          </div>
          <div class="field">
            <label for="s-stats">Analyse world/stats (min)</label>
            <input id="s-stats" class="input" type="number" min="1" bind:value={settings.stats_scan_interval_min} />
          </div>
          <div class="field">
            <label for="s-log">Lignes de console gardées</label>
            <input id="s-log" class="input" type="number" min="200" step="100" bind:value={settings.log_buffer_lines} />
          </div>
        </div>
        <div class="row" style="justify-content: flex-end">
          <button class="btn primary" onclick={saveSettings} disabled={savingSettings}><Icon name="check" size={14} /> Enregistrer</button>
        </div>
      </div>
    {/if}
  </div>

  <div class="card">
    <div class="card-head"><h2>Machine</h2></div>
    {#if app.system}
      <div class="stack small" style="gap: 6px">
        <div class="row between"><span class="muted">Système</span><span>{app.system.info.os || "—"}</span></div>
        <div class="row between"><span class="muted">Processeur</span><span>{app.system.info.cpu_brand || "—"} · {app.system.info.cpu_count} cœurs</span></div>
        <div class="row between"><span class="muted">Mémoire</span><span>{bytes(app.system.info.mem_total)}</span></div>
        <div class="row between"><span class="muted">Panneau</span><span class="mono">{app.appMetrics.cpu.toFixed(1)} % CPU · {bytes(app.appMetrics.mem)} · pid {app.system.app_pid}</span></div>
      </div>
    {/if}
  </div>

  <div class="card">
    <div class="card-head"><h2>Données du panneau</h2><button class="btn ghost sm" onclick={() => api.openPath(app.system?.paths.data_dir ?? "").catch(errorToast)}><Icon name="folder" size={14} /></button></div>
    {#if app.system}
      <div class="stack small" style="gap: 6px">
        <div><span class="muted">Configuration</span><div class="mono ellipsis" title={app.system.paths.config_file}>{app.system.paths.config_file}</div></div>
        <div><span class="muted">Historique (SQLite)</span><div class="mono ellipsis" title={app.system.paths.history_db}>{app.system.paths.history_db}</div></div>
      </div>
    {/if}
  </div>
</div>
