<script lang="ts">
  import { convertFileSrc, invoke, isTauri } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import EditorHeaderBar from "./EditorHeaderBar.svelte";
  import EditorTimeline from "./EditorTimeline.svelte";
  import { formatSpecLine, formatTimecode } from "./editorFormat";
  import { playbackClampState } from "./editorTimeline";
  import type { EditorHeader, EditorKeyframes } from "./editorTypes";

  let header = $state<EditorHeader | null>(null);
  let keyframes = $state<EditorKeyframes | null>(null);
  let errorCode = $state<string | null>(null);
  let videoEl = $state<HTMLVideoElement | undefined>();
  let isPlaying = $state(false);
  let currentTime = $state(0);
  let inSeconds = $state(0);
  let outSeconds = $state(0);

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
        errorCode = "editor_replay_missing";
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
      isPlaying = false;
      currentTime = 0;
      inSeconds = 0;
      outSeconds = keyframeResult.durationSeconds;
    } catch (error) {
      errorCode = typeof error === "string" ? error : "editor_load_failed";
    }
  }

  function togglePlay() {
    if (!videoEl) return;
    if (isPlaying) videoEl.pause();
    else void videoEl.play();
  }

  function handleTimeUpdate() {
    if (!videoEl) return;
    const state = playbackClampState(videoEl.currentTime, inSeconds, outSeconds);
    videoEl.currentTime = state.time;
    if (state.pause) videoEl.pause();
    currentTime = state.time;
  }

  function seekTo(time: number) {
    if (!videoEl) return;
    videoEl.currentTime = playbackClampState(time, inSeconds, outSeconds).time;
  }

  function updateTrim(next: { in: number; out: number }) {
    inSeconds = next.in;
    outSeconds = next.out;
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
    <EditorHeaderBar
      title={header?.title ?? "Replay"}
      specLine={header ? formatSpecLine(header) : ""}
      onBack={backToLibrary}
      onClose={closeWindow}
    />

    <div class="editor-body">
      {#if errorCode}
        <p class="editor-error" role="alert">{errorCode}</p>
      {/if}

      {#if header}
        <div class="editor-preview">
          <!-- svelte-ignore a11y_media_has_caption -->
          <video
            class="editor-video"
            src={convertFileSrc(header.videoPath)}
            bind:this={videoEl}
            onplay={() => (isPlaying = true)}
            onpause={() => (isPlaying = false)}
            ontimeupdate={handleTimeUpdate}
          ></video>
          <button
            type="button"
            class="editor-play"
            onclick={togglePlay}
            aria-label={isPlaying ? "Pause" : "Play"}
          >
            <span class="editor-play__icon" class:editor-play__icon--pause={isPlaying}></span>
          </button>
        </div>

        <p class="editor-readout">
          {formatTimecode(currentTime)} / {formatTimecode(outSeconds - inSeconds)} kept
        </p>
      {/if}

      {#if keyframes}
        <EditorTimeline
          durationSeconds={keyframes.durationSeconds}
          keyframeSeconds={keyframes.seconds}
          {currentTime}
          {inSeconds}
          {outSeconds}
          onSeek={seekTo}
          onTrimChange={updateTrim}
        />
      {/if}
    </div>
  </section>
</main>
