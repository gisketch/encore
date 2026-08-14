<script lang="ts">
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import type { SettingsSnapshot } from "./appearance";

  // Self-contained rather than a prop of the Saving section: the toggle owns
  // one independently-persisted setting, and keeping its state here leaves
  // the section itself a plain layout file.
  //
  // Optimistic first paint matches the backend default (on); the real
  // snapshot replaces it on mount.
  let saveSound = $state(true);
  let busy = $state(false);

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
    const snapshot = await invoke<SettingsSnapshot>("settings_snapshot");
    saveSound = snapshot.save_sound;
  }

  async function toggleSaveSound() {
    busy = true;
    try {
      const snapshot = await invoke<SettingsSnapshot>("update_save_sound", {
        enabled: !saveSound,
      });
      saveSound = snapshot.save_sound;
    } finally {
      busy = false;
    }
  }
</script>

<div class="settings-row">
  <span class="settings-row__text">
    <span class="settings-row__label">Play a sound when a replay is saved</span>
    <span class="settings-row__sublabel">A short chime confirms the replay was written</span>
  </span>
  <button
    type="button"
    class="switch"
    class:active={saveSound}
    role="switch"
    aria-checked={saveSound}
    aria-label="Play a sound when a replay is saved"
    disabled={busy}
    onclick={toggleSaveSound}
  >
    <span class="switch__knob"></span>
  </button>
</div>
