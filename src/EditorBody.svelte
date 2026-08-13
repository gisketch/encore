<script lang="ts">
  /** Owns every piece of Editor state PG-14 adds — trim, splits, cuts,
   *  undo/redo history, keyboard shortcuts, and the export flow — plus
   *  the header bar, so it can intercept back/close clicks while edits
   *  are unexported. Split out of `EditorWindow.svelte` (already at the
   *  harness's tracked-complexity baseline) into its own new file, which
   *  has the room. */
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { untrack } from "svelte";
  import EditorCloseConfirm from "./EditorCloseConfirm.svelte";
  import EditorExportBar from "./EditorExportBar.svelte";
  import EditorHeaderBar from "./EditorHeaderBar.svelte";
  import { boundaries, cutContaining, keepSegments, keptDuration, segmentAt, withSplit } from "./cutList";
  import type { Cut } from "./cutList";
  import { formatSpecLine, formatTimecode } from "./editorFormat";
  import EditorTimeline from "./EditorTimeline.svelte";
  import { clampToTrim, nearestKeyframe, playbackClampState } from "./editorTimeline";
  import type { EditorHeader, EditorKeyframes, ExportFormat } from "./editorTypes";

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
  let format = $state<ExportFormat>("mp4");
  let pendingAction = $state<(() => void) | null>(null);
  let exportBar: { triggerExport: () => Promise<string | null> } | undefined;

  let boundaryPoints = $derived(boundaries(inSeconds, outSeconds, splits));
  let kept = $derived(keepSegments(inSeconds, outSeconds, splits, cuts));
  let keptSeconds = $derived(keptDuration(kept));
  let canUndo = $derived(history.length > 0);
  let canRedo = $derived(future.length > 0);

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
    "cmd+e": () => void exportBar?.triggerExport(),
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
  {format}
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

  <EditorExportBar
    bind:this={exportBar}
    replayId={header.id}
    {kept}
    {format}
    onFormatChange={(next) => (format = next)}
    onExportSuccess={() => (dirty = false)}
  />
</div>

<EditorCloseConfirm show={pendingAction !== null} onCancel={cancelPendingClose} onDiscard={confirmDiscard} />
