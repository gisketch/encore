# Releasing and Hosting

How an Encore build reaches a Mac, and what is still missing before that
experience is genuinely seamless.

## What runs today

| Workflow | Trigger | Result |
|---|---|---|
| `CI` | push to `main`, PRs | Full validation on `macos-26` |
| `Release` | tag `v*`, manual | `.app` + `.dmg`, uploaded and attached to a GitHub Release |
| `Site` | push touching `landing-page.html` | Deploys the page to the VPS |

Runners are `macos-26`. This is not a preference: `screencapturekit` pulls in
`apple-metal`, whose Swift bridge compiles against macOS 26 Metal APIs
(`MTLSamplerReductionMode`, `lodBias`), so `macos-14` and `macos-15` both
fail to build the dependency at all.

## Cutting a release

1. Bump `version` in `src-tauri/tauri.conf.json`.
2. Commit, then tag with the matching version: `git tag v0.2.0 && git push origin v0.2.0`.

The workflow refuses to build when the tag and the config version disagree,
so a DMG can never ship labelled with the wrong number.

Every release publishes two assets: the versioned DMG, and a copy named
`Encore-macOS-arm64.dmg`. The landing page links to the second through
`releases/latest/download/…`, which only resolves for an exact asset name —
that copy is what keeps the download button working without redeploying the
site for each release.

## Signing — the one thing standing between this and "seamless"

Builds are currently **ad-hoc signed**, and that has two consequences:

- Gatekeeper refuses to open the app normally. Users must right-click →
  Open once. The landing page explains this.
- **Screen Recording permission is bound to the code signature.** An ad-hoc
  signature changes on every build, so macOS treats each update as a
  different app and asks for screen access again. For an always-on capture
  tool this is the more damaging half.

Both disappear with a Developer ID. It needs an Apple Developer account
($99/yr) and six repository secrets:

| Secret | What it is |
|---|---|
| `APPLE_CERTIFICATE` | Developer ID Application cert, exported as `.p12`, base64 encoded |
| `APPLE_CERTIFICATE_PASSWORD` | Password used when exporting that `.p12` |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Your Name (TEAMID)` |
| `APPLE_ID` | Apple ID email, for notarization |
| `APPLE_PASSWORD` | App-specific password, not the account password |
| `APPLE_TEAM_ID` | Ten-character team identifier |

The release workflow already reads all six. With them present it signs and
notarizes; with them absent it unsets them and builds unsigned, warning in
the job summary. Nothing needs changing but adding the secrets.

## Hosting encore.gisketch.com

The setup steps — Cloudflare, nginx, and the deploy secrets — live in
[DEPLOYMENT_GUIDE.md](../DEPLOYMENT_GUIDE.md) so there is one copy of them.

The page is a single self-contained file, so deployment is a copy. The Site
workflow uploads it beside the live file and moves it into place, so an
interrupted transfer never leaves half a page being served. The download
button points at GitHub's release CDN rather than the VPS, so the site stays
one small file and releases never touch the server.

## Known gaps

- **Apple Silicon only.** The sidecar script refuses cross-target builds, so
  an Intel build needs its own runner, and no Intel build has been tested.
- **No LICENSE file, and the bundled ffmpeg is GPL-3.0-or-later.**
  Distributing the DMG carries GPL obligations that the repository does not
  currently state. Worth settling before promoting the download widely.
