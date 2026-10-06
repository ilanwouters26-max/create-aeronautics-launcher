<script lang="ts">
  import Icon from "../../components/Icon.svelte";
  import Toggle from "../../components/Toggle.svelte";
  import { duration, time } from "../../lib/format";
  import { app, errorToast, saveConfig, serverCfg, toast } from "../../lib/stores.svelte";
  import type { ScheduleConfig } from "../../lib/types";

  let { id }: { id: string } = $props();

  let draft = $state<ScheduleConfig>({ enabled: false, open_time: "06:00", close_time: "22:00", warning_s: 300 });
  let warningMin = $state(5);
  let saving = $state(false);

  const cfg = $derived(serverCfg(id));
  const st = $derived(app.states[id]);

  $effect(() => {
    if (cfg) {
      draft = structuredClone($state.snapshot(cfg.schedule)) as ScheduleConfig;
      warningMin = Math.round(cfg.schedule.warning_s / 60);
    }
  });

  async function save() {
    saving = true;
    const snapshot = { ...($state.snapshot(draft) as ScheduleConfig), warning_s: Math.max(0, warningMin) * 60 };
    try {
      await saveConfig((c) => {
        const s = c.servers.find((x) => x.id === id);
        if (s) s.schedule = snapshot;
      });
      toast("Planning enregistré", "ok");
    } catch (e) {
      errorToast(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="grid cols-2">
  <div class="card">
    <div class="card-head"><h2>Ouverture et fermeture quotidiennes</h2></div>
    <div class="stack">
      <Toggle bind:checked={draft.enabled} label="Planning actif" />
      <div class="form-grid">
        <div class="field">
          <label for="sc-open">Ouverture</label>
          <input id="sc-open" class="input" type="time" bind:value={draft.open_time} />
          <span class="hint">Vide = pas d'ouverture automatique</span>
        </div>
        <div class="field">
          <label for="sc-close">Fermeture</label>
          <input id="sc-close" class="input" type="time" bind:value={draft.close_time} />
          <span class="hint">Vide = pas de fermeture automatique</span>
        </div>
        <div class="field">
          <label for="sc-warn">Annonce in-game avant fermeture (min)</label>
          <input id="sc-warn" class="input" type="number" min="0" bind:value={warningMin} />
        </div>
      </div>
      <div class="row" style="justify-content: flex-end">
        <button class="btn primary" onclick={save} disabled={saving}><Icon name="check" size={14} /> Enregistrer</button>
      </div>
    </div>
  </div>

  <div class="card">
    <div class="card-head"><h2>Comment ça se passe</h2></div>
    <div class="stack small" style="gap: 8px; color: var(--text-2)">
      {#if st?.next_event}
        <div class="notice ok">Prochaine étape : <strong>{st.next_event.kind === "open" ? "ouverture" : "fermeture"} à {time(st.next_event.at)}</strong> (dans {duration(st.next_event.in_seconds)}).</div>
      {:else}
        <div class="notice">Aucune étape planifiée pour le moment.</div>
      {/if}
      <p>À l'heure d'ouverture, le serveur est lancé s'il est arrêté. Si tu l'arrêtes à la main dans la journée, il n'est pas relancé.</p>
      <p>Avant la fermeture, les joueurs reçoivent une annonce (puis à 1 min, 30 s, 10 s). À l'heure dite : <span class="mono">save-all</span>, <span class="mono">stop</span>, et un kill de secours si le serveur ne répond plus.</p>
      <p>Un serveur lancé à la main hors des horaires n'est pas fermé automatiquement.</p>
      <p>Le planning tourne tant que le panneau est ouvert, même réduit dans la barre système. Active « démarrer avec Windows » dans Système pour ne pas l'oublier.</p>
    </div>
  </div>
</div>
