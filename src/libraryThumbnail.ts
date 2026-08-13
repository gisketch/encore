/** Fetches one Library card's cached thumbnail from the `library_thumbnail`
 *  command as a data URL, generating it on the Rust side on a cache miss.
 *  Resolves to `null` on any failure (including running outside Tauri) so
 *  the card can fall back permanently to its styled placeholder — callers
 *  never need to distinguish failure reasons. */

import { invoke, isTauri } from "@tauri-apps/api/core";

export async function fetchThumbnailDataUrl(id: string): Promise<string | null> {
  return isTauri()
    ? invoke<string>("library_thumbnail", { id })
        .then((base64) => `data:image/jpeg;base64,${base64}`)
        .catch(() => null)
    : null;
}
