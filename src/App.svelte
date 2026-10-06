<script lang="ts">
  import { onMount } from "svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import Toasts from "./components/Toasts.svelte";
  import Modal from "./components/Modal.svelte";
  import Dashboard from "./pages/Dashboard.svelte";
  import ServerPage from "./pages/ServerPage.svelte";
  import ClientPage from "./pages/ClientPage.svelte";
  import LauncherPage from "./pages/LauncherPage.svelte";
  import SystemPage from "./pages/SystemPage.svelte";
  import { api } from "./lib/api";
  import { installDragDrop } from "./lib/dragdrop.svelte";
  import { app, errorToast, loadAll, restoreRoute, subscribeEvents } from "./lib/stores.svelte";

  onMount(() => {
    (async () => {
      restoreRoute();
      await subscribeEvents();
      await loadAll();
      await installDragDrop();
    })();
  });

  const page = $derived(app.route.page);

  async function quit(stop: boolean) {
    try {
      await api.appQuit(stop);
    } catch (e) {
      errorToast(e);
    }
    app.quitPrompt = null;
  }
</script>

{#if app.error}
  <div class="boot">
    <div class="notice err">Le panneau n'a pas pu démarrer : {app.error}</div>
  </div>
{:else if !app.ready}
  <div class="boot muted">Chargement…</div>
{:else}
  <div class="app">
    <Sidebar />
    <main class="page">
      {#if page === "dashboard"}
        <Dashboard />
      {:else if page === "server"}
        {#key app.route.id}
          <ServerPage id={app.route.id} />
        {/key}
      {:else if page === "client"}
        <ClientPage />
      {:else if page === "launcher"}
        <LauncherPage />
      {:else}
        <SystemPage />
      {/if}
    </main>
  </div>
{/if}

<Toasts />

<Modal open={app.quitPrompt !== null} title="Quitter Telek Panel" onclose={() => (app.quitPrompt = null)}>
  <p>Des serveurs tournent encore : <strong>{app.quitPrompt?.join(", ")}</strong>.</p>
  <p class="muted">Si tu quittes sans les arrêter, ils continuent de tourner mais le panneau ne pourra plus s'y rattacher (console, arrêt propre, horaires).</p>
  {#snippet footer()}
    <button class="btn" onclick={() => (app.quitPrompt = null)}>Annuler</button>
    <button class="btn" onclick={() => quit(false)}>Quitter sans arrêter</button>
    <button class="btn primary" onclick={() => quit(true)}>Arrêter les serveurs puis quitter</button>
  {/snippet}
</Modal>

<style>
  .boot {
    height: 100vh;
    display: grid;
    place-items: center;
    padding: 40px;
  }
</style>
