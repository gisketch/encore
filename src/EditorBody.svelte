<script lang="ts">
  /** Owns every piece of Editor state PG-14 adds — trim, splits, cuts,
   *  undo/redo history, keyboard shortcuts, and the export flow — plus
   *  the header bar, so it can intercept back/close clicks while edits
   *  are unexported. Split out of `EditorWindow.svelte` (already at the
   *  harness's tracked-complexity baseline) into its own new file, which
   *  has the room. */
  import { convertFileSrc, invoke } from "@tauri-apps/api/core";
  import { onMount, untrack } from "svelte";
  import type { SettingsSnapshot } from "./appearance";
  import EditorCloseConfirm from "./EditorCloseConfirm.svelte";
  import { exportTrimmedReplay } from "./editorExport";
  import EditorHeaderBar from "./EditorHeaderBar.svelte";
  import { boundaries, cutContaining, keepSegments, keptDuration, segmentAt, withSplit } from "./cutList";
  import type { Cut } from "./cutList";
  import { formatSpecLine, formatTimecode } from "./editorFormat";
  import EditorTimeline from "./EditorTimeline.svelte";
  import { clampToTrim, nearestKeyframe, playbackClampState } from "./editorTimeline";
  import type { EditorHeader, EditorKeyframes } from "./editorTypes";

  let {
    header,
    keyframes,
    onBack,
    onClose,
  }: {
    header: EditorHeader;
    keyframes: EditorKeyframes;
    onBack: () => void;
    onClose: () => void;
  } = $props();

  type Snapshot = { inSeconds: number; outSeconds: number; splits: number[]; cuts: Cut[] };

  let videoEl = $state<HTMLVideoElement | undefined>();
  let isPlaying = $state(false);
  let currentTime = $state(0);
  let inSeconds = $state(0);
  // `{#key header.id}` in `EditorWindow.svelte` remounts this component
  // fresh per replay, so reading `keyframes.durationSeconds` only once
  // here is intentional; `untrack` says so explicitly instead of leaving
  // svelte-check's "captures the initial value" warning unaddressed.
  let outSeconds = $state(untrack(() => keyframes.durationSeconds));
  let splits = $state<number[]>([]);
  let cuts = $state<Cut[]>([]);
  let history = $state<Snapshot[]>([]);
  let future = $state<Snapshot[]>([]);
  let dirty = $state(false);
  let destinationName = $state("");
  let pendingAction = $state<(() => void) | null>(null);
  let exportState = $state<"idle" | "busy" | "success" | "error">("idle");
  let exportErrorCode = $state<string | null>(null);

  let boundaryPoints = $derived(boundaries(inSeconds, outSeconds, splits));
  let kept = $derived(keepSegments(inSeconds, outSeconds, splits, cuts));
  let keptSeconds = $derived(keptDuration(kept));
  let canUndo = $derived(history.length > 0);
  let canRedo = $derived(future.length > 0);

  onMount(() => {
    void invoke<SettingsSnapshot>("settings_snapshot").then((snapshot) => {
      const segments = snapshot.save_destination.split("/");
      const last = segments.at(-1);
      destinationName = last ? last : snapshot.save_destination;
    });
  });

  function snapshot(): Snapshot {
    return { inSeconds, outSeconds, splits: [...splits], cuts: [...cuts] };
  }

  function applySnapshot(next: Snapshot) {
    inSeconds = next.inSeconds;
    outSeconds = next.outSeconds;
    splits = next.splits;
    cuts = next.cuts;
  }

  function pushHistory() {
    history = [...history, snapshot()];
    future = [];
    dirty = true;
  }

  function togglePlay() {
    if (!videoEl) return;
    isPlaying ? videoEl.pause() : void videoEl.play();
  }

  function handleTimeUpdate() {
    if (!videoEl) return;
    const trimmed = playbackClampState(videoEl.currentTime, inSeconds, outSeconds);
    const cut = cutContaining(cuts, trimmed.time);
    const landing = cut ? cut.end : trimmed.time;
    videoEl.currentTime = landing;
    if (trimmed.pause) videoEl.pause();
    currentTime = landing;
  }

  function seekTo(time: number) {
    if (!videoEl) return;
    const clamped = clampToTrim(time, inSeconds, outSeconds);
    const cut = cutContaining(cuts, clamped);
    videoEl.currentTime = cut ? cut.end : clamped;
  }

  function updateTrim(next: { in: number; out: number }) {
    pushHistory();
    inSeconds = next.in;
    outSeconds = next.out;
  }

  function splitAtPlayhead() {
    const snapped = nearestKeyframe(keyframes.seconds, currentTime);
    const clamped = clampToTrim(snapped, inSeconds, outSeconds);
    const nextSplits = withSplit(splits, boundaryPoints, clamped);
    if (nextSplits.length <= splits.length) return;
    pushHistory();
    splits = nextSplits;
  }

  function removeSegment() {
    const segment = segmentAt(boundaryPoints, cuts, currentTime);
    if (!segment) return;
    pushHistory();
    cuts = [...cuts, segment].sort((a, b) => a.start - b.start);
  }

  function undo() {
    const previous = history.at(-1);
    if (!previous) return;
    future = [...future, snapshot()];
    history = history.slice(0, -1);
    applySnapshot(previous);
    dirty = true;
  }

  function redo() {
    const next = future.at(-1);
    if (!next) return;
    history = [...history, snapshot()];
    future = future.slice(0, -1);
    applySnapshot(next);
    dirty = true;
  }

  async function startExport() {
    exportState = "busy";
    exportErrorCode = null;
    const result = await exportTrimmedReplay(header.id, kept);
    exportState = result.ok ? "success" : "error";
    exportErrorCode = result.ok ? null : result.error;
    dirty = result.ok ? false : dirty;
  }

  function showInLibrary() {
    void invoke("open_library_window");
  }

  function isTypingTarget(event: KeyboardEvent): boolean {
    const target = event.target as HTMLElement | null;
    const tag = target ? target.tagName.toLowerCase() : "";
    return ["input", "textarea"].includes(tag);
  }

  function keyId(event: KeyboardEvent): string {
    const modifier = event.metaKey ? (event.shiftKey ? "cmd+shift+" : "cmd+") : "";
    return modifier + event.key.toLowerCase();
  }

  const keyActions: Record<string, () => void> = {
    s: splitAtPlayhead,
    "cmd+e": startExport,
    "cmd+z": undo,
    "cmd+shift+z": redo,
  };

  function handleKeydown(event: KeyboardEvent) {
    const action = isTypingTarget(event) ? undefined : keyActions[keyId(event)];
    if (!action) return;
    event.preventDefault();
    action();
  }

  function requestBack() {
    dirty ? (pendingAction = onBack) : onBack();
  }

  function requestClose() {
    dirty ? (pendingAction = onClose) : onClose();
  }

  function cancelPendingClose() {
    pendingAction = null;
  }

  function confirmDiscard() {
    pendingAction?.();
    pendingAction = null;
    dirty = false;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<EditorHeaderBar
  title={header.title}
  specLine={formatSpecLine(header)}
  onBack={requestBack}
  onClose={requestClose}
/>

<div class="editor-body">
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
    <button type="button" class="editor-play" onclick={togglePlay} aria-label={isPlaying ? "Pause" : "Play"}>
      <span class="editor-play__icon" class:editor-play__icon--pause={isPlaying}></span>
    </button>
  </div>

  <p class="editor-readout">
    {formatTimecode(currentTime)} / {formatTimecode(keptSeconds)} kept
  </p>

  <EditorTimeline
    durationSeconds={keyframes.durationSeconds}
    keyframeSeconds={keyframes.seconds}
    {currentTime}
    {inSeconds}
    {outSeconds}
    {cuts}
    onSeek={seekTo}
    onTrimChange={updateTrim}
  />

  <div class="editor-toolbar">
    <button type="button" class="editor-toolbar__button" onclick={splitAtPlayhead}>Split at playhead (S)</button>
    <button type="button" class="editor-toolbar__button" onclick={removeSegment}>Remove segment</button>
    <span class="spacer" aria-hidden="true"></span>
    <button type="button" class="editor-toolbar__button" onclick={undo} disabled={!canUndo}>Undo</button>
    <button type="button" class="editor-toolbar__button" onclick={redo} disabled={!canRedo}>Redo</button>
  </div>

  <div class="editor-export-bar">
    <div class="editor-export-bar__formats">
      <span class="editor-export-bar__format editor-export-bar__format--active">MP4</span>
      <span class="editor-export-bar__format" title="Coming soon">GIF</span>
    </div>
    <span class="editor-export-bar__destination">{destinationName}</span>
    {#if exportState === "success"}
      <span class="editor-export-bar__success">Exported</span>
      <button type="button" class="editor-export-bar__link" onclick={showInLibrary}>Show in Library</button>
    {:else}
      <button
        type="button"
        class="editor-export-bar__export"
        onclick={startExport}
        disabled={exportState === "busy"}
      >
        {exportState === "busy" ? "Exporting…" : "Export ⌘E"}
      </button>
    {/if}
    {#if exportErrorCode}
      <span class="editor-export-bar__error" role="alert">{exportErrorCode}</span>
    {/if}
  </div>
</div>

<EditorCloseConfirm show={pendingAction !== null} onCancel={cancelPendingClose} onDiscard={confirmDiscard} />
