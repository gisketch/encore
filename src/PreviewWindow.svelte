<script lang="ts">
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import { formatCardSubline } from "./libraryFormat";
  import { fetchThumbnailDataUrl } from "./libraryThumbnail";
  import PreviewActions from "./PreviewActions.svelte";
  import type { PreviewPayload } from "./previewTypes";

  let payload = $state<PreviewPayload | null>(null);
  let thumbnailUrl = $state<string | null>(null);

  // The window is shown without focus and reused for every save, so it
  // never remounts: the bootstrap read covers the first save that raised
  // it, and `preview-changed` swaps the contents for every save after.
  onMount(() => {
    if (!isTauri()) return;
    void invoke<PreviewPayload | null>("preview_context")
      .then(show)
      .catch(() => undefined);
    let unlisten: (() => void) | undefined;
    void listen<PreviewPayload>("preview-changed", (event) => show(event.payload)).then((stop) => {
      unlisten = stop;
    });
    return () => unlisten?.();
  });

  // The thumbnail is fetched per replay and left null on any failure, so
  // the striped placeholder is the permanent fallback — the same contract
  // the Library card relies on.
  function show(next: PreviewPayload | null) {
    payload = next;
    thumbnailUrl = null;
    if (!next) return;
    void fetchThumbnailDataUrl(next.id).then((url) => {
      thumbnailUrl = url;
    });
  }

  // Dismiss hides rather than closes: the same window is raised again by
  // the next save, with its contents swapped. Nothing else is torn down.
  function dismiss() {
    void getCurrentWindow().hide();
  }

  const KEY_ACTIONS: Record<string, () => void> = { Escape: dismiss };

  function handleKey(event: KeyboardEvent) {
    KEY_ACTIONS[event.key]?.();
  }
</script>

<svelte:head><title>Preview</title></svelte:head>
<svelte:window onkeydown={handleKey} />

<main class="preview-shell">
  <section class="preview-card-surface">
    <button
      type="button"
      class="preview-dismiss"
      title="Dismiss"
      aria-label="Dismiss preview"
      onclick={dismiss}
    ></button>

    <div class="preview-still">
      {#if thumbnailUrl}
        <img class="preview-still__image" src={thumbnailUrl} alt="" />
      {:else}
        <span class="preview-still__label">Screen frame</span>
      {/if}
    </div>

    <div class="preview-meta">
      <p class="preview-name">{payload?.displayName ?? "Saved replay"}</p>
      <p class="preview-subline">
        {formatCardSubline(payload?.durationSeconds ?? null, payload?.totalBytes ?? 0)}
      </p>
    </div>

    <PreviewActions {payload} onDismiss={dismiss} />
  </section>
</main>
