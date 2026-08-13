/** Pure timeline math for the Editor: keyframe snapping and playback
 *  clamping. Kept branch-light (reduce/ternary/comparisons rather than
 *  if/&&/===) to hold the harness's TypeScript complexity-1 ceiling for
 *  new files — the timeline's actual drag-handle and pointer-event
 *  branching lives in `EditorTimeline.svelte` instead, per the ticket's
 *  own guidance. */

/** The keyframe closest to `target`, for trim-handle drag snapping. Falls
 *  back to `target` itself when there are no keyframes yet (e.g. still
 *  loading). */
export function nearestKeyframe(keyframeSeconds: number[], target: number): number {
  return keyframeSeconds.reduce(
    (best, candidate) =>
      Math.abs(candidate - target) < Math.abs(best - target) ? candidate : best,
    keyframeSeconds[0] ?? target,
  );
}

/** Clamps `time` into `[inSeconds, outSeconds]` — seeking before the trim
 *  start snaps forward to it, seeking past the trim end snaps back to it. */
export function clampToTrim(time: number, inSeconds: number, outSeconds: number): number {
  return Math.min(Math.max(time, inSeconds), outSeconds);
}

/** Playback's per-`timeupdate` decision: where the video's currentTime
 *  should land, and whether it just crossed the trim end and should pause. */
export function playbackClampState(
  time: number,
  inSeconds: number,
  outSeconds: number,
): { time: number; pause: boolean } {
  const clamped = clampToTrim(time, inSeconds, outSeconds);
  return { time: clamped, pause: clamped >= outSeconds };
}
