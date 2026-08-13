<script lang="ts">
  import { onMount } from "svelte";
  import LibraryCardDelete from "./LibraryCardDelete.svelte";
  import { formatCardSubline, formatDuration, formatSavedTime } from "./libraryFormat";
  import { fetchThumbnailDataUrl } from "./libraryThumbnail";
  import type { LibraryEntry } from "./libraryTypes";

  let {
    entry,
    onOpen,
    onDeleted,
  }: { entry: LibraryEntry; onOpen: (id: string) => void; onDeleted: (id: string) => void } =
    $props();
  let thumbnailUrl = $state<string | null>(null);

  // Fetched after this card has already mounted (i.e. after the list
  // itself rendered) so a slow or missing thumbnail never blocks the
  // index; a null result keeps the styled placeholder permanently.
  onMount(() => {
    void fetchThumbnailDataUrl(entry.id).then((url) => {
      thumbnailUrl = url;
    });
  });
</script>

<div class="library-card__wrap">
  <button
    type="button"
    class="library-card"
    onclick={() => onOpen(entry.id)}
    aria-label={`Open ${entry.displayName} in the editor`}
  >
    <span class="library-card__thumb">
      <span class="library-card__hover-hint" aria-hidden="true">Open in editor</span>
      {#if thumbnailUrl}
        <img class="library-card__thumb-image" src={thumbnailUrl} alt="" />
      {:else}
        <span class="library-card__thumb-label">Screen frame</span>
      {/if}
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
      <span class="library-card__sub"
        >{formatCardSubline(entry.durationSeconds, entry.totalBytes)}</span
      >
    </span>
  </button>
  <LibraryCardDelete {entry} {onDeleted} />
</div>
