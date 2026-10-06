<script lang="ts">
  import type { ModDiff } from "../lib/types";

  let { diff, labelA, labelB }: { diff: ModDiff; labelA: string; labelB: string } = $props();
</script>

<div class="grid cols-3">
  <div class="card tight">
    <h3>Seulement {labelA} ({diff.only_a.length})</h3>
    <div class="list small">
      {#each diff.only_a as m (m.path)}
        <div class="list-item"><span class="ellipsis">{m.name}</span><span class="muted mono">{m.version}</span></div>
      {:else}
        <div class="muted" style="padding: 6px 0">Rien</div>
      {/each}
    </div>
  </div>
  <div class="card tight">
    <h3>Seulement {labelB} ({diff.only_b.length})</h3>
    <div class="list small">
      {#each diff.only_b as m (m.path)}
        <div class="list-item"><span class="ellipsis">{m.name}</span><span class="muted mono">{m.version}</span></div>
      {:else}
        <div class="muted" style="padding: 6px 0">Rien</div>
      {/each}
    </div>
  </div>
  <div class="card tight">
    <h3>Versions différentes ({diff.version_mismatch.length})</h3>
    <div class="list small">
      {#each diff.version_mismatch as [a, b] (a.path)}
        <div class="list-item">
          <span class="ellipsis">{a.name}</span>
          <span class="muted mono">{a.version || a.file_name} → {b.version || b.file_name}</span>
        </div>
      {:else}
        <div class="muted" style="padding: 6px 0">Rien</div>
      {/each}
    </div>
    <div class="muted small" style="margin-top: 8px">{diff.same} mods identiques</div>
  </div>
</div>
