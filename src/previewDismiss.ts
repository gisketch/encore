/** The post-save preview's auto-dismiss timing model, kept pure so the
 *  rule — "count down to ~8s, but freeze while the pointer rests on the
 *  card" — can be read and reasoned about without a window, a timer, or a
 *  webview. `PreviewCountdown.svelte` is the only caller: it owns the
 *  interval and the dismissal, this file owns the arithmetic.
 *
 *  There is no JavaScript test runner in this repository (see
 *  `package.json`), and the harness forbids adding one for this ticket, so
 *  this module is deliberately branch-free and total instead: two
 *  functions, no state, no time source of its own.
 *
 *  Worked through by hand, with TICK_MS = 200:
 *    advanced(0, false)    -> 200     (counting down)
 *    advanced(200, true)   -> 200     (hovered: frozen, never grows)
 *    advanced(200, false)  -> 400     (pointer left: resumes where it was)
 *    shouldDismiss(7800)   -> false
 *    shouldDismiss(8000)   -> true
 *  Hover therefore only ever postpones a dismissal; it can neither cancel
 *  one that already fired nor rewind time already counted. */

/** How long the preview stays up on its own, per the spec's "about eight
 *  seconds". */
export const DISMISS_AFTER_MS = 8000;

/** Countdown resolution. Coarse on purpose: this drives a wall-clock
 *  dismissal, not an animation, so five wakeups a second is plenty and
 *  costs nothing next to the video decode it sits beside. */
export const TICK_MS = 200;

/** Hovered-or-not to "what one tick does to the elapsed total". A lookup
 *  rather than a branch, which is also the honest shape of the rule: the
 *  hovered case is not an early return, it is simply a tick worth zero. */
const ADVANCE: Record<string, (elapsedMs: number, tickMs: number) => number> = {
  true: (elapsedMs) => elapsedMs,
  false: (elapsedMs, tickMs) => elapsedMs + tickMs,
};

/** The elapsed countdown time after one tick. Frozen while `hovered`, so
 *  the pointer resting anywhere on the card — including on its action
 *  buttons — cannot be overtaken by a dismissal mid-click. */
export function advanced(elapsedMs: number, hovered: boolean, tickMs: number = TICK_MS): number {
  return ADVANCE[String(hovered)](elapsedMs, tickMs);
}

/** Whether the preview has now been up long enough to dismiss itself. */
export function shouldDismiss(elapsedMs: number): boolean {
  return elapsedMs >= DISMISS_AFTER_MS;
}
