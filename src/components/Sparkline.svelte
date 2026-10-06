<script lang="ts">
  let {
    values,
    max = 0,
    color = "var(--accent)",
    height = 44,
    fill = true,
  }: { values: number[]; max?: number; color?: string; height?: number; fill?: boolean } = $props();

  const width = 200;

  const points = $derived.by(() => {
    if (!values.length) return "";
    const top = Math.max(max, ...values, 1e-9);
    const n = values.length;
    return values
      .map((v, i) => {
        const x = n === 1 ? width : (i / (n - 1)) * width;
        const y = height - 2 - (Math.min(v, top) / top) * (height - 4);
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
  });

  const area = $derived(points ? `0,${height} ${points} ${width},${height}` : "");
</script>

<svg viewBox="0 0 {width} {height}" preserveAspectRatio="none" style:height="{height}px" class="spark" aria-hidden="true">
  {#if fill && area}
    <polygon points={area} fill={color} opacity="0.12" />
  {/if}
  {#if points}
    <polyline points={points} fill="none" stroke={color} stroke-width="5" opacity="0.18" vector-effect="non-scaling-stroke" />
    <polyline points={points} fill="none" stroke={color} stroke-width="1.6" vector-effect="non-scaling-stroke" />
  {/if}
</svg>

<style>
  .spark {
    width: 100%;
    display: block;
  }
</style>
