<script lang="ts">
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import { applyAppearance, type Appearance, type SettingsSnapshot } from "./appearance";
  import CaptureShell from "./CaptureShell.svelte";
  import LibraryWindow from "./LibraryWindow.svelte";
  import SettingsWindow from "./SettingsWindow.svelte";

  const native = isTauri();
  const label = native ? getCurrentWindow().label : "main";
  let appearance = $state<Appearance>("system");

  onMount(() => {
    if (!native) return;
    let unlisten: (() => void) | undefined;
    void (async () => {
      const snapshot = await invoke<SettingsSnapshot>("settings_snapshot");
      appearance = snapshot.appearance;
      applyAppearance(appearance);
      unlisten = await listen<SettingsSnapshot>("settings-changed", (event) => {
        appearance = event.payload.appearance;
        applyAppearance(appearance);
      });
    })();
    return () => unlisten?.();
  });

  function setAppearance(next: Appearance) {
    void invoke("update_appearance", { appearance: next });
  }
</script>

{#if label === "settings"}
  <SettingsWindow {appearance} onSetAppearance={setAppearance} />
{:else}
  {#if label === "library"}
    <LibraryWindow />
  {:else}
    <CaptureShell />
  {/if}
{/if}
