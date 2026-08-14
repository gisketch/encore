<script lang="ts">
  /** The preview's auto-dismiss clock. Renders nothing: it exists only so
   *  the interval, the restart rule, and the dismissal branch live outside
   *  `PreviewWindow.svelte` (already tracked by the SCC gate at its PP-02
   *  complexity), the same split `PreviewActions` made for the action row.
   *
   *  The arithmetic is in `previewDismiss.ts`, deliberately branch-free and
   *  hand-worked there; this file is just the wiring. */
  import { advanced, shouldDismiss, TICK_MS } from "./previewDismiss";
  import type { PreviewPayload } from "./previewTypes";

  let {
    payload,
    active,
    hovered,
    onElapsed,
  }: {
    payload: PreviewPayload | null;
    active: boolean;
    hovered: boolean;
    onElapsed: () => void;
  } = $props();

  let elapsedMs = $state(0);

  /** The window is hidden and reused rather than closed, so this never
   *  remounts. Restart the countdown whenever the replay on screen swaps
   *  (a second save) or the card is shown again — otherwise a re-shown
   *  preview would inherit a nearly expired clock and vanish instantly. */
  $effect(() => {
    payload;
    active;
    elapsedMs = 0;
  });

  /** No ticking while dismissed: a hidden preview must not keep waking up
   *  to count toward a dismissal that already happened. */
  $effect(() => {
    if (!active) return;
    const timer = setInterval(tick, TICK_MS);
    return () => clearInterval(timer);
  });

  function tick() {
    elapsedMs = advanced(elapsedMs, hovered);
    if (shouldDismiss(elapsedMs)) onElapsed();
  }
</script>
