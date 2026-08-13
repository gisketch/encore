/** Wire shapes for `library_index`, mirroring
 *  `src-tauri/src/library/{scan,group,mod}.rs`'s serialized structs. */

export type LibraryEntry = {
  id: string;
  displayName: string;
  savedAtUnixMs: number;
  durationSeconds: number | null;
  totalBytes: number;
  trimmed: boolean;
};

export type LibraryGroup = {
  label: string;
  count: number;
  totalBytes: number;
  entries: LibraryEntry[];
};

export type LibraryIndex = {
  groups: LibraryGroup[];
  totalCount: number;
  totalBytes: number;
};
