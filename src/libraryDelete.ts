/** Invokes the `delete_replay` command, collapsing any failure to a
 *  wire-safe error code the card can show inline. Kept as its own module
 *  (rather than inline in `LibraryCard.svelte`/`LibraryCardDelete.svelte`)
 *  so the async/catch plumbing lives where the harness's TypeScript
 *  complexity-1 ceiling has room for it. */

import { invoke } from "@tauri-apps/api/core";

export type DeleteReplayResult = { ok: true } | { ok: false; error: string };

const DEFAULT_ERROR = "library_delete_failed";

export async function deleteReplay(id: string): Promise<DeleteReplayResult> {
  return invoke("delete_replay", { id })
    .then(() => ({ ok: true as const }))
    .catch((error: unknown) => ({ ok: false as const, error: errorCode(error) }));
}

function errorCode(error: unknown): string {
  return typeof error === "string" ? error : DEFAULT_ERROR;
}
