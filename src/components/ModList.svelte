<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";
  import { api } from "../lib/api";
  import { browserDropProps, drag, registerDropZone } from "../lib/dragdrop.svelte";
  import { bytes } from "../lib/format";
  import { errorToast, toast } from "../lib/stores.svelte";
  import type { ModInfo, ModsListing } from "../lib/types";

  let { dir, title = "Mods" }: { dir: string; title?: string } = $props();

  let listing = $state<ModsListing | null>(null);
  let loading = $state(false);
  let search = $state("");
  let onlyDisabled = $state(false);
  let toDelete = $state<ModInfo | null>(null);

  const zoneId = $derived(`mods:${dir}`);
  const mods = $derived.by(() => {
    const q = search.trim().toLowerCase();
    return (listing?.mods ?? []).filter((m) => {
      if (onlyDisabled && m.enabled) return false;
      if (!q) return true;
      return m.name.toLowerCase().includes(q) || m.file_name.toLowerCase().includes(q) || m.id.toLowerCase().includes(q);
    });
  });
  const enabledCount = $derived((listing?.mods ?? []).filter((m) => m.enabled).length);
  const totalSize = $derived((listing?.mods ?? []).reduce((n, m) => n + m.size, 0));

  export async function reload() {
    loading = true;
    try {
      listing = await api.modsList(dir);
    } catch (e) {
      errorToast(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    reload();
  });

  $effect(() => {
    const unregister = registerDropZone(zoneId, (paths) => importFiles(paths));
    return unregister;
  });

  async function importFiles(paths: string[]) {
    try {
      const results = await api.modsImport(dir, paths);
      const ok = results.filter((r) => r.ok).length;
      const bad = results.filter((r) => !r.ok);
      if (ok) toast(`${ok} mod${ok > 1 ? "s" : ""} ajouté${ok > 1 ? "s" : ""}`, "ok");
      for (const b of bad) toast(`${b.file} : ${b.message}`, "warn");
      await reload();
    } catch (e) {
      errorToast(e);
    }
  }

  async function pickAndImport() {
    const files = await api.pickJars("Ajouter des mods");
    if (files.length) await importFiles(files);
  }

  async function toggle(m: ModInfo) {
    try {
      await api.modsSetEnabled(m.path, !m.enabled);
      await reload();
    } catch (e) {
      errorToast(e);
    }
  }

  async function confirmDelete() {
    if (!toDelete) return;
    try {
      await api.modsDelete(toDelete.path);
      toast(`${toDelete.file_name} supprimé`, "ok");
      toDelete = null;
      await reload();
    } catch (e) {
      errorToast(e);
    }
  }

  async function openDir() {
    try {
      await api.openPath(dir);
    } catch (e) {
      errorToast(e);
    }
  }
</script>

<div class="card modlist">
  <div class="card-head">
    <div class="row">
      <h2>{title}</h2>
      {#if listing}
        <span class="badge">{listing.mods.length} mods</span>
        <span class="badge ok">{enabledCount} actifs</span>
        {#if listing.mods.length - enabledCount > 0}<span class="badge off">{listing.mods.length - enabledCount} désactivés</span>{/if}
        <span class="muted small">{bytes(totalSize)}</span>
      {/if}
    </div>
    <div class="actions">
      <button class="btn sm" onclick={pickAndImport}><Icon name="plus" size={14} /> Ajouter</button>
      <button class="btn sm" onclick={openDir}><Icon name="folder" size={14} /> Ouvrir</button>
      <button class="btn ghost sm" onclick={reload} disabled={loading}><Icon name="refresh" size={14} /></button>
    </div>
  </div>

  {#if listing && !listing.dir_exists}
    <div class="notice warn">Le dossier <span class="mono">{dir}</span> n'existe pas encore. Dépose un mod pour le créer.</div>
  {/if}

  {#if listing && listing.duplicates.length}
    <div class="notice warn">
      <strong>Doublons</strong> : {#each listing.duplicates as d (d.key)}<span class="mono">{d.files.join(" / ")}</span>{" "}{/each}
    </div>
  {/if}

  <div class="row" style="margin: 10px 0">
    <div class="search">
      <Icon name="search" size={14} />
      <input class="input" placeholder="Chercher un mod" bind:value={search} />
    </div>
    <label class="toggle small"><input type="checkbox" bind:checked={onlyDisabled} /> Désactivés seulement</label>
  </div>

  <div class="dropzone" class:over={drag.overZone === zoneId} data-dropzone={zoneId} {...browserDropProps(zoneId)}>
    {#if drag.active}
      Dépose les .jar ici pour les ajouter à {title.toLowerCase()}
    {:else}
      Glisse des fichiers .jar depuis l'explorateur pour les ajouter
    {/if}
  </div>

  {#if mods.length}
    <table class="table" style="margin-top: 10px">
      <thead>
        <tr><th></th><th>Mod</th><th>Version</th><th>Loader</th><th>Côté</th><th>Taille</th><th></th></tr>
      </thead>
      <tbody>
        {#each mods as m (m.path)}
          <tr class:disabled={!m.enabled}>
            <td style="width: 42px">
              <label class="toggle" title={m.enabled ? "Désactiver (renomme en .disabled)" : "Activer"}>
                <input type="checkbox" checked={m.enabled} onchange={() => toggle(m)} />
              </label>
            </td>
            <td>
              <div class="name">{m.name}</div>
              <div class="muted small mono ellipsis" title={m.file_name}>{m.file_name}</div>
            </td>
            <td class="mono small">{m.version || "—"}</td>
            <td>{#if m.loader}<span class="badge">{m.loader}</span>{/if}</td>
            <td>
              {#if m.environment === "client"}<span class="badge info">client</span>
              {:else if m.environment === "server"}<span class="badge warn">serveur</span>
              {:else if m.environment === "both"}<span class="badge">les deux</span>{/if}
            </td>
            <td class="muted small">{bytes(m.size)}</td>
            <td style="width: 40px"><button class="btn ghost icon sm" title="Supprimer" onclick={() => (toDelete = m)}><Icon name="trash" size={14} /></button></td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else if listing}
    <div class="empty" style="margin-top: 10px">{search || onlyDisabled ? "Aucun mod ne correspond au filtre" : "Aucun mod dans ce dossier"}</div>
  {/if}
</div>

<Modal open={toDelete !== null} title="Supprimer ce mod ?" onclose={() => (toDelete = null)}>
  <p>Le fichier <span class="mono">{toDelete?.file_name}</span> sera supprimé du disque. Pour le garder sans le charger, désactive-le plutôt.</p>
  {#snippet footer()}
    <button class="btn" onclick={() => (toDelete = null)}>Annuler</button>
    <button class="btn danger" onclick={confirmDelete}>Supprimer</button>
  {/snippet}
</Modal>

<style>
  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    flex: 1;
    max-width: 340px;
  }

  tr.disabled .name {
    color: var(--muted);
    text-decoration: line-through;
  }

  .name {
    font-weight: 500;
  }
</style>
