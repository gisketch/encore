# Replay-Buffer Efficiency Comparison

> Date: 2026-08-13  
> Baseline: Encore's implemented macOS video-only recorder.  
> Source policy: repository code plus first-party product documentation,
> platform documentation, and official source repositories. Product claims are
> identified as such; no same-machine competitor benchmark was run.

## Checkpoint verdict

Encore's strategy is **well chosen for QA evidence and already structurally
efficient**, but it is not yet proven to be the lowest-overhead Mac recorder.

- Its steady-state hot path is narrow: ScreenCaptureKit supplies native pixel
  buffers, the app bounds pending frames, and VideoToolbox hardware H.264 does
  the compression. There is no software-encoder fallback, audio DSP, scene
  compositor, or cloud work.
- At the configured 3 Mb/s target, ten minutes of video payload is about 225 MB
  (roughly 0.375 MB/s of steady writes) before container overhead. That is a
  small sequential-I/O workload, but it is intentionally more disk activity
  than a RAM-backed replay buffer.
- Save is efficient: FFmpeg concatenates the already encoded segments with
  stream copy. It does not encode the ten minutes again. Current Save still
  probes each segment, reads all selected segments, writes the output, and runs
  `+faststart`, so it is I/O-bound rather than free.
- The disk-backed design has the strongest failure semantics in this set:
  completed ten-second files remain recoverable after process death. OBS's
  replay deque and Apple's new clip buffer are memory-backed and disappear with
  their process/stream.
- Compared with OBS for a single Mac display, Encore likely avoids work by not
  routing the frame through a general scene compositor. Compared with
  ShadowPlay, Encore cannot match driver-level game capture integration. These
  are architectural inferences, not measured rankings.
- The honest current conclusion is **competitive by design; performance rank
  unknown until a release-build measurement lane exists**.

## What Encore does today

| Stage | Implemented behavior | Efficiency consequence |
|---|---|---|
| Capture | ScreenCaptureKit, aspect-fit capped at 1920x1080, up to 30 fps, BGRA, queue depth 3, no audio | Uses Apple's high-performance capture path and bounds the OS queue; BGRA may still require color conversion before H.264. |
| Handoff | Three-item drop-oldest mailbox | Backpressure cannot grow RAM without bound or stall capture; overload sacrifices old frames explicitly. |
| Encode | AVAssetWriter/VideoToolbox H.264, hardware required, about 3 Mb/s, one-second keyframes, no frame reordering | Compression is ASIC-offloaded and output size is predictable. No software fallback protects the performance contract. |
| Rolling store | Independent fragmented MP4s finalized about every ten seconds, atomically renamed, then pruned to 5/10 minutes | Continuous bounded disk writes buy restart recovery and simple deletion. A completed segment is independently useful. |
| Save | FFprobe compatibility check, FFmpeg concat demuxer, `-c copy`, `+faststart`, transactional publish | No re-encode; reads the retained data and writes approximately one clip's worth of new data. |

The implementation is visible in Encore's
[ScreenCaptureKit configuration](../../src-tauri/src/capture/platform.rs),
[bounded mailbox](../../src-tauri/src/capture/mailbox.rs),
[hardware-writer settings](../../src-tauri/src/encoder/macos_writer.m),
[ten-second timeline](../../src-tauri/src/encoder/timeline.rs),
[rolling store](../../src-tauri/src/retention/store.rs), and
[stream-copy packager](../../src-tauri/src/packager/runner.rs).

Apple documents `minimumFrameInterval` as the capture-rate throttle and says a
larger ScreenCaptureKit queue uses more memory; its sample advises that the
default queue depth is three and should not exceed eight
([Apple ScreenCaptureKit sample](https://developer.apple.com/documentation/screencapturekit/capturing-screen-content-in-macos)).
Apple also documents VideoToolbox as direct access to hardware encoders and the
`RequireHardwareAcceleratedVideoEncoder` option as failing creation when the
hardware path is unavailable
([VideoToolbox](https://developer.apple.com/documentation/videotoolbox),
[hardware requirement](https://developer.apple.com/documentation/videotoolbox/kvtvideoencoderspecification_requirehardwareacceleratedvideoencoder)).

One caveat: Encore currently creates and finalizes a new AVAssetWriter at every
segment boundary. That makes every completed file independently recoverable,
but a continuous encoder plus fragmented muxer would avoid periodic encoder
session setup. Whether that setup is measurable at a ten-second cadence needs a
trace, not speculation.

## Against OBS Replay Buffer on macOS

OBS 32.2 is a real current Mac alternative, with an Apple Silicon build and a
replay-buffer feature
([OBS 32.2 release](https://obsproject.com/blog/obs-studio-32-2-release-notes),
[macOS download](https://obsproject.com/download),
[replay frontend API](https://github.com/obsproject/obs-studio/blob/32.2.0/docs/sphinx/reference-frontend-api.rst)).

OBS's official source establishes the key difference:

- encoded video/audio packets are reference-counted into an in-memory deque;
- old packets are purged by maximum time and/or byte size at keyframe-safe
  boundaries; and
- Save copies/reorders packet references, then a mux thread writes the selected
  packets through the FFmpeg mux helper.

See the 32.2.0
[replay implementation](https://github.com/obsproject/obs-studio/blob/32.2.0/plugins/obs-ffmpeg/obs-ffmpeg-mux.c#L849-L1186)
and
[state structure](https://github.com/obsproject/obs-studio/blob/32.2.0/plugins/obs-ffmpeg/obs-ffmpeg-mux.h).

| Dimension | Encore | OBS Replay Buffer |
|---|---|---|
| Buffer medium | Encoded MP4 segments on disk | Encoded packet deque in RAM |
| Ten minutes at the same 3 Mb/s video rate | About 225 MB payload on disk | About 225 MB payload in RAM, plus packet/allocation and any audio overhead |
| Steady writes | Yes, bounded | No replay-file write before Save |
| Process-crash survival | Completed segments recover on next launch | Unsaved packet deque is lost |
| Encode continuity | New native writer/encoder per ~10-second segment | One continuous encoder feeding the packet deque |
| Save work | Probe files, read files, stream-copy mux to new MP4 | Copy/reorder packet refs, mux packets to a new file |
| General processing | One selected source, no composition/audio | Full scene compositor, filters, scaling, multiple sources/tracks available |
| macOS encoder | Hardware-required VideoToolbox | VideoToolbox is available, but OBS also permits other configurations |

Therefore OBS has the cleaner design when the sole goal is **minimum steady
disk writes and fast ephemeral clips**. Encore has the better design when the
requirement is **recoverable QA evidence after a crash**. For Encore's product
promise, replacing its store with OBS Replay Buffer would be a regression.

The broader OBS pipeline is not automatically slower. It is mature and can use
hardware encoding, while its compositor buys features Encore deliberately does
not have. No first-party same-machine benchmark proves which has lower CPU,
GPU, RAM, or power for Encore's one-display configuration.

## Against NVIDIA Instant Replay / ShadowPlay

The current NVIDIA App page describes a 30-second Instant Replay and manual
capture up to 8K HDR30 or 4K HDR120; its predecessor's first-party page
documented a configurable last-20-minute buffer. Treat 20 minutes as legacy
GeForce Experience behavior, not the current NVIDIA App contract
([current NVIDIA App](https://www.nvidia.com/en-us/software/nvidia-app/),
[legacy Instant Replay](https://www.nvidia.com/en-us/geforce/guides/gfecnt/geforce-experience-shadowplay-is-now-share/)).
NVIDIA says NVENC is a dedicated hardware encoder
([NVENC architecture](https://www.nvidia.com/en-gb/geforce/guides/broadcasting-guide/)).
An older NVIDIA ShadowPlay page reported a 5–10% performance effect when
recording at its maximum-quality 50 Mb/s setting. That is vendor data from old
GeForce generations, not a prediction for current hardware or Encore
([NVIDIA ShadowPlay product page](https://www.nvidia.com/fr-fr/drivers/geforce-experience-shadowplay/)).

NVIDIA does not document, in the reviewed first-party sources, whether current
Instant Replay's rolling data is in system RAM, VRAM, disk, or a mixture, nor
its crash semantics or exact Save/remux path. Those fields should remain
**unknown**, not filled with folklore.

Architecturally, ShadowPlay is the likely performance ceiling for NVIDIA games:
frame acquisition is integrated with the Windows/NVIDIA graphics stack and
NVENC is a dedicated hardware block. Encore uses the analogous Apple hardware
encoder but captures through a public application API and performs its own
file lifecycle. ShadowPlay is not available on macOS and is not a practical
implementation option for Encore's Mac milestone.

## Similar Mac projects

### Rewinder

Rewinder is the closest current open-source Mac comparison. Its repository says
it uses ScreenCaptureKit, a Rust engine, a native SwiftUI shell, a capture
helper, hardware H.264 through FFmpeg/VideoToolbox, adaptive load/thermal
quality, and a battery FPS guard
([repository](https://github.com/abhinavkale-dev/rewinder)).

Its own July 2026 performance report describes a **disk-segmented**, stream-copy
pipeline: roughly half-second segments, bounded disk retention, and no
re-encode on Save. On one 64 GB Apple Silicon test Mac at 1080p60 with system
audio, microphone, and RNNoise active, the authors measured about 150–260 MB of
total physical footprint and reported capture/audio work as the dominant CPU
cost. These are useful implementation measurements, but they are vendor-run on
one heavily loaded machine and are not directly comparable to Encore's
1080p30, video-only workload
([Rewinder performance report](https://github.com/abhinavkale-dev/rewinder/blob/master/PERFORMANCE_REPORT.md)).

Rewinder's landing page and README still say the buffer is in RAM, while that
newer performance report says current retention is on disk. Treat the report as
the latest implementation snapshot and the public description as stale until
the project reconciles them. At the high level, its current design validates
Encore's choice: hardware encode + bounded disk fragments + stream-copy Save is
a credible Mac replay architecture.

Encore should plausibly be lighter in video CPU than that reported Rewinder
configuration because it runs at 30 fps and has no audio mix or denoiser.
Rewinder may be more adaptive under load and battery, while Encore avoids a
long-lived FFmpeg encode process but pays for Tauri/WebView UI processes and
periodic writer creation. Only a controlled measurement can settle the total.

### Backtrack and Apple APIs

Backtrack is a shipping Mac menu-bar product whose App Store page says it
locally overwrites a 60-minute window, optionally up to five hours, and saves
screen plus audio. It publishes no capture, buffer-medium, encode, crash, or
resource architecture, so it proves market existence but cannot support an
efficiency ranking
([Backtrack App Store page](https://apps.apple.com/us/app/backtrack-record-the-past/id1477089520?mt=12)).

Apple now has a beta `SCClipBufferingOutput`. It buffers up to 15 seconds in
memory and can export a clip without interrupting capture
([API](https://developer.apple.com/documentation/screencapturekit/scstream/addclipbufferingoutput%28_%3A%29),
[Apple sample](https://developer.apple.com/documentation/screencapturekit/capturing-screen-content-on-ios)).
It is elegant for a very short ephemeral replay, but cannot replace Encore's
five/ten-minute window or disk-backed crash survival.

Screen Studio is not currently a replay-buffer alternative: its official
workflow starts a recording explicitly, and its public request for a replay
buffer remains “In Review”
([recording workflow](https://screen.studio/guide/starting-finishing-the-recording),
[feature request](https://hub.screen.studio/p/please-add-a-replay-buffer)).

## Best next performance work

Do not redesign from marketing comparisons. Add a repeatable release-build
benchmark and test Encore against OBS with exactly the same source, 1080p30,
3 Mb/s H.264 VideoToolbox, ten-minute duration, and audio disabled.

Measure at idle desktop and high motion, on AC and battery:

1. app plus helper physical footprint (`vmmap`, not summed RSS alone);
2. CPU time, GPU activity, thermal pressure, and package power;
3. rolling bytes written per second and storage high-water mark;
4. captured/dropped frames and output frame cadence;
5. Save latency, bytes read/written, and capture drops during Save; and
6. force-kill recovery: usable evidence duration after restart.

The highest-value experiments after a baseline are:

1. Switch ScreenCaptureKit from BGRA to supported bi-planar `420v` and measure
   whether avoiding RGB-to-YUV conversion reduces memory bandwidth or power.
   Apple documents both formats as supported
   ([pixel formats](https://developer.apple.com/documentation/screencapturekit/scstreamconfiguration/pixelformat)).
2. Move segment compatibility validation to admission/recovery so Save does not
   start one FFprobe process per retained file.
3. Prototype one long-lived VideoToolbox compression session with crash-safe
   fragmented muxing, then compare it with the current writer-per-segment path.
4. Add a battery/thermal policy only after the baseline shows it is needed;
   a 15-fps or lower-bitrate idle/battery mode trades evidence fidelity for
   energy and must be product-visible.

## Bottom line

For a QA tool where “the crash is the bug,” Encore makes the right exchange:
roughly a few tenths of a megabyte per second of bounded disk writes buys
recoverability that RAM replay buffers cannot provide. Its hardware encoder and
stream-copy Save are efficient fundamentals. The next step is measurement and
two targeted experiments—not libobs migration or a wholesale rewrite.
