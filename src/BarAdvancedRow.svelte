<script lang="ts">
  type CaptureSource = { id: string; label: string };

  let {
    sources,
    sourceId,
    retentionMinutes,
    retainedBytes,
    selectDisabled,
    busy,
    native,
    onSwitchSource,
    onSetRetention,
    onQuit,
  }: {
    sources: CaptureSource[];
    sourceId: string;
    retentionMinutes: 5 | 10;
    retainedBytes: number;
    selectDisabled: boolean;
    busy: boolean;
    native: boolean;
    onSwitchSource: (sourceId: string) => void;
    onSetRetention: (minutes: 5 | 10) => void;
    onQuit: () => void;
  } = $props();

  function formatBufferBytes(bytes: number) {
    if (bytes < 1024 * 1024) return `${Math.max(0, Math.round(bytes / 1024))} KB`;
    return `${Math.round(bytes / (1024 * 1024))} MB`;
  }
</script>

<div class="bar-divider" aria-hidden="true"></div>
<div class="bar-row bar-row--advanced">
  <label class="pill-control source-control">
    <span class="source-icon" aria-hidden="true"></span>
    <span class="sr-only">Capture source</span>
    <select
      value={sourceId}
      onchange={(event) => onSwitchSource(event.currentTarget.value)}
      disabled={selectDisabled}
    >
      {#if sources.length === 0}<option value="">Screen</option>{/if}
      {#each sources as source}<option value={source.id}>{source.label}</option>{/each}
    </select>
  </label>

  <div class="retention" aria-label="Replay duration">
    <button class:active={retentionMinutes === 5} type="button" aria-pressed={retentionMinutes === 5} onclick={() => onSetRetention(5)} disabled={busy}>5m</button>
    <button class:active={retentionMinutes === 10} type="button" aria-pressed={retentionMinutes === 10} onclick={() => onSetRetention(10)} disabled={busy}>10m</button>
  </div>

  <span class="spacer" aria-hidden="true"></span>

  <span class="buffer-badge" title="Nothing leaves this Mac">
    <i aria-hidden="true"></i>buffer {formatBufferBytes(retainedBytes)} · local
  </span>

  <button class="quit-button" type="button" onclick={onQuit} disabled={!native}>Quit</button>
</div>
