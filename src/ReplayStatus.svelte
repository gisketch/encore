<script lang="ts">
  import { sourceDetail } from "./barSupport";

  type CaptureState =
    | "stopped"
    | "starting"
    | "capturing"
    | "paused"
    | "recovering"
    | "source_unavailable"
    | "failed";
  type CaptureSnapshot = {
    permission: "unknown" | "permission_required" | "permission_denied" | "restart_required" | "granted";
    capture: CaptureState;
    source: { label: string } | null;
    droppedFrames: number;
    evidenceGapCount: number;
    completedSegments: number;
    pipeline: "idle" | "encoding" | "failed";
    encoderErrorCode: string | null;
    retention: { minutes: 5 | 10; errorCode: string | null };
    sourceNotice: string | null;
  };
  type ReplaySnapshot = {
    state: "unavailable" | "preparing" | "saved" | "failed";
    pending: { segmentCount: number; totalBytes: number } | null;
    saved: { id: string; displayName: string } | null;
    errorCode: string | null;
    shortcut: { errorCode: string | null };
  };
  type Status = { label: string; tone: string; detail: string };

  const captureLabels: Record<CaptureState, { label: string; tone: string }> = {
    stopped: { label: "Idle", tone: "attention" },
    starting: { label: "Starting", tone: "progress" },
    capturing: { label: "Recording", tone: "recording" },
    paused: { label: "Paused", tone: "attention" },
    recovering: { label: "Recovering", tone: "progress" },
    source_unavailable: { label: "Source lost", tone: "attention" },
    failed: { label: "Capture failed", tone: "attention" },
  };
  const permissionDetail: Record<string, string> = { restart_required: "Restart required" };
  const sourceNoticeDetail: Record<string, string> = {
    persisted_window_unavailable: "Saved window unavailable — using full screen",
  };

  let { snapshot, replay, actionError }: {
    snapshot: CaptureSnapshot;
    replay: ReplaySnapshot;
    actionError: string | null;
  } = $props();

  const status = $derived.by(currentStatus);

  function currentStatus(): Status {
    if (replay.state === "preparing") {
      return {
        label: "Saving replay",
        tone: "progress",
        detail: replay.pending ? replayDetail(replay.pending) : "Pending replay",
      };
    }
    if (replay.state === "saved") {
      if (replay.errorCode) {
        return { label: "Finder unavailable", tone: "attention", detail: replay.errorCode };
      }
      return {
        label: "Replay saved",
        tone: "healthy",
        detail: replay.saved?.displayName ?? "Ready in Finder",
      };
    }
    if (replay.state === "failed") {
      return {
        label: replay.pending ? "Replay needs retry" : "Replay failed",
        tone: "attention",
        detail: replay.errorCode ?? "replay_trigger_failed",
      };
    }
    if (replay.shortcut.errorCode) {
      return { label: "Shortcut unavailable", tone: "attention", detail: replay.shortcut.errorCode };
    }
    if (snapshot.permission !== "granted") {
      return { label: "Screen access", tone: "attention", detail: defaultDetail() };
    }
    if (snapshot.pipeline === "failed") {
      return { label: "Recording failed", tone: "attention", detail: defaultDetail() };
    }
    if (snapshot.capture === "capturing" && snapshot.pipeline === "idle") {
      return { label: "Starting encoder", tone: "progress", detail: defaultDetail() };
    }
    if (snapshot.capture === "paused") {
      return {
        label: "Paused",
        tone: "attention",
        detail: `Not recording — keeping last ${snapshot.retention.minutes} min`,
      };
    }
    return { ...captureLabels[snapshot.capture], detail: defaultDetail() };
  }

  function defaultDetail() {
    return (
      actionError ??
      snapshot.retention.errorCode ??
      snapshot.encoderErrorCode ??
      (snapshot.sourceNotice ? sourceNoticeDetail[snapshot.sourceNotice] : null) ??
      sourceDetail(snapshot.source, snapshot.retention.minutes) ??
      permissionDetail[snapshot.permission] ??
      "Choose Enable once"
    );
  }

  function replayDetail(pending: NonNullable<ReplaySnapshot["pending"]>) {
    const suffix = pending.segmentCount === 1 ? "" : "s";
    return `${pending.segmentCount} segment${suffix} · ${formatBytes(pending.totalBytes)}`;
  }

  function formatBytes(bytes: number) {
    if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
</script>

<div class="status" title={`${snapshot.droppedFrames} dropped · ${snapshot.evidenceGapCount} gaps · ${snapshot.completedSegments} segments`}>
  <span class="status__pulse status__pulse--{status.tone}" aria-hidden="true"></span>
  <span class="status__copy">
    <strong>{status.label}</strong>
    <small>{status.detail}</small>
  </span>
</div>
