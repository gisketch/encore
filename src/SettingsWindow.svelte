<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import type { Appearance } from "./appearance";
  import SettingsAppearanceControl from "./SettingsAppearanceControl.svelte";
  import SettingsHotkeysSection from "./SettingsHotkeysSection.svelte";
  import SettingsRecordingSection from "./SettingsRecordingSection.svelte";
  import SettingsSavingSection from "./SettingsSavingSection.svelte";

  let { appearance, onSetAppearance }: {
    appearance: Appearance;
    onSetAppearance: (value: Appearance) => void;
  } = $props();

  function closeWindow() {
    void getCurrentWindow().close();
  }
</script>

<svelte:head><title>Settings</title></svelte:head>

<main class="settings-shell">
  <section class="settings-card">
    <header class="settings-titlebar" data-tauri-drag-region>
      <span class="traffic-lights">
        <button
          type="button"
          class="traffic-dot traffic-dot--close"
          title="Close"
          aria-label="Close settings"
          onclick={closeWindow}
        ></button>
        <i class="traffic-dot traffic-dot--minimize" aria-hidden="true"></i>
        <i class="traffic-dot traffic-dot--zoom" aria-hidden="true"></i>
      </span>
      <h1>Settings</h1>
      <span class="traffic-lights-spacer" aria-hidden="true"></span>
    </header>

    <div class="settings-body">
      <SettingsRecordingSection />

      <SettingsSavingSection />

      <SettingsHotkeysSection />

      <section class="settings-section" aria-labelledby="general-heading">
        <h2 id="general-heading" class="settings-section__title">General</h2>
        <div class="settings-row">
          <span class="settings-row__label">Appearance</span>
          <SettingsAppearanceControl value={appearance} onSelect={onSetAppearance} />
        </div>
      </section>
    </div>
  </section>
</main>
