/** Wire shapes for `editor_header`/`editor_keyframes`, mirroring
 *  `src-tauri/src/editor/{header,keyframes}.rs`'s serialized structs. */

export type EditorHeader = {
  id: string;
  title: string;
  width: number | null;
  height: number | null;
  totalBytes: number;
  videoPath: string;
};

export type EditorKeyframes = {
  seconds: number[];
  durationSeconds: number;
};

/** The export bar's format toggle (PG-15): MP4 stays lossless, GIF is
 *  always a re-encode. */
export type ExportFormat = "mp4" | "gif";
