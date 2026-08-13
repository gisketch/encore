<script lang="ts">
  import type { ExportFormat } from "./editorTypes";

  let {
    title,
    specLine,
    format,
    onBack,
    onClose,
  }: {
    title: string;
    specLine: string;
    format: ExportFormat;
    onBack: () => void;
    onClose: () => void;
  } = $props();
</script>

<header class="editor-titlebar" data-tauri-drag-region="deep">
  <span class="traffic-lights">
    <button
      type="button"
      class="traffic-dot traffic-dot--close"
      title="Close"
      aria-label="Close editor"
      onclick={onClose}
    ></button>
    <i class="traffic-dot traffic-dot--minimize" aria-hidden="true"></i>
    <i class="traffic-dot traffic-dot--zoom" aria-hidden="true"></i>
  </span>
  <button type="button" class="editor-back" onclick={onBack}>‹ Library</button>
  <h1 class="editor-title">{title}</h1>
  {#if specLine}
    <span class="editor-spec">{specLine}</span>
  {/if}
  <span class="spacer" aria-hidden="true"></span>
  {#if format === "gif"}
    <span class="editor-badge editor-badge--gif"><i aria-hidden="true"></i>GIF — re-encoded</span>
  {:else}
    <span class="editor-badge"><i aria-hidden="true"></i>lossless — no re-encode</span>
  {/if}
</header>
