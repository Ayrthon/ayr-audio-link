# AYR Audio Link

A free Windows companion app to [AYR Audio Meter](https://ayrthon.com). AYR Audio Link captures one input device or WASAPI loopback on PC A and streams uncompressed PCM over the LAN to the Meter running on PC B, where it appears as an ordinary entry in the device dropdown tagged `[NET]`.

Typical use: the Meter lives on your main workstation; Link runs on a second PC (a DAW rig, a vocal-booth laptop, a second studio machine) and lets you see that signal on the main monitor without cables.

- User-facing how-to: [docs/README.md](docs/README.md)
- Release workflow: [docs/release-checklist.md](docs/release-checklist.md)

## Repo layout

```
ayr-audio-link/
├── Cargo.toml                     hybrid root package + workspace
├── src/                           the sender binary (ayr-audio-link)
├── crates/
│   └── ayr-audio-link-core/       shared wire format + mDNS (library)
├── installer/                     Inno Setup project + build script
└── docs/                          user docs + release checklist
```

The repo is intentionally shaped to mirror the Meter's layout. That makes it a reusable template for future AYR Audio companion apps — fork this repo, rename the package, swap `src/` and `installer/ayr-audio-link.iss`, and you have a working shell for the next product.

## Build

Requirements: Rust stable (MSVC toolchain on Windows), Inno Setup 6 for the installer.

```powershell
# Build the sender
cargo build --release

# Build the signed installer exe (lands in .\dist\)
powershell -ExecutionPolicy Bypass -File .\installer\build-installer.ps1
```

## Meter integration

The Meter (a separate repo at `Ayrthon/audio-meter`) depends on `ayr-audio-link-core` via a path dep pointing at this repo as a sibling checkout:

```
../ayr-audio-link/crates/ayr-audio-link-core
```

So for local development, clone both repos side-by-side:

```
~/code/
├── audio-meter/        (private, Meter source)
└── ayr-audio-link/     (this repo)
```

Once both repos are pushed to GitHub, the path dep in the Meter's `Cargo.toml` is expected to flip to a git dep so the Meter can be built in CI without a Link checkout. That's marked with a `TODO` comment in the Meter's `Cargo.toml`.

## License

(Not yet decided — the Link app is delivered as a free download but the source is private while still stabilising. See the top-level `TODO` for the license-text decision.)
