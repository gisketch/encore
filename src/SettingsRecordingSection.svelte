<script lang="ts">
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import type { SettingsSnapshot } from "./appearance";
  import { selectedSourceId, type PersistedTarget, type RecordingSource } from "./recordingSettings";

  type RetentionResult = { retention: { minutes: 5 | 10 } };

  let sources = $state<RecordingSource[]>([]);
  let retentionMinutes = $state<5 | 10>(10);
  let defaultTarget = $state<PersistedTarget>({ kind: "display" });
  let busy = $state(false);

  const selectedId = $derived(selectedSourceId(defaultTarget, sources));

  onMount(() => {
    if (!isTauri()) return;
    void refresh();
    // The window hides instead of closing, so state fetched at mount goes
    // stale; refetch whenever the window regains focus.
    let unlisten: (() => void) | undefined;
    void getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        if (focused) void refresh();
      })
      .then((stop) => {
        unlisten = stop;
      });
    return () => unlisten?.();
  });

  async function refresh() {
    const [snapshot, list] = await Promise.all([
      invoke<SettingsSnapshot>("settings_snapshot"),
      invoke<RecordingSource[]>("list_capture_sources").catch(() => []),
    ]);
    retentionMinutes = snapshot.retention_minutes;
    defaultTarget = snapshot.default_target;
    sources = list;
  }

  async function setRetention(minutes: 5 | 10) {
    busy = true;
    try {
      const result = await invoke<RetentionResult>("set_retention_minutes", { minutes });
      retentionMinutes = result.retention.minutes;
    } finally {
      busy = false;
    }
  }

  async function setDefaultSource(sourceId: string) {
    busy = true;
    try {
      const snapshot = await invoke<SettingsSnapshot>("update_default_source", { sourceId });
      defaultTarget = snapshot.default_target;
    } finally {
      busy = false;
    }
  }
</script>

<section class="settings-section" aria-labelledby="recording-heading">
  <h2 id="recording-heading" class="settings-section__title">Recording</h2>

  <div class="settings-row">
    <span class="settings-row__text">
      <span class="settings-row__label">Replay window</span>
      <span class="settings-row__sublabel">How much of the past is kept in the rolling buffer</span>
    </span>
    <label class="settings-select">
      <select
        value={retentionMinutes}
        disabled={busy}
        onchange={(event) => setRetention(Number(event.currentTarget.value) === 5 ? 5 : 10)}
      >
        <option value={5}>5 minutes</option>
        <option value={10}>10 minutes</option>
      </select>
    </label>
  </div>

  <div class="settings-row">
    <span class="settings-row__text">
      <span class="settings-row__label">Default source</span>
      <span class="settings-row__sublabel">Used when Encore launches</span>
    </span>
    <label class="settings-select">
      <select
        value={selectedId}
        disabled={busy || sources.length === 0}
        onchange={(event) => setDefaultSource(event.currentTarget.value)}
      >
        {#if sources.length === 0}<option value="">Main display</option>{/if}
        {#each sources as source}<option value={source.id}>{source.label}</option>{/each}
      </select>
    </label>
  </div>
</section>
