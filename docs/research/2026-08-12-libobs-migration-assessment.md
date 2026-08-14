# libobs Migration Assessment

> Date: 2026-08-12  
> Baseline reviewed: OBS Studio 32.1.2 (`fb4d98bf`), Encore's implemented
> macOS capture/video pipeline, and Encore's approved rolling-buffer contract.  
> Source policy: official OBS source, API documentation, build configuration,
> release metadata, and license only.

## Recommendation

**Do not migrate Encore to libobs now.** Continue the native macOS pipeline and
implement the bounded on-disk rolling store next. Reconsider libobs when Windows,
audio, or multi-source composition becomes an active milestone.

libobs is a mature capture/encoding engine, but it is not a simpler replacement
for the feature Encore needs next. OBS's macOS capture plugin uses the same
ScreenCaptureKit technology Encore already uses, and OBS's named replay buffer
keeps encoded packets in memory until Save. That conflicts with Encore's core
requirement that useful evidence survive a recorder or tested-app crash.

The strongest future reason to adopt libobs is Windows: its platform modules
already contain display/window/game capture and hardware encoder integrations.
That benefit is real, but adopting the engine now would add a C FFI boundary,
GPU composition runtime, plugin/data discovery, FFmpeg helper packaging, and
GPL distribution obligations before Encore has finished its small macOS MVP.

## What would actually be replaced

Encore currently has narrow, product-shaped boundaries:

- ScreenCaptureKit produces native frames through `screencapturekit-rs`.
- A bounded newest-frame mailbox feeds a dedicated encoder worker.
- AVFoundation/VideoToolbox writes atomic, independently playable 10-second MP4
  segments.
- The service owns typed permission, source-switching, recovery, pause, and
  privacy-safe diagnostic state.

With libobs, Encore would instead initialize the C core, initialize video,
manually load modules, create/configure a source, attach it to an output channel,
create a hardware encoder and output, connect signals, and release all reference-
counted objects before shutdown. That lifecycle is explicitly assigned to the
embedding frontend in the [official frontend guide](https://github.com/obsproject/obs-studio/blob/32.1.2/docs/sphinx/frontends.rst#L1-L32),
and the [output/encoder example](https://github.com/obsproject/obs-studio/blob/32.1.2/docs/sphinx/frontends.rst#L217-L251)
is C/C++. Because Encore's core is Rust, a maintained FFI adapter and a safe
ownership/threading layer would be new Encore code.

The macOS source would not switch to fundamentally different capture technology.
OBS's current `screen_capture` plugin:

- builds ScreenCaptureKit filters for a display, window, or application
  ([source](https://github.com/obsproject/obs-studio/blob/32.1.2/plugins/mac-capture/mac-sck-video-capture.m#L74-L194));
- configures and starts an `SCStream`
  ([source](https://github.com/obsproject/obs-studio/blob/32.1.2/plugins/mac-capture/mac-sck-video-capture.m#L197-L272));
- converts the captured IOSurface into an OBS graphics texture and renders it
  through the OBS compositor
  ([source](https://github.com/obsproject/obs-studio/blob/32.1.2/plugins/mac-capture/mac-sck-video-capture.m#L320-L369)); and
- registers that implementation as the `screen_capture` OBS input
  ([source](https://github.com/obsproject/obs-studio/blob/32.1.2/plugins/mac-capture/mac-sck-video-capture.m#L688-L712)).

OBS does add useful capture behavior such as display/window/application modes,
cursor control, excluding its own application, child-window capture on newer
macOS, and optional system audio. Those are visible in the same
[filter/configuration code](https://github.com/obsproject/obs-studio/blob/32.1.2/plugins/mac-capture/mac-sck-video-capture.m#L103-L219).

## Replay-buffer mismatch

The built-in OBS replay output is **RAM-backed, not a rolling disk store**:

- its state owns a `deque packets` plus a second packet array used during Save
  ([source](https://github.com/obsproject/obs-studio/blob/32.1.2/plugins/obs-ffmpeg/obs-ffmpeg-mux.h#L12-L67));
- incoming encoded packets are reference-counted into that deque, then old
  packets are purged by time/size limits
  ([source](https://github.com/obsproject/obs-studio/blob/32.1.2/plugins/obs-ffmpeg/obs-ffmpeg-mux.c#L1046-L1061),
  [source](https://github.com/obsproject/obs-studio/blob/32.1.2/plugins/obs-ffmpeg/obs-ffmpeg-mux.c#L1203-L1247)); and
- only a Save copies/reorders those packets and starts a mux thread that writes
  the file
  ([source](https://github.com/obsproject/obs-studio/blob/32.1.2/plugins/obs-ffmpeg/obs-ffmpeg-mux.c#L1090-L1187)).

At Encore's target 3 Mbps, 10 minutes of video payload alone is about 225 MB
(roughly 215 MiB), before packet/allocation overhead. More importantly, a process
crash loses the unsaved deque. Using this output would regress the approved
crash-recovery behavior.

There is a closer OBS building block: the `ffmpeg_muxer` output supports timed
file splitting at video keyframes and emits a `file_changed` signal
([source](https://github.com/obsproject/obs-studio/blob/32.1.2/plugins/obs-ffmpeg/obs-ffmpeg-mux.c#L689-L759)).
Encore could run this as a continuous split recording and retain/prune completed
files itself. It is still not drop-in parity:

- muxing uses the separately packaged `obs-ffmpeg-mux` helper process;
- `file_changed` is emitted after sending the new filename, not after an Encore-
  style atomic publish/validation acknowledgement; and
- Encore still needs startup recovery, safe admission, pinning, pruning, export
  coordination, and its authoritative state model.

Therefore libobs does not remove the next rolling-buffer ticket. At best it
replaces capture/encode/mux internals while Encore retains the product-specific
storage layer.

## Hardware encoding and performance

Hardware H.264 is available. The macOS VideoToolbox plugin enumerates the
platform encoder list, records whether each encoder is hardware accelerated, and
registers the available H.264/HEVC/ProRes encoders dynamically
([source](https://github.com/obsproject/obs-studio/blob/32.1.2/plugins/mac-videotoolbox/encoder.c#L1400-L1527)).

This is not evidence that libobs will be faster for Encore's one-source path.
Encore currently passes the ScreenCaptureKit pixel buffer directly into a native
hardware writer; OBS's source passes through an OBS graphics texture/compositor
before the encoder. libobs is designed for high-performance real-time mixing,
but the official sources provide no comparable one-source CPU benchmark.
Performance should be treated as unknown until a same-machine release-build
spike measures CPU, GPU, memory, dropped frames, and power. Given Encore's
already-low measured CPU use, performance is not a reason to rewrite.

## Embedding and API stability

libobs exposes a broad public C API; this is a real embedding API, not UI
automation. The core declares that it uses semantic versioning: major for
breaking changes, minor for backward-compatible additions, and patch for fixes
([source](https://github.com/obsproject/obs-studio/blob/32.1.2/libobs/obs-config.h#L20-L48)).

The plugin boundary is version-checked. OBS 32.1.2 rejects modules compiled
against a newer major/minor libobs while ignoring the patch component
([source](https://github.com/obsproject/obs-studio/blob/32.1.2/libobs/obs-module.c#L160-L176)).
This is useful protection, but Encore should still pin and ship one matched set
of libobs, plugins, helper binaries, and module data. The source IDs and their
settings keys are plugin implementation contracts, not a separately documented
stable Rust API.

The convenient `obs_frontend_replay_buffer_*` functions belong to the OBS Studio
frontend API, whose implementation calls the Qt application's main window
([API](https://github.com/obsproject/obs-studio/blob/32.1.2/docs/sphinx/reference-frontend-api.rst#L660-L680),
[implementation](https://github.com/obsproject/obs-studio/blob/32.1.2/frontend/OBSStudioAPI.cpp#L291-L313)).
A Tauri frontend would create and drive the `replay_buffer` or `ffmpeg_muxer`
output through the core output API instead
([API](https://github.com/obsproject/obs-studio/blob/32.1.2/docs/sphinx/reference-outputs.rst#L326-L412)).

## Build, packaging, and runtime footprint

The upstream build is significantly heavier than Encore's current native bridge:

- the root build requires CMake 3.28 and builds `libobs`, a graphics backend,
  plugins, tests, and the frontend; UI, scripting, and HEVC are build options
  ([source](https://github.com/obsproject/obs-studio/blob/32.1.2/CMakeLists.txt#L1-L37));
- libobs itself requires FFmpeg libraries, zlib, jansson, uthash, SIMDe, and
  threads
  ([source](https://github.com/obsproject/obs-studio/blob/32.1.2/libobs/CMakeLists.txt#L1-L30));
- the default macOS dependency setup fetches OBS prebuilt dependencies, Qt 6,
  and CEF (CEF is skipped when browser support is disabled)
  ([preset](https://github.com/obsproject/obs-studio/blob/32.1.2/CMakePresets.json#L23-L109),
  [fetch logic](https://github.com/obsproject/obs-studio/blob/32.1.2/cmake/common/buildspec_common.cmake#L41-L86)); and
- OBS 32.1.2 builds with an Xcode generator and requires Xcode 16 / macOS SDK 15
  ([source](https://github.com/obsproject/obs-studio/blob/32.1.2/cmake/macos/compilerconfig.cmake#L8-L62)).

A headless Encore build can disable Qt UI, browser, scripting, and unrelated
plugins. The required bundle would still include libobs, a graphics backend,
`mac-capture`, `mac-videotoolbox`, `obs-ffmpeg`, FFmpeg dependencies, locale/data
files, and the mux helper. Upstream does not publish a minimal-libobs package, so
the exact reduced size needs a spike.

For scale only, the official full OBS 32.1.2 release metadata reports a
187,527,877-byte Apple Silicon DMG (178.8 MiB) and a 197,263,771-byte Intel DMG
(188.1 MiB). Those include the full OBS application, Qt, and optional features,
so they are an upper bound rather than an Encore estimate
([official release metadata](https://api.github.com/repos/obsproject/obs-studio/releases/tags/32.1.2)).

OBS 32.1.2's deployment target and published macOS support are macOS 12.0, which
is compatible with Encore's stricter macOS 14+ target
([preset](https://github.com/obsproject/obs-studio/blob/32.1.2/CMakePresets.json#L87-L109),
[official download page](https://obsproject.com/download)).

## Windows implications

This is the best argument for libobs later:

- the root build includes D3D11 and WinRT graphics backends on Windows
  ([source](https://github.com/obsproject/obs-studio/blob/32.1.2/CMakeLists.txt#L22-L31));
- `win-capture` registers game, monitor, and window sources, with Windows
  Graphics Capture paths present for monitor/window capture
  ([registration](https://github.com/obsproject/obs-studio/blob/32.1.2/plugins/win-capture/plugin-main.c#L132-L152),
  [window WGC selection](https://github.com/obsproject/obs-studio/blob/32.1.2/plugins/win-capture/window-capture.c#L138-L187)); and
- the Windows plugin set contains NVENC, QSV, and AMF encoder integrations. OBS
  registers FFmpeg NVENC only after a support check
  ([source](https://github.com/obsproject/obs-studio/blob/32.1.2/plugins/obs-ffmpeg/obs-ffmpeg.c#L336-L374)),
  while QSV registers its available encoder variants
  ([source](https://github.com/obsproject/obs-studio/blob/32.1.2/plugins/obs-qsv11/obs-qsv11-plugin-main.c#L67-L105)).

libobs therefore reduces the amount of platform media code Encore would own on
Windows. It does not erase platform policy: Encore must still choose and persist
platform-specific source settings, select only a proven hardware encoder, map
module failures into its state model, package the matching plugins/helpers, and
test each GPU/driver path.

## GPL obligations

OBS Studio and libobs are GPL version 2 **or later**, not LGPL
([official README](https://github.com/obsproject/obs-studio/blob/32.1.2/README.rst#L12-L25),
[license](https://github.com/obsproject/obs-studio/blob/32.1.2/COPYING)).

For a distributed combined/derived work, GPLv2 section 2 requires licensing the
work as a whole under the GPL; section 3 requires providing complete
corresponding source (or a qualifying written offer), including build scripts
([section 2](https://github.com/obsproject/obs-studio/blob/32.1.2/COPYING#L90-L132),
[section 3](https://github.com/obsproject/obs-studio/blob/32.1.2/COPYING#L134-L170)).

Practical consequence: do not embed or link libobs into a distributed Encore
binary unless Encore is intentionally distributed under GPL-compatible terms
with complete corresponding source and notices. Encore is intended to be public,
but this repository currently has no visible top-level license file, so licensing
must be resolved before a libobs migration. This is an engineering risk summary,
not legal advice; counsel should confirm the distribution model.

## Effort and decision trigger

| Path | Rough effort | What it buys |
|---|---:|---|
| Keep native pipeline; build rolling store | Days | Completes the immediate crash-safe MVP requirement with the working low-overhead path |
| Headless libobs feasibility spike | 3–5 engineering days | Measures real bundle size/performance and proves Rust/Tauri → SCK source → hardware VT → 10-second split files |
| Migrate macOS to product parity | About 2–4 engineering weeks | Replaces working capture/encode internals; still needs Encore retention, state, recovery, export, QA, signing, and packaging |
| Add Windows later with libobs | Separate multi-week milestone | Reuses mature Windows capture and encoder modules; strongest potential long-term payoff |

These are planning estimates, not claims from OBS.

Reopen the decision when at least one is true:

1. Windows becomes the next committed milestone.
2. System audio or multi-source composition enters MVP scope.
3. Maintaining native capture/encoder adapters is demonstrably costing more than
   maintaining a pinned libobs distribution.

At that point, run the 3–5 day spike before changing architecture. Its pass gate
should require: hardware H.264 only, 10-second independently playable disk files,
prior-file survival after force-kill, source-loss/state mapping, no capture
content in logs, signed/notarized packaging, and CPU/GPU/RAM measurements no worse
than an agreed threshold over the current release build.
