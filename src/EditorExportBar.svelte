
<script lang="ts">
  /** Owns the export bar's own state — busy/success/error for both the
   *  MP4/GIF export and the "Copy to clipboard" pill — split out of
   *  `EditorBody.svelte` (PG-15) so that already-tracked file's SCC
   *  complexity stays at its committed baseline while this ticket adds
   *  GIF format selection and clipboard copy. `format` is lifted to the
   *  parent (the header bar's lossless/GIF badge needs it too), so this
   *  component receives it as a controlled prop rather than owning it. */
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import type { SettingsSnapshot } from "./appearance";
  import type { Cut } from "./cutList";
  import { exportGifReplay, exportTrimmedReplay } from "./editorExport";
  import type { ExportFormat } from "./editorTypes";

  let {
    replayId,
    kept,
    format,
    onFormatChange,
    onExportSuccess,
  }: {
    replayId: string;
    kept: Cut[];
    format: ExportFormat;
    onFormatChange: (next: ExportFormat) => void;
    onExportSuccess: () => void;
  } = $props();

  let destinationName = $state("");
  let destinationPath = $state("");
  let exportState = $state<"idle" | "busy" | "success" | "error">("idle");
  let exportErrorCode = $state<string | null>(null);
  let lastExportPath = $state<string | null>(null);
  // The format of the export `exportState === "success"` refers to — kept
  // separate from the (possibly since-changed) `format` prop so switching
  // the toggle after a successful MP4 export never mislabels a stale
  // success as a GIF one (or offers a Library link a GIF export has none
  // of).
  let lastExportFormat = $state<ExportFormat | null>(null);
  let copyState = $state<"idle" | "busy" | "success" | "error">("idle");

  onMount(() => {
    void invoke<SettingsSnapshot>("settings_snapshot").then((snapshot) => {
      destinationPath = snapshot.save_destination;
      const segments = snapshot.save_destination.split("/");
      destinationName = segments.at(-1) ?? snapshot.save_destination;
    });
  });

  /** MP4 exports return a Library bundle id; the clipboard needs a full
   *  path, so it is derived from the known destination. GIF exports
   *  already return their published absolute path directly. */
  function fullExportPath(exportedId: string, exportedFormat: ExportFormat): string {
    return exportedFormat === "gif" ? exportedId : `${destinationPath}/${exportedId}/replay.mp4`;
  }

  export async function triggerExport(): Promise<string | null> {
    exportState = "busy";
    exportErrorCode = null;
    const result =
      format === "gif" ? await exportGifReplay(replayId, kept) : await exportTrimmedReplay(replayId, kept);
    exportState = result.ok ? "success" : "error";
    exportErrorCode = result.ok ? null : result.error;
    lastExportFormat = result.ok ? format : lastExportFormat;
    const path = result.ok ? fullExportPath(result.id, format) : null;
    lastExportPath = path ?? lastExportPath;
    if (result.ok) onExportSuccess();
    return path;
  }

  function showInLibrary() {
    void invoke("open_library_window");
  }

  async function copyToClipboard() {
    copyState = "busy";
    const path = lastExportPath ?? (await triggerExport());
    if (!path) {
      copyState = "error";
      return;
    }
    try {
      await invoke("copy_export_to_clipboard", { path });
      copyState = "success";
    } catch {
      copyState = "error";
    }
  }
</script>

<div class="editor-export-bar">
  <div class="editor-export-bar__formats" role="radiogroup" aria-label="Export format">
    <button
      type="button"
      class="editor-export-bar__format"
      class:editor-export-bar__format--active={format === "mp4"}
      onclick={() => onFormatChange("mp4")}
    >MP4</button>
    <button
      type="button"
      class="editor-export-bar__format"
      class:editor-export-bar__format--active={format === "gif"}
      onclick={() => onFormatChange("gif")}
    >GIF</button>
  </div>
  <span class="editor-export-bar__destination">{destinationName}</span>
  <button type="button" class="editor-export-bar__copy" onclick={copyToClipboard} disabled={copyState === "busy"}>
    {copyState === "success" ? "Copied" : "Copy to clipboard"}
  </button>
  {#if exportState === "success"}
    <span class="editor-export-bar__success">Exported</span>
    {#if lastExportFormat === "mp4"}
      <button type="button" class="editor-export-bar__link" onclick={showInLibrary}>Show in Library</button>
    {/if}
  {:else}
    <button
      type="button"
      class="editor-export-bar__export"
      onclick={triggerExport}
      disabled={exportState === "busy"}
    >
      {exportState === "busy" ? "Exporting…" : "Export ⌘E"}
    </button>
  {/if}
  {#if exportErrorCode}
    <span class="editor-export-bar__error" role="alert">{exportErrorCode}</span>
  {/if}
  {#if copyState === "error"}
    <span class="editor-export-bar__error" role="alert">clipboard_failed</span>
  {/if}
</div>
