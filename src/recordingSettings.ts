/** Best-effort capture target identity persisted by the Rust settings
 *  module; mirrors `capture::settings::PersistedTarget`'s wire shape. */
export type PersistedTarget =
  | { kind: "display" }
  | { kind: "window"; bundle_id: string; title: string };

/** The subset of `list_capture_sources`' response this section needs to
 *  match a persisted target against a live, pickable source. */
export type RecordingSource = {
  id: string;
  kind: "display" | "window";
  label: string;
  bundleId: string | null;
  title: string | null;
};

const sourceKey = (source: RecordingSource) =>
  `${source.kind}|${source.bundleId ?? ""}|${source.title ?? ""}`;

const targetKey = (target: PersistedTarget) =>
  `${target.kind}|${(target as { bundle_id?: string }).bundle_id ?? ""}|${(target as { title?: string }).title ?? ""}`;

/** Finds which of `sources` the persisted default target refers to, by
 *  bundle id + title (windows have no stable id across launches). Returns
 *  `""` when nothing matches, so the caller can fall back to an empty
 *  selection rather than guessing. */
export function selectedSourceId(target: PersistedTarget, sources: RecordingSource[]): string {
  const byKey = new Map(sources.map((source) => [sourceKey(source), source.id]));
  return byKey.get(targetKey(target)) ?? "";
}
