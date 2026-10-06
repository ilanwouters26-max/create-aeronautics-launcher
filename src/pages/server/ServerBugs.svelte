<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../../components/Icon.svelte";
  import Modal from "../../components/Modal.svelte";
  import { api, on } from "../../lib/api";
  import { dateTime, relative } from "../../lib/format";
  import { errorToast, toast } from "../../lib/stores.svelte";
  import type { BugRow } from "../../lib/types";

  let { id }: { id: string } = $props();

  let bugs = $state<BugRow[]>([]);
  let includeResolved = $state(false);
  let category = $state<"all" | "error" | "warn" | "lag" | "crash">("all");
  let expanded = $state<number | null>(null);
  let crashText = $state<string | null>(null);
  let confirmClear = $state(false);

  const filtered = $derived(bugs.filter((b) => category === "all" || b.category === category));

  async function load() {
    try {
      bugs = await api.bugsList(id, includeResolved);
    } catch (e) {
      errorToast(e);
    }
  }

  onMount(() => {
    load();
    let unlisten: (() => void) | undefined;
    on<BugRow>("history:bug", (b) => {
      if (b.server_id !== id) return;
      const i = bugs.findIndex((x) => x.id === b.id);
      if (i >= 0) bugs[i] = b;
      else bugs.unshift(b);
    }).then((u) => (unlisten = u));
    return () => unlisten?.();
  });

  $effect(() => {
    void includeResolved;
    load();
  });

  async function resolve(b: BugRow, resolved: boolean) {
    try {
      await api.bugResolve(b.id, id, resolved);
      await load();
    } catch (e) {
      errorToast(e);
    }
  }

  async function remove(b: BugRow) {
    try {
      await api.bugDelete(b.id, id);
      bugs = bugs.filter((x) => x.id !== b.id);
    } catch (e) {
      errorToast(e);
    }
  }

  async function clearAll() {
    try {
      await api.historyClear(id, true, false);
      confirmClear = false;
      bugs = [];
      toast("Historique des bugs vidé", "ok");
    } catch (e) {
      errorToast(e);
    }
  }

  function crashPath(b: BugRow): string | null {
    const m = b.details.match(/Rapport : (.+)$/m);
    return m ? m[1].trim() : null;
  }

  async function openCrash(b: BugRow) {
    const p = crashPath(b);
    if (!p) return;
    try {
      crashText = await api.crashReportRead(p);
    } catch (e) {
      errorToast(e);
    }
  }

  function badge(c: string): string {
    return c === "error" || c === "crash" ? "err" : c === "lag" ? "info" : "warn";
  }

  function label(c: string): string {
    return c === "error" ? "Erreur" : c === "crash" ? "Crash" : c === "lag" ? "Lag" : "Avertissement";
  }
</script>

<div class="card">
  <div class="card-head">
    <div class="row">
      <h2>Bugs détectés</h2>
      <span class="muted small">Erreurs et stacktraces regroupées, une ligne par bug distinct</span>
    </div>
    <div class="actions">
      <label class="toggle small"><input type="checkbox" bind:checked={includeResolved} /> Voir les résolus</label>
      <button class="btn ghost sm danger" onclick={() => (confirmClear = true)} disabled={!bugs.length}>Tout effacer</button>
    </div>
  </div>

  <div class="tabs" style="margin-bottom: 10px">
    {#each [["all", "Tous"], ["error", "Erreurs"], ["warn", "Avertissements"], ["lag", "Lag"], ["crash", "Crashs"]] as [k, l] (k)}
      <button class:active={category === k} onclick={() => (category = k as typeof category)}>{l}</button>
    {/each}
  </div>

  {#if filtered.length}
    <div class="list">
      {#each filtered as b (b.id)}
        <div class="bug" class:resolved={b.resolved}>
          <div class="list-item" role="button" tabindex="0" onclick={() => (expanded = expanded === b.id ? null : b.id)} onkeydown={(e) => { if (e.key === "Enter") expanded = expanded === b.id ? null : b.id; }}>
            <Icon name={expanded === b.id ? "chevron-down" : "chevron-right"} size={14} />
            <span class="badge {badge(b.category)}">{label(b.category)}</span>
            <span class="ellipsis" style="flex: 1" title={b.title}>{b.title}</span>
            {#if b.exception}<span class="badge mono">{b.exception}</span>{/if}
            <span class="badge">×{b.count}</span>
            <span class="muted small" style="flex: 0 0 110px; text-align: right">{relative(b.last_seen)}</span>
          </div>
          {#if expanded === b.id}
            <div class="details">
              <div class="row between small muted" style="margin-bottom: 6px">
                <span>Première fois {dateTime(b.first_seen)} · dernière {dateTime(b.last_seen)}</span>
                <span class="row">
                  {#if crashPath(b)}<button class="btn sm" onclick={() => openCrash(b)}><Icon name="file" size={12} /> Rapport de crash</button>{/if}
                  {#if b.resolved}
                    <button class="btn sm" onclick={() => resolve(b, false)}>Rouvrir</button>
                  {:else}
                    <button class="btn sm" onclick={() => resolve(b, true)}><Icon name="check" size={12} /> Résolu</button>
                  {/if}
                  <button class="btn ghost sm danger" onclick={() => remove(b)}><Icon name="trash" size={12} /></button>
                </span>
              </div>
              <pre class="mono">{b.details}</pre>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {:else}
    <div class="empty">Aucun bug {includeResolved ? "" : "ouvert "}pour ce serveur.</div>
  {/if}
</div>

<Modal open={confirmClear} title="Effacer tous les bugs ?" onclose={() => (confirmClear = false)}>
  <p>L'historique des bugs de ce serveur sera vidé. Les alertes suspectes ne sont pas touchées.</p>
  {#snippet footer()}
    <button class="btn" onclick={() => (confirmClear = false)}>Annuler</button>
    <button class="btn danger" onclick={clearAll}>Effacer</button>
  {/snippet}
</Modal>

<Modal open={crashText !== null} title="Rapport de crash" onclose={() => (crashText = null)} width={900}>
  <pre class="mono crash">{crashText}</pre>
</Modal>

<style>
  .bug.resolved .list-item {
    opacity: 0.55;
  }

  .details {
    padding: 8px 12px 12px 36px;
    border-bottom: 1px solid var(--border);
  }

  pre {
    margin: 0;
    padding: 10px;
    background: rgba(3, 7, 14, 0.9);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    max-height: 320px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 12px;
    color: var(--text-2);
  }

  pre.crash {
    max-height: 70vh;
  }
</style>
