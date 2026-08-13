<script lang="ts">
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import type { Appearance, SettingsSnapshot } from "./appearance";
  import SettingsAppearanceControl from "./SettingsAppearanceControl.svelte";

  let { appearance, onSetAppearance }: {
    appearance: Appearance;
    onSetAppearance: (value: Appearance) => void;
  } = $props();

  let menuBarMode = $state(false);
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
    menuBarMode = snapshot.menu_bar_mode;
  }

  async function toggleMenuBarMode() {
    busy = true;
    try {
      const snapshot = await invoke<SettingsSnapshot>("update_menu_bar_mode", {
        enabled: !menuBarMode,
      });
      menuBarMode = snapshot.menu_bar_mode;
    } finally {
      busy = false;
    }
  }
</script>

<section class="settings-section" aria-labelledby="general-heading">
  <h2 id="general-heading" class="settings-section__title">General</h2>

  <div class="settings-row">
    <span class="settings-row__text">
      <span class="settings-row__label">Show in menu bar</span>
      <span class="settings-row__sublabel">Hide the floating bar, keep a menu bar icon</span>
    </span>
    <button
      type="button"
      class="switch"
      class:active={menuBarMode}
      role="switch"
      aria-checked={menuBarMode}
      aria-label="Show in menu bar"
      disabled={busy}
      onclick={toggleMenuBarMode}
    >
      <span class="switch__knob"></span>
    </button>
  </div>

  <div class="settings-row">
    <span class="settings-row__label">Appearance</span>
    <SettingsAppearanceControl value={appearance} onSelect={onSetAppearance} />
  </div>
</section>
