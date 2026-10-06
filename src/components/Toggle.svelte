<script lang="ts">
  let {
    checked = $bindable(false),
    label = "",
    disabled = false,
    onchange,
  }: { checked?: boolean; label?: string; disabled?: boolean; onchange?: (value: boolean) => void } = $props();
</script>

<label class="toggle">
  <input type="checkbox" bind:checked {disabled} onchange={(e) => onchange?.(e.currentTarget.checked)} />
  {#if label}<span>{label}</span>{/if}
</label>
