//! AYR Audio Link — sender binary.
//!
//! The product: a small, single-window Windows app that captures one
//! chosen audio device (input or loopback of an output) and streams it
//! over the LAN to an AYR Audio Meter running on another PC, where it
//! appears as a normal entry in the device picker.
//!
//! Layout mirrors the meter:
//!
//! * `main.rs`       — eframe bootstrapping, logging, panic hook
//! * `app.rs`        — the single egui window (device picker, level bar,
//!                     peer list, pairing dialog, start/stop toggle)
//! * `audio.rs`      — capture: cpal input + WASAPI loopback on Windows
//! * `net.rs`        — TCP server that accepts meter connections and
//!                     pushes PCM frames
//! * `identity.rs`   — persistent sender identity + trust cache
//!                     (`~/.config/ayr-audio-link/state.json`)
//!
//! Keeping the whole thing in one binary (no daemons, no services) is a
//! deliberate UX choice: the user can see at a glance whether it's
//! running and which meter is attached.

// No console window when double-clicking or starting from Explorer (debug + release).
#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod audio;
mod identity;
mod net;
mod theme;
mod updater;

use eframe::egui::{IconData, ViewportBuilder};

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Log — don't paper over — a panic on the UI thread. Without this hook a
    // panic inside an `egui` update closure turns into a silent taskbar
    // disappearance; the `env_logger` record at least lands in a terminal
    // when the user is debugging.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("ayr-audio-link panic: {info}");
        default_hook(info);
    }));

    let icon = load_icon();
    // Fixed inner width; height is driven from the UI (see `LinkApp::ui`) so tall content does not clip.
    let w = app::LINK_VIEWPORT_INNER_WIDTH_PX;
    let viewport = ViewportBuilder::default()
        .with_inner_size([w, 640.0])
        .with_min_inner_size([w, 520.0])
        .with_max_inner_size([w, 1200.0])
        .with_resizable(false)
        .with_maximize_button(false)
        .with_title("AYR Audio Link")
        .with_icon(icon);

    let native_options = eframe::NativeOptions {
        viewport,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "AYR Audio Link",
        native_options,
        Box::new(|cc| Ok(Box::new(app::LinkApp::new(cc)))),
    )
}

fn load_icon() -> IconData {
    const BYTES: &[u8] =
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/app-icon.ico"));
    const SZ: u32 = 32;
    let Ok(img) = image::load_from_memory(BYTES) else {
        return IconData {
            rgba: vec![0u8; 4],
            width: 1,
            height: 1,
        };
    };
    let rgba = img.to_rgba8();
    let small = image::imageops::resize(
        &rgba,
        SZ,
        SZ,
        image::imageops::FilterType::Lanczos3,
    );
    IconData {
        rgba: small.into_raw(),
        width: SZ,
        height: SZ,
    }
}
