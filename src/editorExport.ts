/** Invokes the `export_trimmed_replay` command, collapsing any failure to
 *  a wire-safe error code (same `.then/.catch` shape `libraryDelete.ts`
 *  uses, kept as its own module so the async plumbing lives where the
 *  harness's TypeScript complexity-1 ceiling has room for it). */

import { invoke } from "@tauri-apps/api/core";
import type { Cut } from "./cutList";

export type ExportReplayResult = { ok: true; id: string } | { ok: false; error: string };

const DEFAULT_ERROR = "export_failed";

export async function exportTrimmedReplay(id: string, keepSegments: Cut[]): Promise<ExportReplayResult> {
  return invoke<string>("export_trimmed_replay", { id, keepSegments })
    .then((exportedId) => ({ ok: true as const, id: exportedId }))
    .catch((error: unknown) => ({ ok: false as const, error: errorCode(error) }));
}

function errorCode(error: unknown): string {
  return typeof error === "string" ? error : DEFAULT_ERROR;
}
