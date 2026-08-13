<script lang="ts">
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import EditorBody from "./EditorBody.svelte";
  import type { EditorHeader, EditorKeyframes } from "./editorTypes";

  let header = $state<EditorHeader | null>(null);
  let keyframes = $state<EditorKeyframes | null>(null);
  let errorCode = $state<string | null>(null);

  let loadedId = $state<string | null>(null);

  onMount(() => {
    if (!isTauri()) return;
    void load();
    // The window hides instead of closing (global close-as-hide handler)
    // and is reused across replays, so it needs its own focus-triggered
    // reload — same convention as the Library window — to pick up a new
    // `editor_context` id when the Library opens a different card.
    let unlisten: (() => void) | undefined;
    void getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        if (focused) void load();
      })
      .then((stop) => {
        unlisten = stop;
      });
    return () => unlisten?.();
  });

  async function load() {
    try {
      const id = await invoke<string | null>("editor_context");
      if (!id) {
        // The hidden window boots before any replay is opened; that is an
        // empty state, not an error.
        errorCode = null;
        return;
      }
      if (id === loadedId) return;
      const [headerResult, keyframeResult] = await Promise.all([
        invoke<EditorHeader>("editor_header", { id }),
        invoke<EditorKeyframes>("editor_keyframes", { id }),
      ]);
      loadedId = id;
      errorCode = null;
      header = headerResult;
      keyframes = keyframeResult;
    } catch (error) {
      errorCode = typeof error === "string" ? error : "editor_load_failed";
      header = null;
      keyframes = null;
    }
  }

  function backToLibrary() {
    void getCurrentWindow().hide();
    void invoke("open_library_window");
  }

  function closeWindow() {
    void getCurrentWindow().close();
  }
</script>

<svelte:head><title>Editor</title></svelte:head>

<main class="editor-shell">
  <section class="editor-card-surface">
    {#if header && keyframes}
      {#key header.id}
        <EditorBody {header} {keyframes} onBack={backToLibrary} onClose={closeWindow} />
      {/key}
    {:else}
      <div class="editor-body">
        <p class={errorCode ? "editor-error" : "editor-empty"} role="status">
          {errorCode ?? "Open a replay from the Library to edit it."}
        </p>
      </div>
    {/if}
  </section>
</main>
