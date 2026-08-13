import { LogicalSize, PhysicalPosition } from "@tauri-apps/api/dpi";
import { getCurrentWindow } from "@tauri-apps/api/window";

export const COLLAPSED_SIZE = { width: 760, height: 84 };
export const EXPANDED_SIZE = { width: 760, height: 148 };

let resizeQueue: Promise<void> = Promise.resolve();

/** Resize the floating bar window while keeping its bottom edge fixed:
 *  the bar floats near the bottom of the work area, so extra height must
 *  grow upward, not off-screen. Calls are serialized: a rapid second
 *  toggle must not read the window position mid-move. */
export function resizeBarWindow(native: boolean, expanded: boolean) {
  resizeQueue = resizeQueue.then(() => applyBarSize(native, expanded));
  return resizeQueue;
}

async function applyBarSize(native: boolean, expanded: boolean) {
  if (!native) return;
  const next = expanded ? EXPANDED_SIZE : COLLAPSED_SIZE;
  const prev = expanded ? COLLAPSED_SIZE : EXPANDED_SIZE;
  try {
    const window = getCurrentWindow();
    const factor = await window.scaleFactor();
    const position = await window.outerPosition();
    const shift = Math.round((next.height - prev.height) * factor);
    await window.setSize(new LogicalSize(next.width, next.height));
    await window.setPosition(new PhysicalPosition(position.x, position.y - shift));
  } catch {
    // Window resize is cosmetic; the bar still renders both states.
  }
}

/** Status sub-line while capture is healthy: `last {N} min · {source}`. */
export function sourceDetail(
  source: { label: string } | null,
  minutes: number,
): string | null {
  return source ? `last ${minutes} min · ${source.label}` : null;
}
