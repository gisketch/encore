<script lang="ts">
  import { animate, stagger } from "motion";
  import { onMount } from "svelte";

  const segmentIds = Array.from({ length: 60 }, (_, index) => index);

  let timeline: HTMLElement;
  let retention = $state<5 | 10>(10);
  let captureTarget = $state("Full screen");
  const visibleSegments = $derived(retention === 10 ? 60 : 30);

  onMount(() => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    const segments = timeline.querySelectorAll(".timeline__segment");
    const latest = timeline.querySelector(".timeline__segment--latest");
    const reveal = animate(
      segments,
      { opacity: [0.22, 1], transform: ["scaleY(0.35)", "scaleY(1)"] },
      { delay: stagger(0.018), duration: 0.32, ease: "easeOut" },
    );

    if (!latest) return () => reveal.stop();

    const pulse = animate(
      latest,
      { opacity: [0.48, 1, 0.48] },
      { duration: 1.8, repeat: Infinity, ease: "easeInOut" },
    );

    return () => {
      reveal.stop();
      pulse.stop();
    };
  });
</script>

<svelte:head>
  <title>Encore — Replay buffer</title>
</svelte:head>

<main class="shell">
  <header class="titlebar" data-tauri-drag-region>
    <a class="brand" href="#status" aria-label="Encore home">
      <span class="brand__mark" aria-hidden="true">
        <span></span><span></span><span></span>
      </span>
      <span>encore</span>
    </a>
    <span class="preview-label">Interface preview</span>
  </header>

  <section class="workspace" id="status">
    <div class="intro">
      <p class="eyebrow">Local replay / macOS</p>
      <h1>The moment<br />before.</h1>
      <p class="lede">
        Keep a short window of evidence ready, then save it only when a bug appears.
      </p>
    </div>

    <section class="recorder" aria-labelledby="recorder-title">
      <div class="recorder__header">
        <div>
          <p class="utility-label">Capture status</p>
          <h2 id="recorder-title">Shell ready</h2>
        </div>
        <span class="status-pill">
          <span class="status-pill__dot" aria-hidden="true"></span>
          Not recording
        </span>
      </div>

      <p class="recorder__message" role="status">
        The interface is running. Native screen capture is the next implementation slice.
      </p>

      <div class="timeline-frame">
        <div class="timeline-meta">
          <span>Rolling window</span>
          <strong>{retention}:00</strong>
        </div>
        <div
          class="timeline"
          bind:this={timeline}
          aria-label={`${retention}-minute replay timeline preview`}
        >
          {#each segmentIds.slice(-visibleSegments) as segment}
            <span
              class:timeline__segment--latest={segment === 59}
              class="timeline__segment"
              aria-hidden="true"
            ></span>
          {/each}
        </div>
        <div class="timeline-scale" aria-hidden="true">
          <span>−{retention}:00</span>
          <span>now</span>
        </div>
      </div>

      <button class="save-button" type="button" disabled>
        <span>Save last {retention} minutes</span>
        <kbd>Not assigned</kbd>
      </button>
      <p class="button-note">Available when capture and the global shortcut are connected.</p>
    </section>

    <aside class="settings" aria-label="Replay settings">
      <div class="settings__heading">
        <p class="utility-label">Replay setup</p>
        <span>Local only</span>
      </div>

      <label class="field">
        <span>Capture target</span>
        <select bind:value={captureTarget} aria-describedby="target-note">
          <option>Full screen</option>
          <option>Application window</option>
        </select>
      </label>
      <p class="field-note" id="target-note">Selected: {captureTarget}. Capture is not connected yet.</p>

      <fieldset class="field">
        <legend>Keep the last</legend>
        <div class="segmented-control">
          <button
            class:active={retention === 5}
            type="button"
            aria-pressed={retention === 5}
            onclick={() => (retention = 5)}>5 min</button
          >
          <button
            class:active={retention === 10}
            type="button"
            aria-pressed={retention === 10}
            onclick={() => (retention = 10)}>10 min</button
          >
        </div>
      </fieldset>

      <div class="storage-note">
        <span class="storage-note__icon" aria-hidden="true">↳</span>
        <div>
          <strong>Nothing leaves this Mac</strong>
          <span>Saved clips will live in Movies/Encore.</span>
        </div>
      </div>
    </aside>
  </section>

  <footer>
    <span>Encore 0.1.0</span>
    <span>Capture engine pending</span>
  </footer>
</main>
