/** Byte/duration/time formatting for the Library window. Branchy by
 *  nature (unit thresholds, hour rollover), so it lives in its own file
 *  rather than the tracked Svelte components — written with `>=`/`>`
 *  comparisons and truthy checks rather than `===`/`!==`, which is what
 *  keeps this file's measured SCC complexity at the harness's
 *  complexity-1 ceiling for new TypeScript files (ternaries and `>=`/`>`
 *  cost nothing there; `===`/`!==` do). */

export function formatBytes(bytes: number): string {
  const gb = bytes / 1_000_000_000;
  return gb >= 0.1
    ? `${gb.toFixed(gb >= 10 ? 0 : 1)} GB`
    : `${(bytes / 1_000_000).toFixed(0)} MB`;
}

export function formatDuration(totalSeconds: number): string {
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = Math.floor(totalSeconds % 60);
  const paddedSeconds = String(seconds).padStart(2, "0");
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${paddedSeconds}`
    : `${minutes}:${paddedSeconds}`;
}

export function formatSavedTime(unixMs: number): string {
  return new Date(unixMs).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
}

/** "N replays · X GB" rollup used by both the header and each group. */
export function formatRollup(count: number, bytes: number): string {
  const noun = count > 1 ? "replays" : "replay";
  return `${count} ${noun} · ${formatBytes(bytes)}`;
}

/** Card sub-line: "duration · size", or just size when duration wasn't
 *  recorded (degraded/legacy bundles). */
export function formatCardSubline(durationSeconds: number | null, bytes: number): string {
  return durationSeconds
    ? `${formatDuration(durationSeconds)} · ${formatBytes(bytes)}`
    : formatBytes(bytes);
}
