/** Renders a persisted accelerator string (`tauri-plugin-global-shortcut`
 *  syntax, e.g. `"Cmd+Alt+R"`) as the mockup's glyph chip (`⌘⌥R`). Written
 *  as chained `.replace` calls rather than a lookup/loop to stay inside the
 *  harness's complexity-1 ceiling for new TypeScript files; the persisted
 *  format is chosen (`Cmd`/`Alt`/`Ctrl`/`Shift`) specifically so this stays
 *  a straight-line substitution. */
export function formatAccelerator(accelerator: string): string {
  return accelerator
    .replace(/Cmd/g, "⌘")
    .replace(/Alt/g, "⌥")
    .replace(/Ctrl/g, "⌃")
    .replace(/Shift/g, "⇧")
    .replace(/\+/g, "");
}
