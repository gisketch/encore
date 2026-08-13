<script lang="ts">
  import ReplayShortcutHint from "./ReplayShortcutHint.svelte";

  type ReplaySnapshot = {
    state: "unavailable" | "preparing" | "saved" | "failed";
    pending: { id: string } | null;
    saved: { id: string; displayName: string } | null;
  };

  let {
    native,
    busy,
    replayBusy,
    eligible,
    replay,
    shortcutRegistered,
    onTrigger,
    onRetryReplay,
    onRevealReplay,
    compact = false,
  }: {
    native: boolean;
    busy: boolean;
    replayBusy: boolean;
    eligible: boolean;
    replay: ReplaySnapshot;
    shortcutRegistered: boolean;
    onTrigger: () => void;
    onRetryReplay: () => void;
    onRevealReplay: (replayId: string) => void;
    compact?: boolean;
  } = $props();

  const disabled = $derived(busy || replayBusy || !native || !eligible);
  const title = $derived(eligible ? "Save the current replay" : "Replay needs retained evidence");
</script>

{#if replay.state === "preparing"}
  <button class="primary-action primary-action--save" class:primary-action--compact={compact} type="button" disabled title="Saving replay">
    <span class="clapper" aria-hidden="true"></span>
    Saving
  </button>
{:else if replay.state === "saved" && replay.saved}
  <div class="replay-saved-actions" class:replay-saved-actions--compact={compact}>
    <button class="primary-action primary-action--open" type="button" onclick={() => onRevealReplay(replay.saved!.id)} disabled={busy || replayBusy || !native} title="Reveal this replay in Finder" aria-label="Open in Finder">
      <span class="saved-action__finder-mark" aria-hidden="true">↗</span>
      <span class="saved-action__label">Open</span>
    </button>
    <button class="primary-action primary-action--save" type="button" onclick={onTrigger} {disabled} title="Save a new replay" aria-label="Save a new replay">
      <span class="clapper" aria-hidden="true"></span>
      <span class="saved-action__label">Save Replay</span>
    </button>
  </div>
{:else if replay.state === "failed" && replay.pending}
  <button class="primary-action primary-action--save" class:primary-action--compact={compact} type="button" onclick={onRetryReplay} disabled={busy || replayBusy || !native} title="Retry saving this replay">
    Retry
  </button>
{:else}
  <button class="primary-action primary-action--save" class:primary-action--compact={compact} type="button" onclick={onTrigger} {disabled} {title}>
    <span class="clapper" aria-hidden="true"></span>
    Save Replay
    {#if shortcutRegistered}<ReplayShortcutHint />{/if}
  </button>
{/if}
