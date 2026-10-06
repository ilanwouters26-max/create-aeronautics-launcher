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
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow);
    border-left-width: 3px;
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
    border-left-color: var(--info);
  }
</style>
