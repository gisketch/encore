import type { PersistedTarget } from "./recordingSettings";

export type Appearance = "light" | "dark" | "system";

export type AfterSave = "preview" | "reveal" | "nothing" | "open_editor";

export type HotkeyId = "save_replay" | "pause_capture" | "open_library";

/** Persisted accelerator strings for the three rebindable hotkeys, in
 *  `tauri-plugin-global-shortcut` syntax (e.g. `"Cmd+Alt+R"`). */
export type Hotkeys = Record<HotkeyId, string>;

/** Snapshot shape shared by `settings_snapshot` and the `settings-changed`
 *  event; grows as later tickets add settings sections. */
export type SettingsSnapshot = {
  appearance: Appearance;
  retention_minutes: 5 | 10;
  default_target: PersistedTarget;
  save_destination: string;
  after_save: AfterSave;
  hotkeys: Hotkeys;
  menu_bar_mode: boolean;
  save_sound: boolean;
};

const APPLY: Record<Appearance, () => void> = {
  light: () => document.documentElement.setAttribute("data-theme", "light"),
  dark: () => document.documentElement.setAttribute("data-theme", "dark"),
  system: () => document.documentElement.removeAttribute("data-theme"),
};

/** Applies the persisted appearance choice to this window's root element:
 *  an explicit light/dark choice sets `data-theme`, system removes it so
 *  theme.css's `prefers-color-scheme` block tracks the OS live. */
export function applyAppearance(value: Appearance): void {
  APPLY[value]();
}
