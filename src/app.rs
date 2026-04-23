//! AYR Audio Link sender UI.
//!
//! Tiny single-window layout built directly on `egui` (no splash, no
//! license gate, no multi-tab — this is a one-purpose utility):
//!
//! ```text
//! ┌───────────────────────────────────────────────┐
//! │  AYR Audio Link                               │
//! │  Broadcasting as: MikeStudio-PC (Input 1/2)   │
//! │                                               │
//! │  Source: [ dropdown ]           [ refresh ]   │
//! │  ████████████████████░░░░░░░░░  -12.4 dBFS   │
//! │                                               │
//! │  Status: Listening on :45451                  │
//! │  Connected meters:                            │
//! │    * 192.168.1.21:54122    12 345 frames      │
//! │                                               │
//! │  [ Start broadcasting ] [ Stop ]              │
//! └───────────────────────────────────────────────┘
//! ```
//!
//! Keeping the layout this linear makes the "is it working?" answer one
//! glance — if the level bar moves and the peer list has at least one
//! entry, everything is fine.

use std::sync::Arc;
use std::sync::mpsc;

use eframe::egui::{self, Color32, CornerRadius, Direction, Frame, RichText, Shape, Stroke, StrokeKind};
use parking_lot::Mutex;

use ayr_audio_link_core::{DEFAULT_PORT, discovery, wire::Handshake};

use crate::audio::{self, Capture, DeviceInfo, SourceKind};
use crate::identity::IdentityStore;
use crate::net::{Server, ServerStatus};
use crate::theme;
use crate::updater;

pub struct LinkApp {
    identity: Arc<IdentityStore>,
    devices: Vec<DeviceInfo>,
    selected_device: Option<usize>,
    capture: Option<Capture>,
    /// When set, a background thread is joining the capture thread after Stop.
    capture_shutdown_rx: Option<mpsc::Receiver<()>>,
    server: Server,
    advertiser: Option<discovery::Advertiser>,

    /// Cheap running peak (dBFS) for the UI level bar, updated from the
    /// audio callback. Not meter-grade — this is just a heartbeat for the
    /// user so they know audio is flowing.
    level_shared: Arc<Mutex<LiveLevel>>,

    /// Set once the capture produces its first block; controls whether
    /// we've registered the mDNS advertisement yet.
    announced_format: Option<Handshake>,

    /// Transient error surfaced at the bottom of the window. Cleared when
    /// the user picks a different device or clicks "Retry".
    last_error: Option<String>,

    port: u16,
    display_name_editor: String,
    show_advanced: bool,
    /// When `true`, pressing the window close button minimizes the app
    /// instead of quitting. This is a poor-man's tray-minimize — a real
    /// `tray-icon` integration is a Phase 2 item per the product plan.
    minimize_on_close: bool,

    /// Auto-updater status, populated by `updater::spawn_check` on boot
    /// and read each frame to show a small "update available" banner.
    /// Kept intentionally unobtrusive — Link is a background utility, so
    /// the update flow is a one-click banner rather than a modal.
    update: updater::SharedUpdateStatus,
    update_dismissed: bool,

    /// Brand artwork (same mockup as marketing); shown in the header.
    brand_texture: Option<egui::TextureHandle>,
}

#[derive(Clone, Copy, Default)]
struct LiveLevel {
    peak_db: f32,
    active: bool,
}

impl LinkApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let brand_texture = theme::load_brand_texture(&cc.egui_ctx);
        let identity = Arc::new(IdentityStore::load_or_init());
        let devices = audio::list_devices().unwrap_or_default();
        let selected_device = pick_default_device(&devices);
        let server = Server::new(identity.clone());
        let display_name_editor = identity.display_name();
        Self {
            identity,
            devices,
            selected_device,
            capture: None,
            capture_shutdown_rx: None,
            server,
            advertiser: None,
            level_shared: Arc::new(Mutex::new(LiveLevel::default())),
            announced_format: None,
            last_error: None,
            port: DEFAULT_PORT,
            display_name_editor,
            show_advanced: false,
            minimize_on_close: true,
            update: {
                let shared = updater::make_shared_status();
                updater::spawn_check(
                    updater::UPDATE_REPO,
                    env!("CARGO_PKG_VERSION"),
                    shared.clone(),
                );
                shared
            },
            update_dismissed: false,
            brand_texture,
        }
    }

    fn capture_alive(&self) -> bool {
        self.capture.is_some()
    }

    fn stopping_capture(&self) -> bool {
        self.capture_shutdown_rx.is_some()
    }

    /// True while broadcasting or while capture is still shutting down (UI should stay busy).
    fn broadcast_session_busy(&self) -> bool {
        self.capture_alive() || self.stopping_capture()
    }

    fn selected(&self) -> Option<&DeviceInfo> {
        self.selected_device.and_then(|i| self.devices.get(i))
    }

    fn start_broadcasting(&mut self) {
        if self.stopping_capture() {
            self.last_error = Some(
                "Still shutting down the previous capture — try again in a moment.".into(),
            );
            return;
        }
        self.last_error = None;
        let Some(dev) = self.selected().cloned() else {
            self.last_error = Some("Pick an audio device first.".into());
            return;
        };
        if let Err(e) = self.server.start(self.port) {
            self.last_error = Some(format!("Couldn't bind port {}: {e}", self.port));
            return;
        }

        let server_handle = self.server.handle();
        let level_shared = self.level_shared.clone();
        let capture_result = audio::start_capture(dev.clone(), move |samples: &[f32], ch: u8, sr: u32| {
            // Forward to the TCP server first, then take the short UI lock for
            // the level bar — avoids holding `level_shared` while contending on
            // server mutexes during Stop/shutdown.
            let mut bytes = Vec::with_capacity(samples.len() * 4);
            for &s in samples {
                bytes.extend_from_slice(&s.to_le_bytes());
            }
            server_handle.update_format_if_changed(ch, sr);
            server_handle.submit(bytes);

            let peak = samples
                .iter()
                .fold(0.0f32, |acc, &s| acc.max(s.abs()));
            let db = if peak > 0.0 {
                20.0 * peak.log10()
            } else {
                -120.0
            };
            *level_shared.lock() = LiveLevel {
                peak_db: db.max(-120.0),
                active: true,
            };
        });

        match capture_result {
            Ok(cap) => self.capture = Some(cap),
            Err(e) => {
                self.last_error = Some(format!("Couldn't start capture: {e:#}"));
                self.server.stop();
            }
        }
    }

    fn stop_broadcasting(&mut self) {
        // 1) Ask capture to stop so the audio thread stops touching the graph
        //    before we tear down TCP (still may run a few more callbacks).
        if let Some(c) = self.capture.as_ref() {
            c.signal_stop();
        }
        // 2) Drop peers / listener so meter stalls cannot wedge `submit` or
        //    peer `flush` against a half-dead session.
        self.server.stop();
        // 3) Join capture off the UI thread — `drop(stream)` / driver teardown
        //    can block for a long time; the meter closing the socket should not
        //    be required for a responsive Stop button.
        if let Some(c) = self.capture.take() {
            let (tx, rx) = mpsc::channel();
            match std::thread::Builder::new()
                .name("ayr-link-capture-shutdown".into())
                .spawn(move || {
                    let mut c = c;
                    c.stop();
                    let _ = tx.send(());
                }) {
                Ok(_) => self.capture_shutdown_rx = Some(rx),
                Err(e) => {
                    // `c` was moved into the closure value; dropping it on this
                    // thread runs `Capture::drop` → `stop()` (rare spawn failure).
                    log::error!("capture shutdown thread spawn failed: {e}");
                }
            }
        }
        self.advertiser = None;
        self.announced_format = None;
        *self.level_shared.lock() = LiveLevel::default();
    }

    fn poll_capture_shutdown_done(&mut self) {
        let Some(rx) = &self.capture_shutdown_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(()) | Err(mpsc::TryRecvError::Disconnected) => {
                self.capture_shutdown_rx = None;
                self.server.reset_handshake();
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }

    /// Render a small "update available" banner if the updater resolved
    /// to `Available` / `Downloading` / `Downloaded` / `LaunchFailed`.
    /// Other states (including network errors) are deliberately silent —
    /// Link is a background tool and a nagging offline-state banner would
    /// be disproportionate to the cost.
    fn maybe_draw_update_banner(&mut self, ui: &mut egui::Ui) {
        use updater::UpdateStatus;

        if self.update_dismissed {
            return;
        }
        // Take a snapshot so we can release the lock before mutating
        // `self`. The updater thread is still free to overwrite the
        // slot we cloned — the UI just re-reads next frame.
        let status = self.update.lock().clone();
        match status {
            UpdateStatus::Available(info) => {
                let banner = Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(12, 42, 48, 240))
                    .corner_radius(CornerRadius::same(10))
                    .stroke(Stroke::NONE)
                    .inner_margin(egui::Margin::symmetric(12, 8));
                banner.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "Update available — v{} (you have v{})",
                                info.version,
                                env!("CARGO_PKG_VERSION"),
                            ))
                            .color(Color32::from_rgb(200, 255, 255))
                            .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if theme::gradient_primary_button(ui, "Update now", 120.0, true).clicked() {
                                updater::spawn_download_and_run(info.clone(), self.update.clone());
                            }
                            if ui.button("Later").clicked() {
                                self.update_dismissed = true;
                            }
                        });
                    });
                });
                ui.add_space(theme::GAP_MD);
            }
            UpdateStatus::Downloading {
                progress,
                total_bytes,
            } => {
                let mb = total_bytes as f32 / (1024.0 * 1024.0);
                Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(14, 28, 40, 240))
                    .corner_radius(CornerRadius::same(10))
                    .stroke(Stroke::NONE)
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Downloading update…")
                                    .color(theme::MUTED)
                                    .strong(),
                            );
                            ui.add(
                                egui::ProgressBar::new(progress)
                                    .desired_width(180.0)
                                    .fill(Color32::from_rgba_unmultiplied(0, 220, 255, 180))
                                    .show_percentage(),
                            );
                            if total_bytes > 0 {
                                ui.label(RichText::new(format!("{mb:.1} MB")).color(theme::MUTED));
                            }
                        });
                    });
                ui.add_space(theme::GAP_MD);
            }
            UpdateStatus::Downloaded(path) => {
                // We don't auto-run the installer for Link — the user is
                // usually mid-session sending audio, and swapping the
                // .exe while a meter is connected would drop the peer.
                // Give them an explicit "Install now" button.
                Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(14, 36, 40, 240))
                    .corner_radius(CornerRadius::same(10))
                    .stroke(Stroke::NONE)
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Update downloaded.")
                                    .color(Color32::from_rgb(200, 255, 255))
                                    .strong(),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if theme::gradient_primary_button(
                                        ui,
                                        "Install now",
                                        124.0,
                                        true,
                                    )
                                    .clicked()
                                    {
                                        // Stop the broadcast before handing off to the
                                        // installer so the Restart Manager doesn't have
                                        // to grapple with a busy TCP port.
                                        self.stop_broadcasting();
                                        match updater::launch_installer(&path) {
                                            Ok(()) => {
                                                *self.update.lock() = UpdateStatus::Launching;
                                                ui.ctx().send_viewport_cmd(
                                                    egui::ViewportCommand::Close,
                                                );
                                            }
                                            Err(e) => {
                                                *self.update.lock() =
                                                    UpdateStatus::LaunchFailed(e.to_string());
                                            }
                                        }
                                    }
                                    if ui.button("Later").clicked() {
                                        self.update_dismissed = true;
                                    }
                                },
                            );
                        });
                    });
                ui.add_space(theme::GAP_MD);
            }
            UpdateStatus::LaunchFailed(msg) => {
                Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(40, 20, 16, 245))
                    .corner_radius(CornerRadius::same(10))
                    .stroke(Stroke::NONE)
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(
                                Color32::from_rgb(255, 200, 160),
                                format!("Update failed: {msg}"),
                            );
                            if ui.button("Dismiss").clicked() {
                                self.update_dismissed = true;
                            }
                        });
                    });
                ui.add_space(theme::GAP_MD);
            }
            _ => {}
        }
    }

    /// Refresh the mDNS advertisement. Called every frame; cheap when the
    /// format hasn't changed.
    fn reconcile_mdns(&mut self) {
        if self.stopping_capture() {
            return;
        }
        let want_format = self.server.current_handshake();
        match (&want_format, &self.announced_format) {
            (Some(new), prev) if Some(new) != prev.as_ref() => {
                // Tear down the old advert; build a fresh one.
                self.advertiser = None;
                match discovery::Advertiser::new(
                    &self.identity.instance_name(),
                    self.port,
                    &new.display_name,
                    &crate::identity::local_peer_fingerprint(),
                    new.sample_rate,
                    new.channels,
                ) {
                    Ok(a) => self.advertiser = Some(a),
                    Err(e) => {
                        log::warn!("mdns advertise: {e}");
                        self.last_error = Some(format!("mDNS announce failed: {e}"));
                    }
                }
                self.announced_format = Some(new.clone());
            }
            (None, Some(_)) => {
                // Capture stopped — drop advert.
                self.advertiser = None;
                self.announced_format = None;
            }
            _ => {}
        }
    }
}

impl eframe::App for LinkApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Poll at ~30Hz so the level bar animates and peer stats refresh.
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(33));

        self.poll_capture_shutdown_done();

        // Poor-man's "tray minimize" — catch the close event and turn it
        // into a minimise instead. Real tray icon (so users can right-click
        // a notification-area glyph to quit) is Phase 2.
        if self.minimize_on_close
            && ui.ctx().input(|i| i.viewport().close_requested())
            && self.broadcast_session_busy()
        {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }

        self.reconcile_mdns();

        egui::CentralPanel::default()
            .frame(Frame::NONE.inner_margin(egui::Margin::same(22)))
            .show_inside(ui, |ui| {
                let panel_rect = ui.max_rect();
                theme::paint_background(&ui.painter(), panel_rect);

                draw_app_header(ui, self.brand_texture.as_ref());

                ui.add_space(theme::GAP_MD);
                self.maybe_draw_update_banner(ui);

                theme::glass_frame().show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = theme::GAP_SM;
                    ui.label(
                        RichText::new("IDENTITY")
                            .size(10.0)
                            .strong()
                            .color(theme::CYAN),
                    );
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = theme::GAP_SM;
                        ui.label(RichText::new("Broadcasting as").color(theme::MUTED));
                        ui.label(
                            RichText::new(&self.display_name_editor)
                                .monospace()
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Edit").clicked() {
                                self.show_advanced = !self.show_advanced;
                            }
                        });
                    });

                    if self.show_advanced {
                        ui.add_space(theme::GAP_XS);
                        ui.separator();
                        ui.add_space(theme::GAP_SM);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Display name")
                                    .color(theme::MUTED)
                                    .size(12.0),
                            );
                            let r = ui.text_edit_singleline(&mut self.display_name_editor);
                            if r.lost_focus() {
                                self.identity
                                    .set_display_name(Some(self.display_name_editor.clone()));
                                self.server
                                    .set_display_name(self.display_name_editor.clone());
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("TCP port")
                                    .color(theme::MUTED)
                                    .size(12.0),
                            );
                            let mut port_txt = self.port.to_string();
                            if ui.text_edit_singleline(&mut port_txt).lost_focus() {
                                if let Ok(p) = port_txt.parse::<u16>() {
                                    self.port = p;
                                }
                            }
                        });
                        ui.checkbox(
                            &mut self.minimize_on_close,
                            "Minimize on close (stay in taskbar)",
                        );
                    }
                });

                theme::glass_frame().show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = theme::GAP_SM;
                    ui.label(
                        RichText::new("AUDIO SOURCE")
                            .size(10.0)
                            .strong()
                            .color(theme::CYAN),
                    );

                    let current_sel = self.selected_device;
                    let devices_snapshot: Vec<(SourceKind, String)> = self
                        .devices
                        .iter()
                        .map(|d| (d.kind, d.name.clone()))
                        .collect();
                    let mut pending_select: Option<usize> = None;
                    let mut pending_refresh = false;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = theme::GAP_SM;
                        let reserve_refresh = 76.0;
                        let combo_w = (ui.available_width()
                            - reserve_refresh
                            - theme::GAP_SM)
                            .max(160.0);
                        let selected_label = current_sel
                            .and_then(|i| self.devices.get(i))
                            .map(device_label)
                            .unwrap_or_else(|| "(none)".into());
                        egui::ComboBox::from_id_salt("device_combo")
                            .popup_style(theme::combo_popup_style())
                            .selected_text(RichText::new(selected_label).color(Color32::WHITE))
                            .width(combo_w)
                            .show_ui(ui, |ui| {
                                let mut section =
                                    |ui: &mut egui::Ui, title: &str, kind: SourceKind| {
                                        ui.label(
                                            RichText::new(title)
                                                .size(11.0)
                                                .strong()
                                                .color(theme::CYAN),
                                        );
                                        for (i, (k, name)) in devices_snapshot.iter().enumerate()
                                        {
                                            if *k != kind {
                                                continue;
                                            }
                                            let sel = Some(i) == current_sel;
                                            if ui.selectable_label(sel, name).clicked() {
                                                pending_select = Some(i);
                                            }
                                        }
                                    };
                                section(ui, "INPUTS (mic / line)", SourceKind::Input);
                                ui.separator();
                                section(
                                    ui,
                                    "LOOPBACK (what speakers play)",
                                    SourceKind::Loopback,
                                );
                            });
                        if ui.button("Refresh").clicked() {
                            pending_refresh = true;
                        }
                    });
                    if let Some(i) = pending_select {
                        self.selected_device = Some(i);
                    }
                    if pending_refresh {
                        let previous = self.selected().map(|d| (d.kind, d.name.clone()));
                        self.devices = audio::list_devices().unwrap_or_default();
                        self.selected_device = previous
                            .and_then(|prev| {
                                self.devices.iter().position(|d| {
                                    d.kind == prev.0 && d.name == prev.1
                                })
                            })
                            .or_else(|| pick_default_device(&self.devices));
                    }

                    ui.add_space(theme::GAP_XS);
                    draw_level_bar(ui, *self.level_shared.lock());
                });

                theme::glass_frame().show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = theme::GAP_SM;
                    let status = self.server.status();
                    draw_status_block(
                        ui,
                        &status,
                        self.capture_alive(),
                        self.stopping_capture(),
                        self.advertiser.is_some(),
                    );
                });

                ui.add_space(theme::GAP_MD);
                let full_btn_w = ui.available_width();
                if self.capture_alive() {
                    if theme::gradient_stop_button(ui, "Stop broadcasting", full_btn_w).clicked() {
                        self.stop_broadcasting();
                    }
                } else if self.stopping_capture() {
                    ui.add_enabled_ui(false, |ui| {
                        theme::gradient_stop_button(ui, "Stopping broadcast…", full_btn_w);
                    });
                } else if theme::gradient_primary_button(
                    ui,
                    "Start broadcasting",
                    full_btn_w,
                    self.selected_device.is_some(),
                )
                .clicked()
                {
                    self.start_broadcasting();
                }

                if let Some(err) = &self.last_error {
                    ui.add_space(theme::GAP_SM);
                    Frame::NONE
                        .fill(Color32::from_rgba_unmultiplied(48, 14, 18, 245))
                        .corner_radius(CornerRadius::same(10))
                        .stroke(Stroke::NONE)
                        .inner_margin(egui::Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            ui.colored_label(Color32::from_rgb(255, 170, 170), err);
                        });
                }
            });
    }
}

fn draw_app_header(ui: &mut egui::Ui, brand_texture: Option<&egui::TextureHandle>) {
    let header_bg = Color32::from_rgba_unmultiplied(12, 17, 28, 252);
    Frame::NONE
        .fill(header_bg)
        .corner_radius(CornerRadius::same(10))
        .stroke(Stroke::NONE)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(12.0, 0.0);
                let accent_h = 44.0;
                let (accent_rect, _) = ui.allocate_exact_size(
                    egui::vec2(3.0, accent_h),
                    egui::Sense::hover(),
                );
                if ui.is_rect_visible(accent_rect) {
                    ui.painter_at(accent_rect).add(Shape::gradient_rect(
                        accent_rect,
                        Direction::TopDown,
                        [theme::CYAN, theme::COBALT],
                    ));
                }
                if let Some(tex) = brand_texture {
                    ui.add(
                        egui::Image::from_texture(tex)
                            .max_size(egui::vec2(44.0, 44.0))
                            .corner_radius(CornerRadius::same(8)),
                    );
                }
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.label(
                            RichText::new("AYR ")
                                .size(19.0)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.label(
                            RichText::new("Audio ")
                                .size(19.0)
                                .strong()
                                .color(theme::MUTED),
                        );
                        ui.label(
                            RichText::new("Link")
                                .size(20.0)
                                .strong()
                                .color(theme::CYAN),
                        );
                    });
                    ui.label(
                        RichText::new("Send audio from this PC to AYR Audio Meter on the LAN.")
                            .size(11.0)
                            .color(Color32::from_rgba_unmultiplied(130, 170, 200, 230)),
                    );
                });
            });
        });
}

fn pick_default_device(devices: &[DeviceInfo]) -> Option<usize> {
    // Prefer the system default input. If we're on a headless Linux and
    // there isn't one, fall back to index 0.
    if let Some(default_in) = audio::default_input_name() {
        if let Some(i) = devices
            .iter()
            .position(|d| d.kind == SourceKind::Input && d.name == default_in)
        {
            return Some(i);
        }
    }
    if !devices.is_empty() {
        Some(0)
    } else {
        None
    }
}

fn device_label(d: &DeviceInfo) -> String {
    let tag = match d.kind {
        SourceKind::Input => "[IN]",
        SourceKind::Loopback => "[OUT]",
    };
    format!("{tag} {}", d.name)
}

fn draw_level_bar(ui: &mut egui::Ui, level: LiveLevel) {
    let db_txt = if level.active {
        format!("{:.1} dBFS", level.peak_db)
    } else {
        "-- dBFS".into()
    };
    let label_color = if level.active {
        Color32::WHITE.lerp_to_gamma(theme::CYAN, 0.35)
    } else {
        theme::MUTED
    };
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Signal (mono peak)")
                .size(10.0)
                .color(Color32::from_rgba_unmultiplied(100, 140, 170, 200)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(db_txt)
                    .size(11.0)
                    .monospace()
                    .color(label_color),
            );
        });
    });
    ui.add_space(theme::GAP_SM);
    let full_width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(full_width, 18.0),
        egui::Sense::hover(),
    );
    let painter = ui.painter();
    let track_radius = CornerRadius::same(5);
    painter.rect_filled(
        rect,
        track_radius,
        Color32::from_rgba_unmultiplied(6, 10, 18, 255),
    );
    painter.rect_stroke(
        rect,
        track_radius,
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 160, 220, 70)),
        StrokeKind::Inside,
    );
    // Map peak_db in [-60..0] to bar fraction [0..1].
    let f = ((level.peak_db + 60.0) / 60.0).clamp(0.0, 1.0);
    let filled = egui::Rect::from_min_max(
        rect.min + egui::vec2(1.0, 1.0),
        egui::pos2(rect.min.x + 1.0 + (rect.width() - 2.0) * f, rect.max.y - 1.0),
    );
    if f > 0.001 {
        painter.add(Shape::gradient_rect(
            filled,
            Direction::LeftToRight,
            [theme::CYAN, theme::COBALT],
        ));
        let warn = if level.peak_db > -3.0 {
            Some(Color32::from_rgba_unmultiplied(255, 60, 60, 110))
        } else if level.peak_db > -18.0 {
            Some(Color32::from_rgba_unmultiplied(255, 200, 60, 90))
        } else {
            None
        };
        if let Some(tint) = warn {
            painter.rect_filled(filled, CornerRadius::same(4), tint);
        }
    }
}

fn draw_status_block(
    ui: &mut egui::Ui,
    status: &ServerStatus,
    capture_alive: bool,
    stopping_capture: bool,
    mdns_advertising: bool,
) {
    ui.label(
        RichText::new("NETWORK")
            .size(10.0)
            .strong()
            .color(theme::CYAN),
    );
    ui.add_space(theme::GAP_XS);
    let headline = if stopping_capture {
        "Stopping broadcast…".to_string()
    } else {
        match (capture_alive, status.listening_on) {
            (true, Some(addr)) => format!("Listening on {addr}"),
            (true, None) => "Listening (no local address resolved)".to_string(),
            (false, _) => "Not broadcasting".to_string(),
        }
    };
    ui.label(
        RichText::new(headline)
            .strong()
            .size(14.0)
            .color(Color32::WHITE),
    );
    if capture_alive {
        ui.add_space(theme::GAP_XS);
        let mdns_txt = if mdns_advertising {
            "mDNS: visible to AYR Audio Meter on this LAN"
        } else {
            "mDNS: waiting for first audio block before announcing"
        };
        ui.label(RichText::new(mdns_txt).size(12.0).color(theme::MUTED));
    }

    ui.add_space(theme::GAP_MD);
    ui.label(
        RichText::new(format!("Connected meters: {}", status.peers.len()))
            .strong()
            .size(12.0)
            .color(Color32::from_rgb(230, 245, 255)),
    );
    ui.add_space(theme::GAP_XS);
    if status.peers.is_empty() {
        ui.label(
            RichText::new("(none — choose this stream in the meter’s device list)")
                .size(12.0)
                .color(theme::MUTED),
        );
    } else {
        ui.spacing_mut().item_spacing.y = theme::GAP_XS;
        ui.vertical(|ui| {
            for p in &status.peers {
                ui.label(
                    RichText::new(format!("• {} — {} frames", p.addr, p.frames_sent))
                        .size(12.0)
                        .color(Color32::from_rgb(200, 230, 245)),
                );
            }
        });
    }
}

