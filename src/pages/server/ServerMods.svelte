<script lang="ts">
  import ModList from "../../components/ModList.svelte";
  import ModDiffView from "../../components/ModDiffView.svelte";
  import { api } from "../../lib/api";
  import { joinPath } from "../../lib/format";
  import { app, errorToast } from "../../lib/stores.svelte";
  import type { ModDiff, ServerConfig } from "../../lib/types";

  let { cfg }: { cfg: ServerConfig } = $props();

  let instanceId = $state("");
  let diff = $state<ModDiff | null>(null);
  let comparing = $state(false);

  const modsDir = $derived(cfg.kind.type === "local" ? joinPath(cfg.kind.dir, "mods") : "");
  const instances = $derived(app.config?.instances ?? []);
  const instance = $derived(instances.find((i) => i.id === instanceId));

  async function compare() {
    if (!instance || !modsDir) return;
    comparing = true;
    try {
      diff = await api.modsDiff(modsDir, joinPath(instance.dir, "mods"));
    } catch (e) {
      errorToast(e);
    } finally {
      comparing = false;
    }
  }
</script>

{#if cfg.kind.type !== "local" || !cfg.kind.dir}
  <div class="notice">Pas d'accès aux fichiers de ce serveur depuis le panneau.</div>
{:else}
  <div class="stack">
    <ModList dir={modsDir} title="Mods du serveur" />

    <div class="card">
      <div class="card-head">
        <h2>Comparer avec une instance client</h2>
        <div class="row">
          <select class="input" bind:value={instanceId} style="min-width: 240px">
            <option value="">Choisir une instance…</option>
            {#each instances as i (i.id)}<option value={i.id}>{i.name}</option>{/each}
          </select>
          <button class="btn" onclick={compare} disabled={!instance || comparing}>Comparer</button>
        </div>
      </div>
      {#if diff && instance}
        <ModDiffView {diff} labelA="serveur" labelB="client" />
        <p class="muted small" style="margin-top: 10px">Un mod « seulement client » est normal pour les mods visuels (minimap, shaders). Un mod « seulement serveur » manquant côté client empêche souvent la connexion.</p>
      {:else if !instances.length}
        <div class="muted">Ajoute d'abord une instance dans la page Client.</div>
      {/if}
    </div>
  </div>
{/if}
