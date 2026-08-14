/** The post-save preview's payload, mirroring Rust's `PreviewPayload`.
 *  `durationSeconds` is null for a bundle that never recorded an evidence
 *  window, in which case the meta line omits the duration segment rather
 *  than inventing one. */

export type PreviewPayload = {
  id: string;
  displayName: string;
  durationSeconds: number | null;
  totalBytes: number;
  videoPath: string;
};
