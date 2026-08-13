<script lang="ts">
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import type { HotkeyId, Hotkeys, SettingsSnapshot } from "./appearance";
  import { formatAccelerator } from "./hotkeyDisplay";

  const ROWS: { id: HotkeyId; label: string }[] = [
    { id: "save_replay", label: "Save replay" },
    { id: "pause_capture", label: "Pause capture" },
    { id: "open_library", label: "Open library" },
  ];

  const MODIFIER_KEYS = new Set(["Meta", "Control", "Alt", "Shift"]);
  const DEFAULT_HOTKEYS: Hotkeys = {
    save_replay: "Cmd+Alt+R",
    pause_capture: "Cmd+Alt+P",
    open_library: "Cmd+Alt+L",
  };

  let hotkeys = $state<Hotkeys>(DEFAULT_HOTKEYS);
  let capturingId = $state<HotkeyId | null>(null);
  let errorId = $state<HotkeyId | null>(null);
  let errorMessage = $state("");
  let busy = $state(false);

  onMount(() => {
    if (!isTauri()) return;
    void refresh();
  });

  async function refresh() {
    const snapshot = await invoke<SettingsSnapshot>("settings_snapshot");
    hotkeys = snapshot.hotkeys;
  }

  function startCapture(id: HotkeyId) {
    if (busy) return;
    capturingId = id;
    errorId = null;
  }

  function cancelCapture() {
    capturingId = null;
  }

  function chordFromEvent(event: KeyboardEvent): string | null {
    if (MODIFIER_KEYS.has(event.key)) return null;
    const tokens: string[] = [];
    if (event.metaKey) tokens.push("Cmd");
    if (event.ctrlKey) tokens.push("Ctrl");
    if (event.altKey) tokens.push("Alt");
    if (event.shiftKey) tokens.push("Shift");
    if (tokens.length === 0) return null;
    tokens.push(event.key.length === 1 ? event.key.toUpperCase() : event.key);
    return tokens.join("+");
  }

  async function handleKeydown(event: KeyboardEvent) {
    const id = capturingId;
    if (!id) return;
    event.preventDefault();
    if (event.key === "Escape") {
      cancelCapture();
      return;
    }
    const chord = chordFromEvent(event);
    if (!chord) return;
    busy = true;
    try {
      const snapshot = await invoke<SettingsSnapshot>("update_hotkey", { id, accelerator: chord });
      hotkeys = snapshot.hotkeys;
      capturingId = null;
    } catch (error) {
      errorId = id;
      errorMessage = typeof error === "string" ? error : "hotkey_registration_failed";
      capturingId = null;
      setTimeout(() => {
        if (errorId === id) errorId = null;
      }, 2200);
    } finally {
      busy = false;
    }
  }
</script>

<svelte:window onkeydown={capturingId ? handleKeydown : undefined} />

<section class="settings-section" aria-labelledby="hotkeys-heading">
  <h2 id="hotkeys-heading" class="settings-section__title">Hotkeys</h2>

  {#each ROWS as row (row.id)}
    <div class="settings-row">
      <span class="settings-row__label">{row.label}</span>
      <span class="hotkey-actions">
        {#if capturingId === row.id}
          <span class="hotkey-chip hotkey-chip--capturing">Press keys…</span>
        {:else if errorId === row.id}
          <span class="hotkey-chip hotkey-chip--error" title={errorMessage}>{errorMessage}</span>
        {:else}
          <kbd class="hotkey-chip">{formatAccelerator(hotkeys[row.id])}</kbd>
        {/if}
        <button
          type="button"
          class="settings-pill-button"
          disabled={busy}
          onclick={() => startCapture(row.id)}
        >Edit</button>
      </span>
    </div>
  {/each}
</section>
