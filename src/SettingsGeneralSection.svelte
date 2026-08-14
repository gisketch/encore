<script lang="ts">
  /** Settings → General. The "Show in menu bar" toggle it used to carry was
   *  retired in MB-02: the menu bar is Encore's permanent control surface
   *  rather than a mode, so there is nothing left for the toggle to mean.
   *  With only the appearance control left, this section needs no data of
   *  its own — its value and setter come from the window. */
  import type { Appearance } from "./appearance";
  import SettingsAppearanceControl from "./SettingsAppearanceControl.svelte";

  let { appearance, onSetAppearance }: {
    appearance: Appearance;
    onSetAppearance: (value: Appearance) => void;
  } = $props();
</script>

<section class="settings-section" aria-labelledby="general-heading">
  <h2 id="general-heading" class="settings-section__title">General</h2>

  <div class="settings-row">
    <span class="settings-row__label">Appearance</span>
    <SettingsAppearanceControl value={appearance} onSelect={onSetAppearance} />
  </div>
</section>
