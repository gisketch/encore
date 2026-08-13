<script lang="ts">
  import LibraryCard from "./LibraryCard.svelte";
  import { formatRollup } from "./libraryFormat";
  import type { LibraryGroup } from "./libraryTypes";

  let {
    group,
    tint,
    expanded,
    onExpand,
    onOpen,
  }: {
    group: LibraryGroup;
    tint: number;
    expanded: boolean;
    onExpand: () => void;
    onOpen: (id: string) => void;
  } = $props();
</script>

<section class="library-group" aria-label={group.label}>
  <header class="library-group__header">
    <span class="library-chip library-chip--{tint}">{group.label}</span>
    <span class="library-rollup">{formatRollup(group.count, group.totalBytes)}</span>
    <span class="library-group__rule" aria-hidden="true"></span>
  </header>

  {#if expanded}
    <div class="library-grid">
      {#each group.entries as entry (entry.id)}
        <LibraryCard {entry} {onOpen} />
      {/each}
    </div>
  {:else}
    <button type="button" class="library-pill" onclick={onExpand}>Show</button>
  {/if}
</section>
