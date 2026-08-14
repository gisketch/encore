<script lang="ts">
  /** The preview card's action row: Edit, Share, Open Folder for the
   *  replay currently on screen. Split out of `PreviewWindow.svelte`
   *  (which is already tracked by the SCC gate at its PP-02 complexity)
   *  so the notice state and the three handlers live in a file of their
   *  own, following the `SettingsSaveSoundRow` / `EditorExportBar`
   *  precedent.
   *
   *  Every action targets `payload.id` — the bundle folder name, which is
   *  exactly the id `open_editor_window` and the library commands expect —
   *  and `payload.videoPath`, which is inside the save destination and so
   *  clears `copy_export_to_clipboard`'s own guard. */
  import { invoke } from "@tauri-apps/api/core";
  import type { PreviewPayload } from "./previewTypes";

  let { payload, onDismiss }: { payload: PreviewPayload | null; onDismiss: () => void } = $props();

  /** How long the inline confirmation and the inline failure label stay
   *  up. Failures linger longer because they are the message the tester
   *  actually needs to read. */
  const CONFIRM_MS = 2000;
  const FAILURE_MS = 4000;

  let notice = $state<string | null>(null);
  let failed = $state(false);
  let noticeTimer: ReturnType<typeof setTimeout> | undefined;

  /** The window is hidden and reused rather than closed, so this row never
   *  remounts: a swapped-in replay (or the next save re-showing the window)
   *  must not inherit the previous one's "Copied" or failure label. */
  $effect(() => {
    payload;
    clearTimeout(noticeTimer);
    notice = null;
  });

  function say(text: string, isFailure: boolean) {
    clearTimeout(noticeTimer);
    notice = text;
    failed = isFailure;
    noticeTimer = setTimeout(() => (notice = null), isFailure ? FAILURE_MS : CONFIRM_MS);
  }

  /** Dismiss-on-action applies only when the action actually worked: a
   *  failure that hid the preview would take its own error label with it. */
  async function leaveFor(command: string, failure: string) {
    try {
      await invoke(command, { id: payload?.id });
      clearTimeout(noticeTimer);
      notice = null;
      onDismiss();
    } catch {
      say(failure, true);
    }
  }

  /** Share stays open by design — the tester pastes the reference while
   *  the preview is still showing what they copied. */
  async function share() {
    try {
      await invoke("copy_export_to_clipboard", { path: payload?.videoPath });
      say("Copied", false);
    } catch {
      say("Copy failed", true);
    }
  }
</script>

<div class="preview-actions">
  {#if notice}
    <span class="preview-notice" class:preview-notice--failed={failed} role="status">{notice}</span>
  {/if}
  <button
    type="button"
    class="preview-action preview-action--primary"
    disabled={!payload}
    onclick={() => leaveFor("open_editor_window", "Editor failed")}>Edit</button
  >
  <button
    type="button"
    class="preview-action"
    title="Copies the replay file to the clipboard"
    disabled={!payload}
    onclick={share}>Share</button
  >
  <button
    type="button"
    class="preview-action"
    disabled={!payload}
    onclick={() => leaveFor("reveal_replay_bundle", "Reveal failed")}>Open Folder</button
  >
</div>
