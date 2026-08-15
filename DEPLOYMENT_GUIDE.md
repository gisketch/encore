# Deployment Guide

How to put Encore online. Three setup jobs, then it runs itself.

Reference detail (signing secrets, runner choice, known gaps) lives in
[docs/releasing.md](docs/releasing.md). This file is the steps.

## What already works

- The Mac app builds in GitHub Actions.
- The download link is live:
  `https://github.com/gisketch/encore/releases/latest/download/Encore-macOS-arm64.dmg`
- The landing page is one file: `landing-page.html`.
- Three workflows run: **CI**, **Release**, **Site**.

You do not need a server to distribute the app. GitHub hosts the download.
The server is only for the landing page.

---

## Job 1 — Cloudflare

⚠️ Set the SSL mode before you send traffic. "Flexible" mode serves your
users HTTPS but talks plain HTTP to your VPS.

1. Open **DNS** and add a record.
2. Set type `A`, name `encore`, content = your VPS IP address.
3. Set the proxy status to **Proxied**. The cloud icon turns orange.
4. Open **SSL/TLS** and set the mode to **Full (strict)**.

Proxied mode gives you the CDN, DDoS protection, and hides the VPS IP.

---

## Job 2 — VPS

Create the directory that nginx serves:

```bash
sudo mkdir -p /var/www/encore
sudo chown $USER:$USER /var/www/encore
```

Add this nginx server block:

```nginx
server {
    server_name encore.gisketch.com;
    root /var/www/encore;
    index index.html;
    location / { try_files $uri $uri/ =404; }
}
```

Get the certificate. **Full (strict)** in Cloudflare requires one:

```bash
sudo certbot --nginx -d encore.gisketch.com
```

---

## Job 3 — Automatic deploys

Add four **secrets** in GitHub → Settings → Secrets and variables → Actions:

| Secret | Value |
|---|---|
| `VPS_SSH_KEY` | Private half of a deploy key the VPS accepts |
| `VPS_HOST` | Server hostname or IP address |
| `VPS_USER` | SSH user |
| `VPS_SITE_PATH` | `/var/www/encore` |

Add `VPS_PORT` only when SSH does not use port 22.

Then add one **variable** (a variable, not a secret):

| Variable | Value |
|---|---|
| `SITE_ENABLED` | `true` |

The Site workflow skips while `SITE_ENABLED` is unset. This keeps every
push green before the secrets exist.

Every push that changes `landing-page.html` now deploys it. The workflow
uploads the page beside the live file, then moves it into place. An
interrupted upload cannot leave half a page online.

---

## Ship a new app version

⚠️ The tag must match the version in `src-tauri/tauri.conf.json`. The
workflow stops the build when they differ, so a DMG never ships with the
wrong number on it.

1. Change `version` in `src-tauri/tauri.conf.json`.
2. Commit the change.
3. Tag it and push the tag:

```bash
git tag v0.2.0 && git push origin v0.2.0
```

The workflow builds the app and attaches the DMG to a GitHub release. It
publishes two files: the versioned DMG, and a copy named
`Encore-macOS-arm64.dmg`. The landing page links to the second name, so the
download button keeps working without a site deploy.

---

## Two things to remember

**Cloudflare caches HTML.** Purge the cache after a deploy, or add a Cache
Rule with a short TTL for `encore.gisketch.com/`. Without this you see the
old page and think the deploy failed.

**The app is not signed.** Apple code signing costs $99 per year and this
project does not use it. Two effects follow:

- macOS refuses to open the app on the first launch. The user must
  right-click the app and choose **Open**. The landing page explains this.
- macOS asks for Screen Recording permission again after each update. The
  permission is tied to the code signature, and an unsigned signature
  changes on every build.

Nothing here is broken. This is the normal behavior for an unsigned app.

---

## Still open

- **Apple Silicon only.** The sidecar script refuses cross-target builds. An
  Intel build needs its own runner, and nobody has tested one.
- **No LICENSE file.** The bundled ffmpeg is GPL-3.0-or-later. Distributing
  the DMG carries GPL obligations that this repository does not yet state.
  Settle this before you promote the download widely.
