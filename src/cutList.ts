/** Pure cut-list model for the Editor: split points, cut (removed)
 *  ranges, and the derived kept-segment list `export_trimmed_replay`
 *  consumes. Written with array-method chains, ternaries, and `!`/`<`/
 *  `>=` comparisons rather than `if`/`===`/`&&`/`||`/`??`, matching
 *  `editorFormat.ts`/`editorTimeline.ts`'s approach to holding the
 *  harness's TypeScript complexity-1 ceiling for new files — the cut
 *  list's actual mutation/undo-redo/keyboard branching lives in
 *  `EditorBody.svelte` instead, where the Svelte ceiling has the room. */

export type Cut = { start: number; end: number };

const EPSILON_SECONDS = 1e-6;

function closeTo(a: number, b: number): boolean {
  return Math.abs(a - b) < EPSILON_SECONDS;
}

/** Ascending boundary points across `[inSeconds, outSeconds]`: the trim
 *  points plus every split. */
export function boundaries(inSeconds: number, outSeconds: number, splits: number[]): number[] {
  return [inSeconds, ...splits, outSeconds].sort((a, b) => a - b);
}

/** Every boundary-delimited sub-segment, paired start/end. */
function subSegments(boundaryPoints: number[]): Cut[] {
  return boundaryPoints.slice(0, -1).map((start, index) => ({ start, end: boundaryPoints[index + 1] }));
}

/** The sub-segment containing `time`, skipping any range already cut —
 *  what "Remove segment" removes and what playback should not land in. */
export function segmentAt(boundaryPoints: number[], cuts: Cut[], time: number): Cut | undefined {
  return subSegments(boundaryPoints)
    .filter((segment) => time >= segment.start)
    .filter((segment) => time < segment.end)
    .filter((segment) => !cuts.some((cut) => closeTo(cut.start, segment.start)))
    .at(-1);
}

/** The KEPT ranges — every boundary sub-segment minus the cut ones —
 *  exactly the complement `export_trimmed_replay` expects. */
export function keepSegments(inSeconds: number, outSeconds: number, splits: number[], cuts: Cut[]): Cut[] {
  return subSegments(boundaries(inSeconds, outSeconds, splits)).filter(
    (segment) => !cuts.some((cut) => closeTo(cut.start, segment.start)),
  );
}

export function keptDuration(segments: Cut[]): number {
  return segments.reduce((total, segment) => total + (segment.end - segment.start), 0);
}

/** The cut range containing `time`, if any — playback jumps past it
 *  rather than rendering the removed footage. */
export function cutContaining(cuts: Cut[], time: number): Cut | undefined {
  return cuts.filter((cut) => time >= cut.start).find((cut) => time < cut.end);
}

/** `splits` plus `point`, unless a split (or a trim point) already sits
 *  within `EPSILON_SECONDS` of it. */
export function withSplit(splits: number[], boundaryPoints: number[], point: number): number[] {
  return boundaryPoints.some((existing) => closeTo(existing, point))
    ? splits
    : [...splits, point].sort((a, b) => a - b);
}
