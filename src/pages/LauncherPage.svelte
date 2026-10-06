<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../components/Icon.svelte";
  import Modal from "../components/Modal.svelte";
  import Toggle from "../components/Toggle.svelte";
  import { api, on } from "../lib/api";
  import { defaultLauncher } from "../lib/defaults";
  import { bytes, dateTime, joinPath, relative } from "../lib/format";
  import { app, errorToast, saveConfig, toast } from "../lib/stores.svelte";
  import type { FileDiff, LauncherConfig, LauncherScan, Progress, PublishReport } from "../lib/types";

  let selectedId = $state("");
  let scan = $state<LauncherScan | null>(null);
  let scanning = $state(false);
  let filter = $state<"all" | "changed">("all");
  let version = $state("");
  let push = $state(true);
  let editing = $state<LauncherConfig | null>(null);
  let isNew = $state(false);
  let confirmDelete = $state(false);
  let progress = $state({ active: false, step: 0, total: 0, label: "", lines: [] as string[], done: false, ok: true, message: "" });
  let logEl: HTMLDivElement | undefined = $state();

  const launchers = $derived(app.config?.launchers ?? []);
  const launcher = $derived(launchers.find((l) => l.id === selectedId) ?? launchers[0]);
  const files = $derived((scan?.files ?? []).filter((f) => filter === "all" || f.status !== "unchanged"));
  const changedCount = $derived((scan?.files ?? []).filter((f) => f.status !== "unchanged").length);
  const publishedSize = $derived((scan?.files ?? []).filter((f) => f.status !== "removed" && !isExcluded(f.path)).reduce((n, f) => n + f.size, 0));

  onMount(() => {
    try {
      selectedId = localStorage.getItem("launcher:selected") ?? "";
    } catch {
      selectedId = "";
    }
    let unlistenProgress: (() => void) | undefined;
    let unlistenDone: (() => void) | undefined;
    on<{ launcher_id: string; progress: Progress }>("publish:progress", (p) => {
      if (p.launcher_id !== launcher?.id) return;
      const pr = p.progress;
      if (pr.kind === "step") {
        progress.step = pr.index;
        progress.total = pr.total;
        progress.label = pr.label;
        progress.lines.push(`— ${pr.label}`);
      } else if (pr.kind === "line") {
        progress.lines.push(pr.text);
      } else {
        progress.done = true;
        progress.ok = pr.ok;
        progress.message = pr.message;
      }
      scrollLog();
    }).then((u) => (unlistenProgress = u));
    on<{ launcher_id: string; report: PublishReport | null; error: string }>("publish:done", (d) => {
      if (d.launcher_id !== launcher?.id) return;
      progress.active = false;
      progress.done = true;
      if (d.error) {
        progress.ok = false;
        progress.message = d.error;
        toast(`Publication échouée : ${d.error}`, "err", 8000);
      } else if (d.report) {
        toast(d.report.message, "ok");
      }
      load();
    }).then((u) => (unlistenDone = u));
    return () => {
      unlistenProgress?.();
      unlistenDone?.();
    };
  });

  $effect(() => {
    if (launcher?.id) load();
  });

  function scrollLog() {
    requestAnimationFrame(() => {
      if (logEl) logEl.scrollTop = logEl.scrollHeight;
    });
  }

  function select(id: string) {
    selectedId = id;
    scan = null;
    try {
      localStorage.setItem("launcher:selected", id);
    } catch {
      // ignoré
    }
  }

  async function load() {
    if (!launcher) return;
    scanning = true;
    try {
      scan = await api.launcherScan(launcher.id);
      if (scan.publishing) progress.active = true;
    } catch (e) {
      errorToast(e);
    } finally {
      scanning = false;
    }
  }

  function isExcluded(path: string): boolean {
    return launcher?.excluded.includes(path) ?? false;
  }

  async function toggleExcluded(f: FileDiff) {
    if (!launcher) return;
    const id = launcher.id;
    const excluded = !isExcluded(f.path);
    try {
      await saveConfig((c) => {
        const l = c.launchers.find((x) => x.id === id);
        if (!l) return;
        l.excluded = excluded ? [...l.excluded, f.path] : l.excluded.filter((p) => p !== f.path);
      });
    } catch (e) {
      errorToast(e);
    }
  }

  async function publish() {
    if (!launcher) return;
    progress = { active: true, step: 0, total: 0, label: "Démarrage", lines: [], done: false, ok: true, message: "" };
    try {
      await api.launcherPublish(launcher.id, version.trim() || null, push);
    } catch (e) {
      progress.active = false;
      errorToast(e);
    }
  }

  function openNew() {
    editing = defaultLauncher();
    isNew = true;
  }

  function openEdit() {
    if (!launcher) return;
    editing = structuredClone($state.snapshot(launcher)) as LauncherConfig;
    isNew = false;
  }

  async function pickProject() {
    if (!editing) return;
    const dir = await api.pickFolder("Dossier du projet launcher (racine git)");
    if (dir) editing.project_dir = dir;
  }

  async function saveEdit() {
    if (!editing) return;
    const snapshot = $state.snapshot(editing) as LauncherConfig;
    try {
      await saveConfig((c) => {
        const i = c.launchers.findIndex((l) => l.id === snapshot.id);
        if (i >= 0) c.launchers[i] = snapshot;
        else c.launchers.push(snapshot);
      });
      select(snapshot.id);
      editing = null;
      toast("Launcher enregistré", "ok");
      load();
    } catch (e) {
      errorToast(e);
    }
  }

  async function deleteLauncher() {
    if (!launcher) return;
    const id = launcher.id;
    try {
      await saveConfig((c) => {
        c.launchers = c.launchers.filter((l) => l.id !== id);
      });
      confirmDelete = false;
      select("");
      toast("Launcher retiré du panneau", "ok");
    } catch (e) {
      errorToast(e);
    }
  }

  function statusBadge(s: FileDiff["status"]): string {
    return s === "new" ? "ok" : s === "changed" ? "warn" : s === "removed" ? "err" : "";
  }

  function statusLabel(s: FileDiff["status"]): string {
    return s === "new" ? "nouveau" : s === "changed" ? "modifié" : s === "removed" ? "retiré" : "inchangé";
  }
</script>

<div class="page-head">
  <div>
    <h1>Launcher</h1>
    <p class="sub">Publication du modpack sur GitHub pour les launchers</p>
  </div>
  <div class="actions">
    {#if launcher}
      <button class="btn" onclick={openEdit}><Icon name="edit" size={14} /> Configurer</button>
    {/if}
    <button class="btn" onclick={openNew}><Icon name="plus" size={14} /> Ajouter un launcher</button>
  </div>
</div>

{#if launchers.length === 0}
  <div class="empty">
    <p>Aucun launcher configuré.</p>
    <p class="small" style="margin-top: 6px">Ajoute le dossier d'un projet launcher (celui qui contient « release files » et le dépôt git).</p>
  </div>
{:else}
  <div class="tabs">
    {#each launchers as l (l.id)}
      <button class:active={launcher?.id === l.id} onclick={() => select(l.id)}><Icon name="upload" size={14} /> {l.name}</button>
    {/each}
  </div>

  {#if launcher}
    <div class="layout">
      <div class="stack">
        <div class="card">
          <div class="card-head" style="flex-wrap: wrap">
            <div class="row wrap">
              <h2 style="white-space: nowrap">Fichiers de release</h2>
              {#if scan}<span class="badge">{scan.files.length} fichiers</span>{#if changedCount}<span class="badge warn">{changedCount} à publier</span>{/if}<span class="muted small">{bytes(publishedSize)} publiés</span>{/if}
            </div>
            <div class="actions">
              <div class="tabs" style="margin: 0; border: 0">
                <button class:active={filter === "all"} onclick={() => (filter = "all")}>Tous</button>
                <button class:active={filter === "changed"} onclick={() => (filter = "changed")}>Changements</button>
              </div>
              <button class="btn ghost sm" onclick={() => api.openPath(scan?.release_dir ?? joinPath(launcher.project_dir, launcher.release_dir)).catch(errorToast)} title="Ouvrir release files"><Icon name="folder" size={14} /></button>
              <button class="btn ghost sm" onclick={load} disabled={scanning}><Icon name="refresh" size={14} /></button>
            </div>
          </div>
          {#if scan?.error}
            <div class="notice err">{scan.error}</div>
          {:else if scanning && !scan}
            <div class="muted">Analyse des fichiers (empreintes SHA-1)…</div>
          {:else if files.length}
            <table class="table">
              <thead><tr><th>Publier</th><th>Fichier</th><th>État</th><th>Taille</th></tr></thead>
              <tbody>
                {#each files as f (f.path)}
                  <tr class:removed={f.status === "removed"}>
                    <td style="width: 60px">
                      {#if f.status !== "removed"}
                        <label class="toggle" title={isExcluded(f.path) ? "Exclu du manifest" : "Inclus dans le manifest"}><input type="checkbox" checked={!isExcluded(f.path)} onchange={() => toggleExcluded(f)} /></label>
                      {/if}
                    </td>
                    <td><div class="mono small">{f.path}</div></td>
                    <td><span class="badge {statusBadge(f.status)}">{statusLabel(f.status)}</span></td>
                    <td class="muted small">{bytes(f.size)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <div class="muted">{filter === "changed" ? "Rien n'a changé depuis le dernier manifest." : "Aucun fichier dans le dossier de release."}</div>
          {/if}
          <p class="muted small" style="margin-top: 10px">Décocher un fichier le laisse sur le disque mais le retire du manifest. Le manifest généré est générique (chemin, taille, sha1, url) : à aligner sur le format lu par le launcher.</p>
        </div>
      </div>

      <div class="stack">
        <div class="card">
          <div class="card-head"><h2>Publier</h2></div>
          <div class="stack" style="gap: 10px">
            <div class="field">
              <label for="pub-version">Version (vide = date du jour)</label>
              <input id="pub-version" class="input mono" bind:value={version} placeholder="2026.10.06-2114" />
            </div>
            <Toggle bind:checked={push} label="Pousser sur GitHub (git push)" />
            <button class="btn primary" onclick={publish} disabled={progress.active || !!scan?.error}><Icon name="upload" size={14} /> {progress.active ? "Publication en cours…" : "Publier le modpack"}</button>
            <div class="muted small">Séquence : analyse → manifest → git add → git commit → git push sur <span class="mono">{launcher.branch}</span>.</div>
          </div>
          {#if progress.active || progress.done}
            <div class="progress-box">
              <div class="row between small" style="margin-bottom: 6px">
                <span>{progress.done ? (progress.ok ? "Terminé" : "Échec") : progress.label}</span>
                <span class="muted">{progress.total ? `${progress.step}/${progress.total}` : ""}</span>
              </div>
              <div class="progress" class:warn={progress.done && !progress.ok}><span style:width="{progress.done ? 100 : progress.total ? (progress.step / progress.total) * 100 : 5}%"></span></div>
              <div class="log mono" bind:this={logEl}>
                {#each progress.lines as l, i (i)}<div>{l}</div>{/each}
                {#if progress.done}<div class={progress.ok ? "ok" : "err"}>{progress.message}</div>{/if}
              </div>
            </div>
          {/if}
        </div>

        <div class="card">
          <div class="card-head"><h2>Dépôt git</h2><button class="btn ghost sm" onclick={() => api.openPath(launcher.project_dir).catch(errorToast)}><Icon name="folder" size={14} /></button></div>
          {#if scan}
            {#if scan.git.error}
              <div class="notice warn">{scan.git.error}</div>
            {:else}
              <div class="stack small" style="gap: 6px">
                <div class="row between"><span class="muted">Branche</span><span class="mono">{scan.git.branch}</span></div>
                <div class="row between"><span class="muted">Remote</span><span class="mono ellipsis" style="max-width: 260px" title={scan.git.remote}>{scan.git.remote || "—"}</span></div>
                <div class="row between"><span class="muted">Dernier commit</span><span class="ellipsis" style="max-width: 260px" title={scan.git.last_commit}>{scan.git.last_commit || "—"}</span></div>
                <div class="row between"><span class="muted">Fichiers non commités</span><span>{scan.git.dirty_count}</span></div>
                {#if scan.manifest}<div class="row between"><span class="muted">Manifest</span><span class="mono">{scan.manifest.version} · {scan.manifest.files.length} entrées</span></div>{/if}
              </div>
            {/if}
          {/if}
        </div>

        <div class="card">
          <div class="card-head"><h2>Publications</h2></div>
          {#if scan?.history.length}
            <div class="list small">
              {#each scan.history as h (h.id)}
                <div class="list-item">
                  <span class="badge {h.ok ? 'ok' : 'err'}">{h.ok ? "ok" : "échec"}</span>
                  <div style="flex: 1; min-width: 0">
                    <div class="mono">{h.version} · {h.files} fichiers, {h.changed} modifiés</div>
                    <div class="muted ellipsis" title={h.message}>{h.message}</div>
                  </div>
                  <span class="muted" title={dateTime(h.ts)}>{relative(h.ts)}</span>
                </div>
              {/each}
            </div>
          {:else}
            <div class="muted small">Aucune publication enregistrée.</div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
{/if}

<Modal open={editing !== null} title={isNew ? "Nouveau launcher" : "Configurer le launcher"} onclose={() => (editing = null)} width={720}>
  {#if editing}
    <div class="form-grid">
      <div class="field"><label for="l-name">Nom</label><input id="l-name" class="input" bind:value={editing.name} /></div>
      <div class="field"><label for="l-branch">Branche git</label><input id="l-branch" class="input mono" bind:value={editing.branch} /></div>
      <div class="field full">
        <label for="l-dir">Dossier du projet (racine git)</label>
        <div class="input-row"><input id="l-dir" class="input mono" bind:value={editing.project_dir} placeholder="C:\TelekGamer\Launcher Create" /><button class="btn" onclick={pickProject}><Icon name="folder" size={14} /></button></div>
      </div>
      <div class="field"><label for="l-rel">Dossier des fichiers publiés</label><input id="l-rel" class="input mono" bind:value={editing.release_dir} /><span class="hint">Relatif au projet</span></div>
      <div class="field"><label for="l-dist">Dossier de build du launcher</label><input id="l-dist" class="input mono" bind:value={editing.dist_dir} /><span class="hint">Informatif (bouton d'ouverture)</span></div>
      <div class="field"><label for="l-man">Fichier manifest</label><input id="l-man" class="input mono" bind:value={editing.manifest_path} /></div>
      <div class="field"><label for="l-msg">Message de commit</label><input id="l-msg" class="input" bind:value={editing.commit_message} /><span class="hint">{"{date}"} et {"{version}"} sont remplacés</span></div>
      <div class="field full"><label for="l-url">URL de base des téléchargements</label><input id="l-url" class="input mono" bind:value={editing.base_url} placeholder="https://raw.githubusercontent.com/<user>/<repo>/main/release files" /><span class="hint">Préfixe des URL écrites dans le manifest</span></div>
      <div class="field full"><label for="l-ign">Suffixes ignorés au scan</label><input id="l-ign" class="input mono" value={editing.ignore_suffixes.join(", ")} onchange={(e) => { if (editing) editing.ignore_suffixes = e.currentTarget.value.split(",").map((s) => s.trim()).filter(Boolean); }} /></div>
    </div>
    {#if !isNew}
      <div class="row between" style="margin-top: 8px">
        <span class="muted small">Retirer ce launcher du panneau ne touche pas aux fichiers.</span>
        <button class="btn ghost sm danger" onclick={() => (confirmDelete = true)}>Retirer</button>
      </div>
    {/if}
  {/if}
  {#snippet footer()}
    <button class="btn" onclick={() => (editing = null)}>Annuler</button>
    <button class="btn primary" onclick={saveEdit} disabled={!editing?.name.trim() || !editing?.project_dir.trim()}><Icon name="check" size={14} /> Enregistrer</button>
  {/snippet}
</Modal>

<Modal open={confirmDelete} title="Retirer le launcher ?" onclose={() => (confirmDelete = false)}>
  <p>{launcher?.name} disparaîtra du panneau. Le projet et le dépôt git restent intacts.</p>
  {#snippet footer()}
    <button class="btn" onclick={() => (confirmDelete = false)}>Annuler</button>
    <button class="btn danger" onclick={() => { editing = null; deleteLauncher(); }}>Retirer</button>
  {/snippet}
</Modal>

<style>
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1.6fr) minmax(300px, 1fr);
    gap: 14px;
    align-items: start;
  }

  tr.removed td {
    opacity: 0.55;
  }

  .progress-box {
    margin-top: 14px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }

  .log {
    margin-top: 8px;
    max-height: 220px;
    overflow: auto;
    background: rgba(3, 7, 14, 0.9);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 8px 10px;
    font-size: 12px;
    color: var(--text-2);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .log .ok {
    color: var(--ok);
  }

  .log .err {
    color: var(--err);
  }
</style>
