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
├── launch-*.ps1 / launch-*.sh     Windows build + staged launch (from WSL or PowerShell)
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

## Run (WSL + Windows binary — same pattern as AYR Audio Meter)

The shippable app is the **Windows** `.exe`. If you edit from WSL, do **not** rely on a bare Linux `cargo run` for day-to-day UI work; use these scripts so Cargo runs on the Windows side, artifacts land on NTFS, and the staged copy avoids SmartScreen noise on `\\wsl$\` paths.

| Goal | Windows (repo root) | WSL (repo root) |
|------|----------------------|-----------------|
| Fast debug iteration | `powershell -ExecutionPolicy Bypass -File .\launch-dev.ps1` | `./launch-dev.sh` |
| Release binary + launch | `powershell -ExecutionPolicy Bypass -File .\launch-release.ps1` | `./launch-release.sh` |
| Watch + rebuild + restart | `powershell -ExecutionPolicy Bypass -File .\launch-dev-watch.ps1` (needs `cargo install cargo-watch`) | `./launch-dev-watch.sh` |

Staging directory: `%LOCALAPPDATA%\AyrAudioLink\` (debug object files use `%LOCALAPPDATA%\AyrAudioLink\cargo-target-dev\` so incremental builds stay off the UNC-mounted tree).

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
