<script lang="ts">
  import { clampToTrim, nearestKeyframe } from "./editorTimeline";
  import { formatTimecode } from "./editorFormat";

  const RULER_MARK_COUNT = 6;

  let {
    durationSeconds,
    keyframeSeconds,
    currentTime,
    inSeconds,
    outSeconds,
    cuts,
    onSeek,
    onTrimChange,
  }: {
    durationSeconds: number;
    keyframeSeconds: number[];
    currentTime: number;
    inSeconds: number;
    outSeconds: number;
    cuts: { start: number; end: number }[];
    onSeek: (time: number) => void;
    onTrimChange: (next: { in: number; out: number }) => void;
  } = $props();

  let trackEl = $state<HTMLDivElement | undefined>();
  let dragging = $state<"in" | "out" | null>(null);

  let rulerMarks = $derived(
    Array.from(
      { length: RULER_MARK_COUNT },
      (_, index) => (durationSeconds * index) / (RULER_MARK_COUNT - 1),
    ),
  );

  function ratio(seconds: number): number {
    return Math.min(Math.max(seconds / Math.max(durationSeconds, 0.001), 0), 1);
  }

  function percent(seconds: number): string {
    return `${ratio(seconds) * 100}%`;
  }

  function timeAtClientX(clientX: number): number {
    const rect = trackEl?.getBoundingClientRect();
    const width = rect?.width || 1;
    const left = rect?.left ?? 0;
    return Math.min(Math.max((clientX - left) / width, 0), 1) * durationSeconds;
  }

  function startDrag(handle: "in" | "out") {
    return (event: PointerEvent) => {
      event.stopPropagation();
      dragging = handle;
    };
  }

  function handlePointerMove(event: PointerEvent) {
    if (!dragging) return;
    const snapped = nearestKeyframe(keyframeSeconds, timeAtClientX(event.clientX));
    const next =
      dragging === "in"
        ? { in: Math.min(snapped, outSeconds), out: outSeconds }
        : { in: inSeconds, out: Math.max(snapped, inSeconds) };
    onTrimChange(next);
  }

  function handlePointerUp() {
    dragging = null;
  }

  function seekFromTrack(event: MouseEvent) {
    onSeek(clampToTrim(timeAtClientX(event.clientX), inSeconds, outSeconds));
  }

  const KEY_STEP_SECONDS = 1;

  function handleTrackKeydown(event: KeyboardEvent) {
    const step = event.key === "ArrowRight" ? KEY_STEP_SECONDS : event.key === "ArrowLeft" ? -KEY_STEP_SECONDS : 0;
    if (step) onSeek(clampToTrim(currentTime + step, inSeconds, outSeconds));
  }
</script>

<svelte:window onpointermove={handlePointerMove} onpointerup={handlePointerUp} />

<div class="editor-timeline">
  <div class="editor-timeline__ruler">
    {#each rulerMarks as mark (mark)}
      <span>{formatTimecode(mark)}</span>
    {/each}
  </div>
  <div
    class="editor-timeline__track"
    bind:this={trackEl}
    onclick={seekFromTrack}
    onkeydown={handleTrackKeydown}
    role="slider"
    aria-label="Playhead"
    aria-valuemin={0}
    aria-valuemax={durationSeconds}
    aria-valuenow={currentTime}
    tabindex="0"
  >
    <span class="editor-timeline__dim editor-timeline__dim--head" style="width:{percent(inSeconds)}"
    ></span>
    <span
      class="editor-timeline__dim editor-timeline__dim--tail"
      style="width:{percent(durationSeconds - outSeconds)}"
    ></span>
    {#each cuts as cut (cut.start)}
      <span
        class="editor-timeline__cut"
        style="left:{percent(cut.start)};width:{percent(cut.end - cut.start)}"
        aria-hidden="true"
      >
        <span class="editor-timeline__cut-chip">CUT {formatTimecode(cut.end - cut.start)}</span>
      </span>
    {/each}
    <span class="editor-timeline__playhead" style="left:{percent(currentTime)}" aria-hidden="true"
    ></span>
    <button
      type="button"
      class="editor-timeline__handle editor-timeline__handle--in"
      style="left:{percent(inSeconds)}"
      onpointerdown={startDrag("in")}
      aria-label="Trim start"
    ></button>
    <button
      type="button"
      class="editor-timeline__handle editor-timeline__handle--out"
      style="left:{percent(outSeconds)}"
      onpointerdown={startDrag("out")}
      aria-label="Trim end"
    ></button>
  </div>
</div>
