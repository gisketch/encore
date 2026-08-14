<script lang="ts">
  /** The preview card's media area and its fallback chain: looping muted
   *  video of the saved replay, then the still thumbnail, then the striped
   *  placeholder. Something is always drawn — the media box is never blank
   *  or broken, which is the spec's acceptance criterion.
   *
   *  Split out of `PreviewWindow.svelte` (tracked by the SCC gate at its
   *  PP-02 complexity) so the branching lives here, following the
   *  `PreviewActions` precedent.
   *
   *  The video loads through `convertFileSrc`, exactly as the Editor does;
   *  `preview::show` grants the asset protocol the save destination first,
   *  mirroring `editor::open`. */
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { PreviewPayload } from "./previewTypes";

  let {
    payload,
    thumbnailUrl,
    active,
  }: {
    payload: PreviewPayload | null;
    thumbnailUrl: string | null;
    active: boolean;
  } = $props();

  /** Read once, like `CaptureShell`'s own check. Under reduced motion the
   *  card shows the still frame and never autoplays. */
  const REDUCED_MOTION = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  let videoEl = $state<HTMLVideoElement | undefined>();
  let videoFailed = $state(false);

  /** A replay whose file cannot be decoded (or is already gone) must fall
   *  back visibly rather than leave an empty box, and the next replay
   *  deserves a fresh attempt — this row never remounts, so the reset is
   *  explicit. */
  $effect(() => {
    payload;
    videoFailed = false;
  });

  /** Releasing the element is the point: when the card is dismissed or the
   *  payload swaps, `videoEl` becomes undefined and this cleanup runs with
   *  the outgoing element, stopping decode behind a hidden window instead
   *  of leaving a 10-minute 1080p file playing to nobody. */
  $effect(() => {
    const outgoing = videoEl;
    return () => release(outgoing);
  });

  function release(element: HTMLVideoElement | undefined) {
    element?.pause();
    element?.removeAttribute("src");
    element?.load();
  }

  let playable = $derived(active && !REDUCED_MOTION && !videoFailed && payload !== null);
  let videoSrc = $derived(payload === null ? "" : convertFileSrc(payload.videoPath));
</script>

{#if playable}
  <!-- svelte-ignore a11y_media_has_caption -->
  <video
    bind:this={videoEl}
    class="preview-still__video"
    src={videoSrc}
    autoplay
    loop
    muted
    playsinline
    preload="auto"
    onerror={() => (videoFailed = true)}
  ></video>
{:else if thumbnailUrl}
  <img class="preview-still__image" src={thumbnailUrl} alt="" />
{:else}
  <span class="preview-still__label">Screen frame</span>
{/if}
