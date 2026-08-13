<script lang="ts">
  import ReplayAction from "./ReplayAction.svelte";

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
  type ReplaySnapshot = {
    state: "unavailable" | "preparing" | "saved" | "failed";
    pending: { id: string } | null;
    saved: { id: string; displayName: string } | null;
  };

  let {
    permission,
    capture,
    native,
    busy,
    replayBusy,
    eligible,
    replay,
    shortcutRegistered,
    onEnable,
    onOpenSettings,
    onQuit,
    onRetry,
    onFallbackToFullScreen,
    onTrigger,
    onRetryReplay,
    onRevealReplay,
  }: {
    permission: PermissionState;
    capture: CaptureState;
    native: boolean;
    busy: boolean;
    replayBusy: boolean;
    eligible: boolean;
    replay: ReplaySnapshot;
    shortcutRegistered: boolean;
    onEnable: () => void;
    onOpenSettings: () => void;
    onQuit: () => void;
    onRetry: () => void;
    onFallbackToFullScreen: () => void;
    onTrigger: () => void;
    onRetryReplay: () => void;
    onRevealReplay: (replayId: string) => void;
  } = $props();

  const captureActionRequired = $derived(
    (permission !== "granted" && permission !== "unknown") ||
      ["failed", "source_unavailable"].includes(capture),
  );
</script>

<div class="rail-actions" class:rail-actions--paired={captureActionRequired}>
  {#if permission === "permission_required"}
    <button class="primary-action primary-action--compact primary-action--capture" type="button" onclick={onEnable} disabled={busy || !native} title="Enable capture">Enable</button>
  {:else if permission === "permission_denied"}
    <button class="primary-action primary-action--compact primary-action--capture" type="button" onclick={onOpenSettings} disabled={!native} title="Open screen-recording settings">Settings</button>
  {:else if permission === "restart_required"}
    <button class="primary-action primary-action--compact primary-action--capture" type="button" onclick={onQuit} disabled={!native} title="Quit to apply screen-recording access">Restart</button>
  {:else if capture === "failed"}
    <button class="primary-action primary-action--compact primary-action--capture" type="button" onclick={onRetry} disabled={busy} title="Retry capture">Retry</button>
  {:else if capture === "source_unavailable"}
    <div class="source-unavailable-actions">
      <button class="primary-action primary-action--compact primary-action--capture" type="button" onclick={onRetry} disabled={busy} title="Retry the same source">Retry</button>
      <button class="primary-action primary-action--compact primary-action--capture" type="button" onclick={onFallbackToFullScreen} disabled={busy} title="Fall back to full-screen capture">Full screen</button>
    </div>
  {/if}

  <ReplayAction
    {native}
    {busy}
    {replayBusy}
    {eligible}
    {replay}
    {shortcutRegistered}
    {onTrigger}
    {onRetryReplay}
    {onRevealReplay}
    compact={captureActionRequired}
  />
</div>
