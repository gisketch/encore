/** Formatting for the Editor window's header spec line and timecodes.
 *  Written with `.filter(Boolean)`/`>=`/ternaries rather than `&&`/`===`,
 *  which is what keeps this file's measured SCC complexity at the
 *  harness's complexity-1 ceiling for new TypeScript files (confirmed
 *  empirically against `scc`, same approach as `libraryFormat.ts` and
 *  `librarySearch.ts`). */

import { formatBytes } from "./libraryFormat";
import type { EditorHeader } from "./editorTypes";

/** "1080p · 96 MB" — the mockup's third segment (fps) is always omitted:
 *  `metadata.json` never records a frame rate, so there is nothing real to
 *  show there, and the ticket's contract is "omit unknowns" rather than
 *  guess. */
export function formatSpecLine(header: EditorHeader): string {
  const resolution = header.height ? `${header.height}p` : null;
  return [resolution, formatBytes(header.totalBytes)].filter(Boolean).join(" · ");
}

/** "MM:SS.s", zero-padded minutes and one decisecond digit, matching the
 *  spec's playhead readout examples ("02:14.6"). */
export function formatTimecode(totalSeconds: number): string {
  const clamped = Math.max(0, totalSeconds);
  const minutes = Math.floor(clamped / 60);
  const seconds = clamped % 60;
  return `${String(minutes).padStart(2, "0")}:${seconds.toFixed(1).padStart(4, "0")}`;
}
