<script lang="ts">
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import type { SettingsSnapshot } from "./appearance";
  import { formatAccelerator } from "./hotkeyDisplay";

  let label = $state("⌘⌥R");

  onMount(() => {
    if (!isTauri()) return;
    let unlisten: (() => void) | undefined;
    void (async () => {
      const snapshot = await invoke<SettingsSnapshot>("settings_snapshot");
      label = formatAccelerator(snapshot.hotkeys.save_replay);
      unlisten = await listen<SettingsSnapshot>("settings-changed", (event) => {
        label = formatAccelerator(event.payload.hotkeys.save_replay);
      });
    })();
    return () => unlisten?.();
  });
</script>

<kbd>{label}</kbd>
