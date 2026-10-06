<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../components/Icon.svelte";
  import Modal from "../components/Modal.svelte";
  import ModList from "../components/ModList.svelte";
  import ModDiffView from "../components/ModDiffView.svelte";
  import { api } from "../lib/api";
  import { newId } from "../lib/defaults";
  import { joinPath } from "../lib/format";
  import { app, errorToast, saveConfig, toast } from "../lib/stores.svelte";
  import type { DetectedInstance, InstanceConfig, ModDiff } from "../lib/types";

  let selectedId = $state("");
  let detecting = $state(false);
  let detected = $state<DetectedInstance[] | null>(null);
  let serverId = $state("");
  let diff = $state<ModDiff | null>(null);
  let comparing = $state(false);

  const instances = $derived(app.config?.instances ?? []);
  const selected = $derived(instances.find((i) => i.id === selectedId) ?? instances[0]);
  const servers = $derived((app.config?.servers ?? []).filter((s) => s.kind.type === "local"));
  const server = $derived(servers.find((s) => s.id === serverId));

  onMount(() => {
    try {
      selectedId = localStorage.getItem("client:selected") ?? "";
    } catch {
      selectedId = "";
    }
  });

  function select(id: string) {
    selectedId = id;
    diff = null;
    try {
      localStorage.setItem("client:selected", id);
    } catch {
      // ignoré
    }
  }

  async function detect() {
    detecting = true;
    try {
      detected = await api.instancesDetect();
    } catch (e) {
      errorToast(e);
    } finally {
      detecting = false;
    }
  }

  function alreadyAdded(dir: string): boolean {
    return instances.some((i) => i.dir.toLowerCase() === dir.toLowerCase());
  }

  async function add(d: DetectedInstance) {
    const inst: InstanceConfig = { id: newId(), name: d.name, dir: d.dir, source: d.source, loader: d.loader, mc_version: d.mc_version };
    try {
      await saveConfig((c) => c.instances.push(inst));
      select(inst.id);
      toast(`${inst.name} ajoutée`, "ok");
    } catch (e) {
      errorToast(e);
    }
  }

  async function pickManual() {
    const dir = await api.pickFolder("Dossier de l'instance (celui qui contient mods/)");
    if (!dir) return;
    try {
      const d = await api.instanceInspect(dir);
      await add(d);
    } catch (e) {
      errorToast(e);
    }
  }

  async function remove(inst: InstanceConfig) {
    try {
      await saveConfig((c) => {
        c.instances = c.instances.filter((i) => i.id !== inst.id);
      });
      toast(`${inst.name} retirée du panneau`, "ok");
    } catch (e) {
      errorToast(e);
    }
  }

  async function compare() {
    if (!selected || !server || server.kind.type !== "local") return;
    comparing = true;
    try {
      diff = await api.modsDiff(joinPath(selected.dir, "mods"), joinPath(server.kind.dir, "mods"));
    } catch (e) {
      errorToast(e);
    } finally {
      comparing = false;
    }
  }

  function sourceLabel(s: string): string {
    return s === "curseforge" ? "CurseForge" : s === "official" ? "Launcher officiel" : s === "prism" ? "Prism" : "Dossier";
  }
</script>

<div class="page-head">
  <div>
    <h1>Client</h1>
    <p class="sub">Instances Minecraft sur ce PC et leurs mods</p>
  </div>
  <div class="actions">
    <button class="btn" onclick={detect} disabled={detecting}><Icon name="search" size={14} /> Détecter les instances</button>
    <button class="btn" onclick={pickManual}><Icon name="folder" size={14} /> Choisir un dossier</button>
  </div>
</div>

{#if instances.length === 0}
  <div class="empty">
    <p>Aucune instance enregistrée.</p>
    <p class="small" style="margin-top: 6px">Détecte les instances CurseForge / launcher officiel / Prism, ou choisis un dossier à la main.</p>
  </div>
{:else}
  <div class="instances">
    {#each instances as inst (inst.id)}
      <div class="card tight inst" class:active={selected?.id === inst.id} role="button" tabindex="0" onclick={() => select(inst.id)} onkeydown={(e) => { if (e.key === "Enter") select(inst.id); }}>
        <div class="row between">
          <strong class="ellipsis">{inst.name}</strong>
          <span class="badge">{sourceLabel(inst.source)}</span>
        </div>
        <div class="muted small ellipsis mono" title={inst.dir}>{inst.dir}</div>
        <div class="muted small" style="margin-top: 4px">{[inst.loader, inst.mc_version].filter(Boolean).join(" ") || "loader inconnu"}</div>
      </div>
    {/each}
  </div>

  {#if selected}
    <div class="row between" style="margin: 16px 0 10px">
      <h2>{selected.name}</h2>
      <div class="actions">
        <button class="btn sm" onclick={() => api.openPath(selected.dir).catch(errorToast)}><Icon name="folder" size={13} /> Dossier de l'instance</button>
        <button class="btn ghost sm danger" onclick={() => remove(selected)}>Retirer</button>
      </div>
    </div>
    {#key selected.id}
      <ModList dir={joinPath(selected.dir, "mods")} title="Mods client" />
    {/key}

    <div class="card" style="margin-top: 14px">
      <div class="card-head">
        <h2>Comparer avec un serveur</h2>
        <div class="row">
          <select class="input" bind:value={serverId} style="min-width: 240px">
            <option value="">Choisir un serveur local…</option>
            {#each servers as s (s.id)}<option value={s.id}>{s.name}</option>{/each}
          </select>
          <button class="btn" onclick={compare} disabled={!server || comparing}>Comparer</button>
        </div>
      </div>
      {#if diff}
        <ModDiffView {diff} labelA="client" labelB="serveur" />
      {/if}
    </div>
  {/if}
{/if}

<Modal open={detected !== null} title="Instances détectées" onclose={() => (detected = null)} width={720}>
  {#if detected?.length}
    <div class="list">
      {#each detected as d (d.dir)}
        <div class="list-item">
          <div style="flex: 1; min-width: 0">
            <div class="row"><strong>{d.name}</strong><span class="badge">{sourceLabel(d.source)}</span><span class="muted small">{[d.loader, d.mc_version].filter(Boolean).join(" ")}</span></div>
            <div class="muted small mono ellipsis" title={d.dir}>{d.dir}</div>
          </div>
          <span class="muted small">{d.mods_count} mods</span>
          {#if alreadyAdded(d.dir)}
            <span class="badge ok">Ajoutée</span>
          {:else}
            <button class="btn sm" onclick={() => add(d)}><Icon name="plus" size={13} /> Ajouter</button>
          {/if}
        </div>
      {/each}
    </div>
  {:else}
    <p class="muted">Rien trouvé dans les emplacements habituels (curseforge\minecraft\Instances, .minecraft, PrismLauncher). Utilise « Choisir un dossier ».</p>
  {/if}
</Modal>

<style>
  .instances {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 12px;
  }

  .inst {
    cursor: pointer;
  }

  .inst.active {
    border-color: var(--accent);
  }
</style>
