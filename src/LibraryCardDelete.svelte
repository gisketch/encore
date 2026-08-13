<script lang="ts">
  import { deleteReplay } from "./libraryDelete";
  import type { LibraryEntry } from "./libraryTypes";

  let { entry, onDeleted }: { entry: LibraryEntry; onDeleted: (id: string) => void } = $props();

  let confirming = $state(false);
  let deleting = $state(false);
  let errorCode = $state<string | null>(null);

  function openConfirm(event: MouseEvent) {
    event.stopPropagation();
    confirming = true;
    errorCode = null;
  }

  function cancelConfirm(event: MouseEvent) {
    event.stopPropagation();
    confirming = false;
  }

  async function confirmDelete(event: MouseEvent) {
    event.stopPropagation();
    deleting = true;
    const result = await deleteReplay(entry.id);
    deleting = false;
    if (result.ok) {
      onDeleted(entry.id);
      return;
    }
    confirming = false;
    errorCode = result.error;
  }
</script>

<button
  type="button"
  class="library-card__delete"
  onclick={openConfirm}
  aria-label={`Delete ${entry.displayName}`}
>
  ×
</button>

{#if confirming}
  <div class="library-card__confirm" role="alertdialog" aria-label={`Delete ${entry.displayName}?`}>
    <p class="library-card__confirm-text">Delete "{entry.displayName}"?</p>
    <div class="library-card__confirm-actions">
      <button type="button" class="library-card__confirm-cancel" onclick={cancelConfirm}>
        Cancel
      </button>
      <button
        type="button"
        class="library-card__confirm-delete"
        onclick={confirmDelete}
        disabled={deleting}
      >
        {deleting ? "Deleting…" : "Delete"}
      </button>
    </div>
  </div>
{/if}

{#if errorCode}
  <p class="library-card__delete-error" role="alert">{errorCode}</p>
{/if}
