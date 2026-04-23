//! Brand-aligned visuals — mockup: deep #050A14 canvas, neon cyan, cyan→violet gradients.

use egui::{
    Color32, CornerRadius, CursorIcon, Direction, FontId, Frame, Margin, Pos2, Rect, Response,
    Sense, Shape, Stroke, StrokeKind, Style, TextureHandle, Ui, Vec2,
};
use egui::style::StyleModifier;

/// Canvas (radial mockup feel is painted on top in `paint_background`).
pub const BG_TOP: Color32 = Color32::from_rgb(5, 10, 20);
pub const BG_BOTTOM: Color32 = Color32::from_rgb(4, 7, 14);
pub const CYAN: Color32 = Color32::from_rgb(0, 242, 255);
/// Mockup accent highlights (#33FBFF range).
pub const CYAN_BRIGHT: Color32 = Color32::from_rgb(51, 251, 255);
pub const COBALT: Color32 = Color32::from_rgb(0, 91, 255);
/// End of level / CTA gradient (#4B2FD3).
pub const VIOLET: Color32 = Color32::from_rgb(75, 47, 211);
pub const MUTED: Color32 = Color32::from_rgba_unmultiplied_const(140, 165, 195, 235);
pub const GLASS_FILL: Color32 = Color32::from_rgba_unmultiplied_const(12, 18, 30, 235);
pub const GLASS_STROKE: Color32 = Color32::from_rgba_unmultiplied_const(255, 255, 255, 18);

/// When `true`, major containers get **1px** saturated outline colors (layout debug). Turn off for normal chrome.
pub const DEBUG_LAYOUT_OUTLINES: bool = true;

#[inline]
pub fn layout_debug_stroke_prod(prod: Color32, dev: Color32) -> Stroke {
    if DEBUG_LAYOUT_OUTLINES {
        Stroke::new(1.0, dev)
    } else {
        Stroke::new(1.0, prod)
    }
}

#[inline]
pub fn layout_debug_stroke_prod_none(dev: Color32) -> Stroke {
    if DEBUG_LAYOUT_OUTLINES {
        Stroke::new(1.0, dev)
    } else {
        Stroke::NONE
    }
}

/// Vertical / section rhythm (px). Use these instead of ad-hoc literals.
pub const GAP_XS: f32 = 4.0;
pub const GAP_SM: f32 = 6.0;
pub const GAP_MD: f32 = 10.0;

/// Vertical gap between stacked main blocks (header, version strip, glass cards), in px.
/// Integer because egui 0.34 [`Margin`] uses `i8`; cast to `f32` for [`egui::Style::spacing`] fields.
pub const SECTION_STACK_GAP: i8 = 2;

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
    let glow_y = rect.top() + rect.height() * 0.16;
    let center = Pos2::new(rect.center().x, glow_y);
    painter.circle_filled(
        center,
        rect.width() * 0.58,
        Color32::from_rgba_unmultiplied(0, 80, 160, 26),
    );
    painter.circle_filled(
        center,
        rect.width() * 0.38,
        Color32::from_rgba_unmultiplied(0, 200, 255, 18),
    );
}

pub fn glass_frame() -> Frame {
    Frame::NONE
        .fill(GLASS_FILL)
        .corner_radius(CornerRadius::same(14))
        .stroke(Stroke::new(1.0, GLASS_STROKE))
        .inner_margin(Margin::symmetric(14, 12))
        .outer_margin(Margin::symmetric(0_i8, SECTION_STACK_GAP))
}

/// Expand this UI to the full width the parent row allocated (header, cards,
/// banners, and footer blocks share one column width).
#[inline]
pub fn fill_horizontal_strip(ui: &mut Ui) {
    let w = ui.available_width();
    if w.is_finite() && w > 0.0 {
        ui.set_width(w);
    }
}

/// Solid rows + opaque popup chrome for [`egui::ComboBox`] (`Frame::popup` uses `window_fill`).
pub fn combo_popup_style() -> StyleModifier {
    StyleModifier::new(|style: &mut Style| {
        let menu_bg = Color32::from_rgb(16, 22, 34);
        let row = Color32::from_rgb(24, 32, 50);
        let row_hi = Color32::from_rgb(34, 48, 72);
        style.visuals.extreme_bg_color = menu_bg;
        style.visuals.window_fill = menu_bg;
        style.visuals.window_stroke = Stroke::new(1.0, GLASS_STROKE);
        style.visuals.widgets.inactive.weak_bg_fill = row;
        style.visuals.widgets.inactive.bg_fill = row;
        style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
        style.visuals.widgets.hovered.weak_bg_fill = row_hi;
        style.visuals.widgets.hovered.bg_fill = row_hi;
        style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
        style.visuals.widgets.active.weak_bg_fill = row_hi;
        style.visuals.widgets.active.bg_fill = row_hi;
        style.visuals.widgets.active.bg_stroke = Stroke::NONE;
        style.visuals.widgets.open.bg_stroke = Stroke::NONE;
        style.visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(0, 200, 255, 120);
        style.spacing.button_padding = Vec2::new(6.0, 4.0);
    })
}

pub fn load_brand_texture(ctx: &egui::Context) -> Option<TextureHandle> {
    load_png_texture(
        ctx,
        "ayr_link_brand",
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/brand-hero.png")),
    )
}

/// Square app artwork (AYR Audio Link icon) for header + textures.
pub fn load_app_icon_texture(ctx: &egui::Context) -> Option<TextureHandle> {
    load_png_texture(
        ctx,
        "ayr_link_app_icon",
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/app-icon.ico")),
    )
}

pub fn load_png_texture(ctx: &egui::Context, id: &str, bytes: &[u8]) -> Option<TextureHandle> {
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let rgba = img.into_raw();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);
    Some(ctx.load_texture(id, color_image, Default::default()))
}

/// Circular badge with a glyph (emoji / symbol) for section cards.
pub fn section_icon(ui: &mut Ui, glyph: &str) -> Response {
    let d = 44.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(d), Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter_at(rect);
        let c = rect.center();
        let r = d * 0.46;
        p.circle_filled(c, r, Color32::from_rgba_unmultiplied(8, 14, 26, 252));
        p.circle_stroke(
            c,
            r,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 220, 255, 100)),
        );
        let font = FontId::proportional(20.0);
        let galley = p.layout_no_wrap(glyph.to_string(), font, CYAN_BRIGHT);
        p.galley(c - galley.size() * 0.5, galley, Color32::WHITE);
    }
    response
}

/// Cyan → violet pill used for the primary action (mockup CTA).
pub fn gradient_primary_button(
    ui: &mut Ui,
    label: &str,
    min_width: f32,
    enabled: bool,
) -> Response {
    let font = FontId::proportional(16.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, Color32::WHITE);
    let pad = Vec2::new(20.0, 12.0);
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
        let rounding = CornerRadius::same(12);
        if enabled && response.hovered() {
            painter.add(Shape::rect_filled(
                rect.expand(2.0),
                rounding,
                Color32::from_rgba_unmultiplied(0, 255, 255, 32),
            ));
        }
        let (left, right) = if enabled {
            (CYAN_BRIGHT, VIOLET)
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
