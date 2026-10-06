<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  let {
    open,
    title,
    onclose,
    children,
    footer,
    width = 540,
  }: { open: boolean; title: string; onclose: () => void; children?: Snippet; footer?: Snippet; width?: number } = $props();

  function onkeydown(e: KeyboardEvent) {
    if (open && e.key === "Escape") onclose();
  }
</script>

<svelte:window {onkeydown} />

{#if open}
  <div class="backdrop" role="presentation" onclick={onclose}>
    <div class="modal" style:width="{width}px" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="dialog" tabindex="-1" aria-modal="true" aria-label={title}>
      <div class="modal-head">
        <h2>{title}</h2>
        <button class="btn ghost icon" onclick={onclose} aria-label="Fermer"><Icon name="x" /></button>
      </div>
      <div class="modal-body">{@render children?.()}</div>
      {#if footer}
        <div class="modal-foot">{@render footer()}</div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(2, 5, 10, 0.72);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
    padding: 20px;
  }

  .modal {
    position: relative;
    max-width: 100%;
    max-height: calc(100vh - 40px);
    display: flex;
    flex-direction: column;
    background: linear-gradient(180deg, rgba(14, 30, 54, 0.98), rgba(7, 14, 27, 0.99));
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow), 0 0 40px rgba(34, 200, 255, 0.1);
    outline: none;
  }

  .modal::before,
  .modal::after {
    content: "";
    position: absolute;
    width: 18px;
    height: 18px;
    border: 0 solid var(--accent);
    pointer-events: none;
  }

  .modal::before {
    top: -1px;
    left: -1px;
    border-width: 2px 0 0 2px;
  }

  .modal::after {
    bottom: -1px;
    right: -1px;
    border-width: 0 2px 2px 0;
  }

  .modal-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px 10px;
    border-bottom: 1px solid var(--border);
  }

  .modal-body {
    padding: 16px;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .modal-foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
  }
</style>
