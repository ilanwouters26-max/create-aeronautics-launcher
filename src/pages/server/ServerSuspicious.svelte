<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../../components/Icon.svelte";
  import Toggle from "../../components/Toggle.svelte";
  import { api, on } from "../../lib/api";
  import { defaultDetection, miningEventDetection } from "../../lib/defaults";
  import { dateTime, relative } from "../../lib/format";
  import { errorToast, saveConfig, serverCfg, toast } from "../../lib/stores.svelte";
  import type { DetectionConfig, SuspiciousRow } from "../../lib/types";

  let { id }: { id: string } = $props();

  let rows = $state<SuspiciousRow[]>([]);
  let onlyOpen = $state(true);
  let severity = $state<"all" | "high" | "warn" | "info">("all");
  let showSettings = $state(false);
  let draft = $state<DetectionConfig>(defaultDetection());
  let saving = $state(false);

  const cfg = $derived(serverCfg(id));
  const filtered = $derived(rows.filter((r) => (!onlyOpen || !r.acknowledged) && (severity === "all" || r.severity === severity)));

  $effect(() => {
    if (cfg) draft = structuredClone($state.snapshot(cfg.detection)) as DetectionConfig;
  });

  async function load() {
    try {
      rows = await api.suspiciousList(id);
    } catch (e) {
      errorToast(e);
    }
  }

  onMount(() => {
    load();
    let unlisten: (() => void) | undefined;
    on<SuspiciousRow>("history:suspicious", (r) => {
      if (r.server_id === id) rows.unshift(r);
    }).then((u) => (unlisten = u));
    return () => unlisten?.();
  });

  async function ack(r: SuspiciousRow, value: boolean) {
    try {
      await api.suspiciousAck(r.id, id, value);
      r.acknowledged = value;
    } catch (e) {
      errorToast(e);
    }
  }

  async function remove(r: SuspiciousRow) {
    try {
      await api.suspiciousDelete(r.id, id);
      rows = rows.filter((x) => x.id !== r.id);
    } catch (e) {
      errorToast(e);
    }
  }

  async function ackAll() {
    for (const r of rows.filter((x) => !x.acknowledged)) await ack(r, true);
  }

  async function saveDetection() {
    saving = true;
    const snapshot = $state.snapshot(draft) as DetectionConfig;
    try {
      await saveConfig((c) => {
        const s = c.servers.find((x) => x.id === id);
        if (s) s.detection = snapshot;
      });
      toast("Détection enregistrée", "ok");
    } catch (e) {
      errorToast(e);
    } finally {
      saving = false;
    }
  }

  function badge(s: string): string {
    return s === "high" ? "err" : s === "warn" ? "warn" : "info";
  }

  function categoryLabel(c: string): string {
    const map: Record<string, string> = { gamemode: "Gamemode", give: "Give", op: "Op", teleport: "Téléportation", admin: "Commande admin", early_advancement: "Progrès rapide", mining: "Minage", movement: "Déplacement" };
    return map[c] ?? c;
  }

  function blocksText(rule: { blocks: string[] }): string {
    return rule.blocks.join(", ");
  }

  function setBlocks(rule: { blocks: string[] }, text: string) {
    rule.blocks = text.split(",").map((s) => s.trim()).filter(Boolean);
  }
</script>

<div class="stack">
  <div class="card">
    <div class="card-head">
      <div class="row">
        <h2>Actions suspectes</h2>
        <span class="muted small">Détectées dans les logs et les statistiques, à vérifier par toi</span>
      </div>
      <div class="actions">
        <label class="toggle small"><input type="checkbox" bind:checked={onlyOpen} /> Non traitées seulement</label>
        <button class="btn ghost sm" onclick={ackAll} disabled={!rows.some((r) => !r.acknowledged)}>Tout marquer vu</button>
        <button class="btn sm" onclick={() => (showSettings = !showSettings)}><Icon name="settings" size={14} /> Réglages</button>
      </div>
    </div>

    <div class="tabs" style="margin-bottom: 10px">
      {#each [["all", "Toutes"], ["high", "Graves"], ["warn", "À vérifier"], ["info", "Info"]] as [k, l] (k)}
        <button class:active={severity === k} onclick={() => (severity = k as typeof severity)}>{l}</button>
      {/each}
    </div>

    {#if filtered.length}
      <div class="list">
        {#each filtered as r (r.id)}
          <div class="list-item" class:seen={r.acknowledged}>
            <span class="badge {badge(r.severity)}">{categoryLabel(r.category)}</span>
            <div style="flex: 1; min-width: 0">
              <div class="ellipsis">{r.title}</div>
              {#if r.details}<div class="muted small ellipsis" title={r.details}>{r.details}</div>{/if}
            </div>
            <span class="muted small" title={dateTime(r.ts)} style="flex: 0 0 100px; text-align: right">{relative(r.ts)}</span>
            {#if r.acknowledged}
              <button class="btn ghost sm" onclick={() => ack(r, false)} title="Remettre à traiter"><Icon name="eye-off" size={13} /></button>
            {:else}
              <button class="btn sm" onclick={() => ack(r, true)} title="Marquer comme vu"><Icon name="check" size={13} /> Vu</button>
            {/if}
            <button class="btn ghost sm danger" onclick={() => remove(r)}><Icon name="trash" size={13} /></button>
          </div>
        {/each}
      </div>
    {:else}
      <div class="empty">Rien de suspect {onlyOpen ? "à traiter" : "enregistré"}.</div>
    {/if}
  </div>

  {#if showSettings}
    <div class="card">
      <div class="card-head">
        <h2>Réglages de détection</h2>
        <div class="actions">
          <button class="btn sm" onclick={() => (draft = miningEventDetection())} title="Désactive les règles de minage et de progrès rapide">Préréglage événement minage</button>
          <button class="btn sm" onclick={() => (draft = defaultDetection())}>Par défaut</button>
        </div>
      </div>
      <div class="stack">
        <Toggle bind:checked={draft.enabled} label="Détection active sur ce serveur" />
        <div class="form-grid">
          <Toggle bind:checked={draft.gamemode} label="Changement de gamemode par un joueur" />
          <Toggle bind:checked={draft.give} label="/give par un joueur" />
          <Toggle bind:checked={draft.op} label="/op et /deop par un joueur" />
          <Toggle bind:checked={draft.teleport} label="Téléportations" />
          <Toggle bind:checked={draft.other_admin} label="Autres commandes admin (time, weather, kill…)" />
          <Toggle bind:checked={draft.early_advancements} label="Progrès obtenus trop vite après la première connexion" />
        </div>
        {#if draft.early_advancements}
          <table class="table">
            <thead><tr><th>Progrès (titre exact du log)</th><th>Suspect si obtenu avant (min)</th><th></th></tr></thead>
            <tbody>
              {#each draft.early_rules as rule, i (i)}
                <tr>
                  <td><input class="input" bind:value={rule.title} /></td>
                  <td><input class="input" type="number" bind:value={rule.max_minutes} min="1" /></td>
                  <td style="width: 40px"><button class="btn ghost sm" onclick={() => draft.early_rules.splice(i, 1)}><Icon name="trash" size={13} /></button></td>
                </tr>
              {/each}
            </tbody>
          </table>
          <button class="btn sm" style="align-self: flex-start" onclick={() => draft.early_rules.push({ title: "", max_minutes: 30 })}><Icon name="plus" size={13} /> Règle</button>
        {/if}
        <hr class="sep" />
        <Toggle bind:checked={draft.mining} label="Taux de minage anormal (world/stats, par heure de jeu)" />
        {#if draft.mining}
          <div class="field" style="max-width: 320px">
            <label for="min-play">Temps de jeu minimum avant d'évaluer (min)</label>
            <input id="min-play" class="input" type="number" bind:value={draft.mining_min_playtime_min} min="1" />
          </div>
          <table class="table">
            <thead><tr><th>Règle</th><th>Blocs (IDs séparés par des virgules)</th><th>Seuil / heure</th><th></th></tr></thead>
            <tbody>
              {#each draft.mining_rules as rule, i (i)}
                <tr>
                  <td><input class="input" bind:value={rule.label} /></td>
                  <td><input class="input mono" value={blocksText(rule)} onchange={(e) => setBlocks(rule, e.currentTarget.value)} /></td>
                  <td style="width: 120px"><input class="input" type="number" bind:value={rule.per_hour} min="1" /></td>
                  <td style="width: 40px"><button class="btn ghost sm" onclick={() => draft.mining_rules.splice(i, 1)}><Icon name="trash" size={13} /></button></td>
                </tr>
              {/each}
            </tbody>
          </table>
          <button class="btn sm" style="align-self: flex-start" onclick={() => draft.mining_rules.push({ label: "", blocks: [], per_hour: 30 })}><Icon name="plus" size={13} /> Règle</button>
        {/if}
        <div class="row" style="justify-content: flex-end">
          <button class="btn primary" onclick={saveDetection} disabled={saving}><Icon name="check" size={14} /> Enregistrer</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .list-item.seen {
    opacity: 0.55;
  }
</style>
