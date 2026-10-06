<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../../components/Icon.svelte";
  import { api } from "../../lib/api";
  import { dateTime, minutes, relative, time } from "../../lib/format";
  import { app, errorToast, toast } from "../../lib/stores.svelte";
  import type { PlayerRow, StatsScan } from "../../lib/types";

  let { id, remote }: { id: string; remote: boolean } = $props();

  let history = $state<PlayerRow[]>([]);
  let scan = $state<StatsScan | null>(null);
  let scanning = $state(false);

  const st = $derived(app.states[id]);

  async function load() {
    try {
      history = await api.serverPlayers(id);
    } catch (e) {
      errorToast(e);
    }
  }

  onMount(load);

  async function runScan() {
    scanning = true;
    try {
      scan = await api.serverScanStats(id);
      if (scan.recorded) toast(`${scan.recorded} nouvelle${scan.recorded > 1 ? "s" : ""} alerte${scan.recorded > 1 ? "s" : ""} de minage`, "warn");
    } catch (e) {
      errorToast(e);
    } finally {
      scanning = false;
    }
  }

  function mined(s: { mined: Record<string, number> }, ids: string[]): number {
    return ids.reduce((n, k) => n + (s.mined[k] ?? 0), 0);
  }
</script>

<div class="stack">
  <div class="card">
    <div class="card-head">
      <h2>Connectés ({st?.players.length ?? 0}{st?.players_max ? ` / ${st.players_max}` : ""})</h2>
      {#if remote}<span class="muted small">Liste partielle fournie par le ping du serveur</span>{/if}
    </div>
    {#if st?.players.length}
      <div class="row wrap">
        {#each st.players as p (p.name)}
          <span class="badge ok" title="Connecté depuis {time(p.since)}">{p.name} <span class="muted">· {relative(p.since).replace("il y a ", "")}</span></span>
        {/each}
      </div>
    {:else}
      <div class="muted">Personne en ligne.</div>
    {/if}
  </div>

  {#if !remote}
    <div class="card">
      <div class="card-head">
        <h2>Historique de présence</h2>
        <button class="btn ghost sm" onclick={load}><Icon name="refresh" size={14} /></button>
      </div>
      {#if history.length}
        <table class="table">
          <thead><tr><th>Joueur</th><th>Sessions</th><th>Temps total</th><th>Première venue</th><th>Dernière fois</th></tr></thead>
          <tbody>
            {#each history as p (p.name)}
              <tr>
                <td><strong>{p.name}</strong></td>
                <td>{p.sessions}</td>
                <td>{minutes(p.total_minutes)}</td>
                <td class="muted">{dateTime(p.first_seen)}</td>
                <td class="muted">{relative(p.last_seen)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <div class="muted">Aucune connexion enregistrée pour l'instant. L'historique se remplit avec les logs du serveur.</div>
      {/if}
    </div>

    <div class="card">
      <div class="card-head">
        <div>
          <h2>Statistiques de jeu (world/stats)</h2>
          <div class="muted small">Blocs minés par heure de jeu, mis à jour par le serveur à chaque sauvegarde. Les seuils se règlent dans l'onglet Suspect.</div>
        </div>
        <button class="btn" onclick={runScan} disabled={scanning}><Icon name="pickaxe" size={14} /> {scanning ? "Analyse…" : "Analyser maintenant"}</button>
      </div>
      {#if scan}
        {#if scan.alerts.length}
          <div class="notice warn" style="margin-bottom: 10px">
            {#each scan.alerts as a (a.player + a.rule)}
              <div><strong>{a.player}</strong> : {a.rule} {a.per_hour}/h ({a.count} en {a.play_hours} h, seuil {a.threshold}/h)</div>
            {/each}
          </div>
        {/if}
        {#if scan.stats.length}
          <table class="table">
            <thead><tr><th>Joueur</th><th>Temps de jeu</th><th>Diamants</th><th>Diamants / h</th><th>Débris antiques</th><th>Morts</th><th>Kills</th></tr></thead>
            <tbody>
              {#each scan.stats as s (s.uuid)}
                {@const d = mined(s, ["minecraft:diamond_ore", "minecraft:deepslate_diamond_ore"])}
                <tr>
                  <td><strong>{s.name}</strong></td>
                  <td>{s.play_hours.toFixed(1)} h</td>
                  <td>{d}</td>
                  <td>{s.play_hours > 0 ? (d / s.play_hours).toFixed(1) : "—"}</td>
                  <td>{mined(s, ["minecraft:ancient_debris"])}</td>
                  <td>{s.deaths}</td>
                  <td>{s.player_kills}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {:else}
          <div class="muted">Aucun fichier de statistiques trouvé (le dossier world/stats se crée à la première sauvegarde).</div>
        {/if}
      {:else}
        <div class="muted">Pas encore analysé. L'analyse tourne aussi toute seule pendant que le serveur tourne.</div>
      {/if}
    </div>
  {/if}
</div>
