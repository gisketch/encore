<script lang="ts">
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { animate } from "motion";
  import { onMount } from "svelte";
  import BarAdvancedRow from "./BarAdvancedRow.svelte";
  import { resizeBarWindow } from "./barSupport";
  import RailActions from "./RailActions.svelte";
  import ReplayStatus from "./ReplayStatus.svelte";

  type PermissionState =
    | "unknown"
    | "permission_required"
    | "permission_denied"
    | "restart_required"
    | "granted";
  type CaptureState =
    | "stopped"
    | "starting"
    | "capturing"
    | "paused"
    | "recovering"
    | "source_unavailable"
    | "failed";
  type PipelineState = "idle" | "encoding" | "failed";
  type CaptureSource = {
    id: string;
    kind: "display" | "window";
    label: string;
    width: number;
    height: number;
    isMain: boolean;
    bundleId: string | null;
    title: string | null;
  };
  type CaptureSnapshot = {
    permission: PermissionState;
    capture: CaptureState;
    source: CaptureSource | null;
    droppedFrames: number;
    retryCount: number;
    errorCode: string | null;
    evidenceGapCount: number;
    pipeline: PipelineState;
    completedSegments: number;
    encoderErrorCode: string | null;
    retention: {
      state: "ready" | "failed";
      minutes: 5 | 10;
      retainedSegments: number;
      retainedBytes: number;
      errorCode: string | null;
    };
    sourceNotice: string | null;
  };
  type ReplaySnapshot = {
    state: "unavailable" | "preparing" | "saved" | "failed";
    pending: {
      id: string;
      createdAtUnixMs: number;
      segmentCount: number;
      totalBytes: number;
      evidenceStartUnixMs: number;
      evidenceEndUnixMs: number;
    } | null;
    saved: {
      id: string;
      displayName: string;
    } | null;
    errorCode: string | null;
    shortcut: {
      state: "unavailable" | "registered";
      errorCode: string | null;
    };
  };

  const previewSnapshot: CaptureSnapshot = {
    permission: "permission_required",
    capture: "stopped",
    source: null,
    droppedFrames: 0,
    retryCount: 0,
    errorCode: null,
    evidenceGapCount: 0,
    pipeline: "idle",
    completedSegments: 0,
    encoderErrorCode: null,
    retention: {
      state: "ready",
      minutes: 10,
      retainedSegments: 0,
      retainedBytes: 0,
      errorCode: null,
    },
    sourceNotice: null,
  };
  const previewReplay: ReplaySnapshot = {
    state: "unavailable",
    pending: null,
    saved: null,
    errorCode: null,
    shortcut: { state: "unavailable", errorCode: null },
  };

  let rail: HTMLElement;
  let snapshot = $state<CaptureSnapshot>(previewSnapshot);
  let replay = $state<ReplaySnapshot>(previewReplay);
  let sources = $state<CaptureSource[]>([]);
  let busy = $state(false);
  let replayBusy = $state(false);
  let native = $state(false);
  let expanded = $state(false);
  let actionError = $state<string | null>(null);
  const replayEligible = $derived(
    snapshot.retention.state === "ready" && snapshot.retention.retainedSegments > 0,
  );

  onMount(() => {
    native = isTauri();
    let unlistenCapture: (() => void) | undefined;
    let unlistenReplay: (() => void) | undefined;
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const entrance = animate(
      rail,
      { opacity: [0, 1], transform: ["translateY(14px) scale(0.985)", "translateY(0) scale(1)"] },
      { duration: reduced ? 0 : 0.34, ease: "easeOut" },
    );

    void (async () => {
      if (!native) return;
      unlistenCapture = await listen<CaptureSnapshot>("capture-state-changed", (event) => {
        snapshot = event.payload;
        actionError = snapshot.errorCode;
        if (snapshot.permission === "granted" && sources.length === 0) void refreshSources();
      });
      unlistenReplay = await listen<ReplaySnapshot>("replay-state-changed", (event) => {
        replay = event.payload;
      });
      [snapshot, replay] = await Promise.all([
        invoke<CaptureSnapshot>("capture_snapshot"),
        invoke<ReplaySnapshot>("replay_snapshot"),
      ]);
      actionError = snapshot.errorCode;
      if (snapshot.permission === "granted") await refreshSources();
    })().catch(showError);

    return () => {
      entrance.stop();
      unlistenCapture?.();
      unlistenReplay?.();
    };
  });

  async function refreshSources() {
    sources = await invoke<CaptureSource[]>("list_capture_sources");
  }

  async function run(action: () => Promise<CaptureSnapshot>) {
    busy = true;
    actionError = null;
    try {
      snapshot = await action();
    } catch (error) {
      showError(error);
    } finally {
      busy = false;
    }
  }

  function enableCapture() {
    return run(() => invoke<CaptureSnapshot>("request_capture_permission"));
  }

  function retryCapture() {
    return run(() => invoke<CaptureSnapshot>("retry_capture"));
  }

  function fallbackToFullScreen() {
    return run(() => invoke<CaptureSnapshot>("start_capture"));
  }

  function pauseCapture() {
    return run(() => invoke<CaptureSnapshot>("pause_capture"));
  }

  function resumeCapture() {
    return run(() => invoke<CaptureSnapshot>("resume_capture"));
  }

  function switchSource(sourceId: string) {
    if (!sourceId || sourceId === snapshot.source?.id) return;
    return run(async () => {
      const next = await invoke<CaptureSnapshot>("switch_capture_source", { sourceId });
      await refreshSources();
      return next;
    });
  }

  async function triggerReplay() {
    replayBusy = true;
    actionError = null;
    try {
      replay = await invoke<ReplaySnapshot>("trigger_replay");
    } catch (error) {
      replay = await invoke<ReplaySnapshot>("replay_snapshot");
      actionError = typeof error === "string" ? error : "replay_trigger_failed";
    } finally {
      replayBusy = false;
    }
  }

  async function retryReplay() {
    replayBusy = true;
    actionError = null;
    try {
      replay = await invoke<ReplaySnapshot>("retry_replay");
    } catch (error) {
      replay = await invoke<ReplaySnapshot>("replay_snapshot");
      actionError = typeof error === "string" ? error : "replay_trigger_failed";
    } finally {
      replayBusy = false;
    }
  }

  async function revealSavedReplay(replayId: string) {
    replayBusy = true;
    actionError = null;
    try {
      replay = await invoke<ReplaySnapshot>("reveal_saved_replay", { replayId });
    } catch (error) {
      replay = await invoke<ReplaySnapshot>("replay_snapshot");
      actionError = typeof error === "string" ? error : "export_reveal_failed";
    } finally {
      replayBusy = false;
    }
  }

  function showError(error: unknown) {
    actionError = typeof error === "string" ? error : "capture_command_failed";
  }

  function toggleExpanded() {
    expanded = !expanded;
    void resizeBarWindow(native, expanded);
  }

  function openLibrary() {
    void invoke("open_export_folder").catch(showError);
  }

  function openSettings() {
    void invoke("open_settings_window").catch(showError);
  }
</script>

<svelte:head><title>Encore</title></svelte:head>

<main class="floating-shell">
  <section class="action-bar" bind:this={rail} aria-label="Encore replay controls">
    <div class="bar-row">
      <div class="drag-zone" data-tauri-drag-region aria-label="Drag Encore">
        <span class="grip" aria-hidden="true">
          {#each Array(6) as _}<i></i>{/each}
        </span>
        <span class="buffer-mark" aria-hidden="true">
          <i></i><i></i><i></i>
        </span>
      </div>

      <ReplayStatus {snapshot} {replay} {actionError} />

      <span class="spacer" aria-hidden="true"></span>

      <button class="icon-button" type="button" title="Open library" aria-label="Open library" onclick={openLibrary} disabled={!native}>
        <span class="glyph-grid" aria-hidden="true"><i></i><i></i><i></i><i></i></span>
      </button>

      <RailActions
        permission={snapshot.permission}
        capture={snapshot.capture}
        {native}
        {busy}
        {replayBusy}
        eligible={replayEligible}
        {replay}
        shortcutRegistered={replay.shortcut.state === "registered"}
        onEnable={enableCapture}
        onOpenSettings={() => invoke("open_screen_recording_settings")}
        onQuit={() => invoke("quit_encore")}
        onRetry={retryCapture}
        onFallbackToFullScreen={fallbackToFullScreen}
        onTrigger={triggerReplay}
        onRetryReplay={retryReplay}
        onRevealReplay={revealSavedReplay}
      />

      <button
        class="icon-button"
        type="button"
        title={expanded ? "Hide advanced controls" : "Show advanced controls"}
        aria-label={expanded ? "Hide advanced controls" : "Show advanced controls"}
        aria-expanded={expanded}
        onclick={toggleExpanded}
      >
        <i class="glyph-chevron" class:glyph-chevron--up={expanded} aria-hidden="true"></i>
      </button>
    </div>

    {#if expanded}
      <BarAdvancedRow
        {sources}
        sourceId={snapshot.source?.id ?? ""}
        retainedBytes={snapshot.retention.retainedBytes}
        selectDisabled={snapshot.permission !== "granted" || sources.length === 0 || busy}
        capture={snapshot.capture}
        {busy}
        {native}
        onSwitchSource={switchSource}
        onPause={pauseCapture}
        onResume={resumeCapture}
        onOpenSettings={openSettings}
        onQuit={() => invoke("quit_encore")}
      />
    {/if}
  </section>
</main>
