//! Brand-aligned visuals: deep navy canvas, cyan–cobalt accents, glass panels.

use egui::{
    Color32, CornerRadius, CursorIcon, Direction, FontId, Frame, Margin, Pos2, Rect, Response,
    Sense, Shape, Stroke, StrokeKind, Style, TextureHandle, Ui, Vec2,
};
use egui::style::StyleModifier;

pub const BG_TOP: Color32 = Color32::from_rgb(22, 32, 48);
pub const BG_BOTTOM: Color32 = Color32::from_rgb(8, 11, 18);
pub const CYAN: Color32 = Color32::from_rgb(0, 242, 255);
pub const COBALT: Color32 = Color32::from_rgb(0, 91, 255);
pub const MUTED: Color32 = Color32::from_rgba_unmultiplied_const(150, 175, 200, 230);
pub const GLASS_FILL: Color32 = Color32::from_rgba_unmultiplied_const(16, 22, 34, 230);

/// Vertical / section rhythm (px). Use these instead of ad-hoc literals.
pub const GAP_XS: f32 = 4.0;
pub const GAP_SM: f32 = 6.0;
pub const GAP_MD: f32 = 10.0;

/// Apply once at startup — dark chrome, cyan hover, transparent panel chrome.
pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.global_style()).clone();
    style.visuals.dark_mode = true;
    style.visuals.panel_fill = Color32::TRANSPARENT;
    style.visuals.window_fill = Color32::TRANSPARENT;
    style.visuals.window_stroke = Stroke::NONE;
    // Popups / combo menus: solid backing (global fallback).
    style.visuals.extreme_bg_color = Color32::from_rgb(16, 22, 34);
    style.visuals.widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::NONE;
    style.visuals.widgets.noninteractive.fg_stroke.color = MUTED;
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgba_unmultiplied(20, 30, 48, 200);
    style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgba_unmultiplied(20, 30, 48, 200);
    style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    style.visuals.widgets.inactive.fg_stroke.color = Color32::from_gray(230);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgba_unmultiplied(0, 180, 255, 55);
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgba_unmultiplied(0, 180, 255, 55);
    style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    style.visuals.widgets.hovered.fg_stroke.color = Color32::WHITE;
    style.visuals.widgets.active.bg_fill = Color32::from_rgba_unmultiplied(0, 200, 255, 90);
    style.visuals.widgets.active.fg_stroke.color = Color32::WHITE;
    style.visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(0, 200, 255, 50);
    style.visuals.selection.stroke = Stroke::new(1.0, CYAN);
    style.visuals.widgets.open.bg_fill = Color32::from_rgb(22, 30, 46);
    style.visuals.widgets.open.weak_bg_fill = Color32::from_rgb(22, 30, 46);
    style.visuals.widgets.open.bg_stroke = Stroke::NONE;
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.window_margin = Margin::same(0);
    ctx.set_global_style(style);
}

pub fn paint_background(painter: &egui::Painter, rect: Rect) {
    painter.add(Shape::gradient_rect(
        rect,
        Direction::TopDown,
        [BG_TOP, BG_BOTTOM],
    ));
    let glow_y = rect.top() + rect.height() * 0.18;
    let center = Pos2::new(rect.center().x, glow_y);
    painter.circle_filled(
        center,
        rect.width() * 0.55,
        Color32::from_rgba_unmultiplied(0, 100, 180, 22),
    );
    painter.circle_filled(
        center,
        rect.width() * 0.35,
        Color32::from_rgba_unmultiplied(0, 220, 255, 12),
    );
}

pub fn glass_frame() -> Frame {
    Frame::NONE
        .fill(GLASS_FILL)
        .corner_radius(CornerRadius::same(12))
        .stroke(Stroke::NONE)
        .inner_margin(Margin::symmetric(14, 10))
        .outer_margin(Margin::symmetric(0, 6))
}

/// Solid rows + menu background for [`egui::ComboBox`] popups (not transparent).
pub fn combo_popup_style() -> StyleModifier {
    StyleModifier::new(|style: &mut Style| {
        let menu_bg = Color32::from_rgb(16, 22, 34);
        let row = Color32::from_rgb(24, 32, 50);
        let row_hi = Color32::from_rgb(34, 48, 72);
        style.visuals.extreme_bg_color = menu_bg;
        style.visuals.widgets.inactive.weak_bg_fill = row;
        style.visuals.widgets.inactive.bg_fill = row;
        style.visuals.widgets.hovered.weak_bg_fill = row_hi;
        style.visuals.widgets.hovered.bg_fill = row_hi;
        style.visuals.widgets.active.weak_bg_fill = row_hi;
        style.visuals.widgets.active.bg_fill = row_hi;
        style.visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(0, 200, 255, 120);
    })
}

pub fn load_brand_texture(ctx: &egui::Context) -> Option<TextureHandle> {
    let bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/brand-hero.png"));
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let rgba = img.into_raw();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);
    Some(ctx.load_texture("ayr_link_brand", color_image, Default::default()))
}

/// Cyan → cobalt pill used for the primary action.
pub fn gradient_primary_button(
    ui: &mut Ui,
    label: &str,
    min_width: f32,
    enabled: bool,
) -> Response {
    let font = FontId::proportional(15.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, Color32::WHITE);
    let pad = Vec2::new(18.0, 8.0);
    let size = Vec2::new(
        (galley.size().x + pad.x * 2.0).max(min_width),
        galley.size().y + pad.y * 2.0,
    );
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter_at(rect);
        let rounding = CornerRadius::same(10);
        if enabled && response.hovered() {
            painter.add(Shape::rect_filled(
                rect.expand(2.0),
                rounding,
                Color32::from_rgba_unmultiplied(0, 255, 255, 28),
            ));
        }
        let (left, right) = if enabled {
            (CYAN, COBALT)
        } else {
            (
                Color32::from_rgb(45, 55, 68),
                Color32::from_rgb(32, 38, 48),
            )
        };
        painter.add(Shape::gradient_rect(
            rect,
            Direction::LeftToRight,
            [left, right],
        ));
        painter.rect_stroke(
            rect,
            rounding,
            Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(255, 255, 255, if enabled { 55 } else { 25 }),
            ),
            StrokeKind::Inside,
        );
        let text_color = if enabled {
            Color32::WHITE
        } else {
            Color32::from_rgba_unmultiplied(200, 210, 220, 180)
        };
        painter.galley(
            rect.center() - galley.size() * 0.5,
            galley,
            text_color,
        );
    }
    if enabled && response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

/// Dark fill + warm outline for destructive / stop.
pub fn gradient_stop_button(ui: &mut Ui, label: &str, min_width: f32) -> Response {
    let font = FontId::proportional(15.0);
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        font,
        Color32::from_rgb(255, 230, 230),
    );
    let pad = Vec2::new(18.0, 8.0);
    let size = Vec2::new(
        (galley.size().x + pad.x * 2.0).max(min_width),
        galley.size().y + pad.y * 2.0,
    );
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter_at(rect);
        let rounding = CornerRadius::same(10);
        let fill_top = Color32::from_rgba_unmultiplied(48, 22, 28, 240);
        let fill_bot = Color32::from_rgba_unmultiplied(22, 12, 16, 240);
        painter.add(Shape::gradient_rect(
            rect,
            Direction::TopDown,
            [fill_top, fill_bot],
        ));
        let stroke = if response.hovered() {
            Stroke::new(1.5, Color32::from_rgb(255, 120, 120))
        } else {
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 100, 100, 160))
        };
        painter.rect_stroke(rect, rounding, stroke, StrokeKind::Inside);
        painter.galley(
            rect.center() - galley.size() * 0.5,
            galley,
            Color32::from_rgb(255, 235, 235),
        );
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}
