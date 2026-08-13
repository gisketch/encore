import type { PersistedTarget } from "./recordingSettings";

export type Appearance = "light" | "dark" | "system";

/** Snapshot shape shared by `settings_snapshot` and the `settings-changed`
 *  event; grows as later tickets add settings sections. */
export type SettingsSnapshot = {
  appearance: Appearance;
  retention_minutes: 5 | 10;
  default_target: PersistedTarget;
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
