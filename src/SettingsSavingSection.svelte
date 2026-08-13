<script lang="ts">
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { homeDir } from "@tauri-apps/api/path";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import type { AfterSave, SettingsSnapshot } from "./appearance";
  import { abbreviateHome } from "./savingSettings";

  const AFTER_SAVE_OPTIONS: { value: AfterSave; label: string }[] = [
    { value: "reveal", label: "Show in Finder" },
    { value: "open_editor", label: "Open editor" },
    { value: "nothing", label: "Nothing" },
  ];

  let saveDestination = $state("");
  let home = $state("");
  let afterSave = $state<AfterSave>("nothing");
  let busy = $state(false);

  const displayDestination = $derived(
    home ? abbreviateHome(saveDestination, home) : saveDestination,
  );

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
    const [snapshot, homePath] = await Promise.all([
      invoke<SettingsSnapshot>("settings_snapshot"),
      homeDir().catch(() => ""),
    ]);
    saveDestination = snapshot.save_destination;
    afterSave = snapshot.after_save;
    home = homePath;
  }

  async function changeDestination() {
    const picked = await open({ directory: true, defaultPath: saveDestination });
    if (typeof picked !== "string") return;
    busy = true;
    try {
      const snapshot = await invoke<SettingsSnapshot>("update_save_destination", { path: picked });
      saveDestination = snapshot.save_destination;
    } finally {
      busy = false;
    }
  }

  async function setAfterSave(value: AfterSave) {
    busy = true;
    try {
      const snapshot = await invoke<SettingsSnapshot>("update_after_save", { afterSave: value });
      afterSave = snapshot.after_save;
    } finally {
      busy = false;
    }
  }
</script>

<section class="settings-section" aria-labelledby="saving-heading">
  <h2 id="saving-heading" class="settings-section__title">Saving</h2>

  <div class="settings-row">
    <span class="settings-row__text">
      <span class="settings-row__label">Save replays to</span>
      <span class="settings-row__sublabel settings-row__path">{displayDestination}</span>
    </span>
    <button type="button" class="settings-pill-button" disabled={busy} onclick={changeDestination}>
      Change…
    </button>
  </div>

  <div class="settings-row">
    <span class="settings-row__text">
      <span class="settings-row__label">After saving</span>
      <span class="settings-row__sublabel">What happens once a replay is packaged</span>
    </span>
    <div class="appearance-control" role="radiogroup" aria-label="After saving">
      {#each AFTER_SAVE_OPTIONS as option}
        <button
          type="button"
          class:active={afterSave === option.value}
          role="radio"
          aria-checked={afterSave === option.value}
          disabled={busy}
          onclick={() => setAfterSave(option.value)}
        >{option.label}</button>
      {/each}
    </div>
  </div>
</section>
