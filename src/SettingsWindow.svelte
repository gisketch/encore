<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import type { Appearance } from "./appearance";
  import SettingsGeneralSection from "./SettingsGeneralSection.svelte";
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

      <SettingsGeneralSection {appearance} onSetAppearance={onSetAppearance} />
    </div>
  </section>
</main>
