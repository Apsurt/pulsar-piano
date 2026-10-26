# Pulsar

A piano app: MIDI keyboard in, sound out. Tauri 2 shell, Rust engine, Svelte 5 UI.

## Architecture rule: audio never touches the webview

```
MIDI device ──midir──► lock-free queue ──► audio thread (cpal) ──► speakers
UI (webview) ◄── throttled note events ──┘
UI ──commands──► engine control (instrument, volume, …)
```

```
crates/engine/   MIDI, audio, instruments. Pure Rust, no Tauri dependency
crates/song/     Song model, MIDI import, song library (JSON in app data)
src-tauri/       Tauri commands/events, thin shell around engine and songs
ui/              Svelte 5 + Vite + TS frontend
```

## Develop

Needs Rust (stable) and Node (LTS).

```bash
npm install
npm run dev        # runs "Pulsar (Dev)", id com.apsurt.pulsar.dev
```

Dev and release builds use different bundle identifiers, so they have separate app data
and settings and can be installed side by side. User content (sounds, settings) always lives
in `app_data_dir()`, never next to the binary.

Other scripts: `npm run check` (svelte-check), `npm run build:dev` (installable dev build).
`npm run build` produces a signed release build and needs the signing key env vars below.

## Release

```bash
npm version patch  # or minor / major: bumps package.json, commits, tags vX.Y.Z
git push --follow-tags
```

The `release` workflow builds Windows (NSIS, per-user), macOS (arm64 + x64) and Linux
(AppImage + deb) into a draft GitHub Release, then publishes it once every build has passed.
Installed apps poll `releases/latest/download/latest.json` for updates.

The app version comes from `package.json` only. The tag must match it, and CI checks that.

### Updater signing key

Updates are signed. The private key is `~/.tauri/pulsar.key` (password in
`~/.tauri/pulsar.key.password`). Both are stored as repo secrets `TAURI_SIGNING_PRIVATE_KEY` and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. **Back the key up.** If it's lost, installed copies can
never auto-update again. For a local `npm run build`, export both as env vars.

Builds are not code-signed, so Windows SmartScreen and macOS Gatekeeper will warn on first
install (on macOS: `xattr -cr /Applications/Pulsar.app`).
