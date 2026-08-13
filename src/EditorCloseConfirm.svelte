<script lang="ts">
  /** In-app confirm overlay for closing the Editor with unexported edits,
   *  the same overlay pattern `LibraryCardDelete.svelte` uses — except the
   *  trigger (back link / close dot) lives outside this component, so
   *  visibility is driven by the `show` prop rather than local state. */
  let {
    show,
    onCancel,
    onDiscard,
  }: {
    show: boolean;
    onCancel: () => void;
    onDiscard: () => void;
  } = $props();
</script>

{#if show}
  <div class="editor-discard" role="alertdialog" aria-label="Discard unexported edits?">
    <p class="editor-discard__text">Discard your unexported edits?</p>
    <div class="editor-discard__actions">
      <button type="button" class="editor-discard__cancel" onclick={onCancel}>Keep editing</button>
      <button type="button" class="editor-discard__confirm" onclick={onDiscard}>Discard</button>
    </div>
  </div>
{/if}
