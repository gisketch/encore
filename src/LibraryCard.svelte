<script lang="ts">
  import { formatCardSubline, formatDuration, formatSavedTime } from "./libraryFormat";
  import type { LibraryEntry } from "./libraryTypes";

  let { entry, onOpen }: { entry: LibraryEntry; onOpen: (id: string) => void } = $props();
</script>

<button
  type="button"
  class="library-card"
  onclick={() => onOpen(entry.id)}
  aria-label={`Open ${entry.displayName} in the system player`}
>
  <span class="library-card__thumb">
    <span class="library-card__thumb-label">Screen frame</span>
    {#if entry.trimmed}
      <span class="library-card__badge library-card__badge--trimmed">Trimmed</span>
    {/if}
    {#if entry.durationSeconds}
      <span class="library-card__badge library-card__badge--duration">
        {formatDuration(entry.durationSeconds)}
      </span>
    {/if}
  </span>
  <span class="library-card__meta">
    <span class="library-card__time">{formatSavedTime(entry.savedAtUnixMs)}</span>
    <span class="library-card__sub">{formatCardSubline(entry.durationSeconds, entry.totalBytes)}</span>
  </span>
</button>
