<script lang="ts">
  import { app } from "../lib/stores.svelte";
</script>

<div class="toasts" aria-live="polite">
  {#each app.toasts as t (t.id)}
    <div class="toast {t.kind}">{t.message}</div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 18px;
    bottom: 18px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 60;
    max-width: 420px;
  }

  .toast {
    padding: 10px 14px;
    background: rgba(8, 16, 30, 0.97);
    border: 1px solid var(--border-strong);
    border-left-width: 3px;
    box-shadow: var(--shadow);
    clip-path: polygon(0 0, 100% 0, 100% calc(100% - 8px), calc(100% - 8px) 100%, 0 100%);
  }

  .toast.ok {
    border-left-color: var(--ok);
  }

  .toast.warn {
    border-left-color: var(--warn);
  }

  .toast.err {
    border-left-color: var(--err);
  }

  .toast.info {
    border-left-color: var(--accent);
  }
</style>
