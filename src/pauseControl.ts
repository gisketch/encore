export type CaptureState =
  | "stopped"
  | "starting"
  | "capturing"
  | "paused"
  | "recovering"
  | "source_unavailable"
  | "failed";

const PAUSE_LABEL: Record<CaptureState, string> = {
  stopped: "Pause",
  starting: "Pause",
  capturing: "Pause",
  paused: "Resume",
  recovering: "Pause",
  source_unavailable: "Pause",
  failed: "Pause",
};

/** "Pause" while capturing/starting/recovering (and other non-paused states);
 *  "Resume" once the state machine reports paused. */
export function pauseControlLabel(capture: CaptureState): string {
  return PAUSE_LABEL[capture];
}

/** States with nothing to pause or resume; the control disables there. */
const PAUSE_INERT: Record<CaptureState, boolean> = {
  stopped: true,
  starting: false,
  capturing: false,
  paused: false,
  recovering: false,
  source_unavailable: true,
  failed: true,
};

/** The pause control is disabled while a command is in flight, the shell
 *  isn't running under Tauri (no native command surface to call), or the
 *  capture state has nothing to pause or resume. */
export function pauseControlDisabled(
  capture: CaptureState,
  busy: boolean,
  native: boolean,
): boolean {
  return [busy, !native, PAUSE_INERT[capture]].some(Boolean);
}

/** Route the pause control click to resume when paused, pause otherwise. */
export function resolvePauseAction(
  capture: CaptureState,
  onPause: () => void,
  onResume: () => void,
): () => void {
  const actions: Record<CaptureState, () => void> = {
    stopped: onPause,
    starting: onPause,
    capturing: onPause,
    paused: onResume,
    recovering: onPause,
    source_unavailable: onPause,
    failed: onPause,
  };
  return actions[capture];
}
