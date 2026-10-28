use crate::i18n::Language;
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Margin, Response, Sense, Stroke, StrokeKind,
    Ui,
};

// ==========================================
//  SwitchHosts Design System & Theme Tokens
// ==========================================
#[allow(dead_code)]
pub struct SwitchHostsTheme {
    pub bg_rail: Color32,
    pub bg_sidebar: Color32,
    pub bg_editor: Color32,
    pub bg_gutter: Color32,
    pub bg_container: Color32,
    pub bg_elevated: Color32,
    pub bg_hover: Color32,
    pub border: Color32,
    pub border_subtle: Color32,

    // SwitchHosts Signature Red (#D9363E)
    pub primary: Color32,
    pub primary_hover: Color32,
    pub primary_active: Color32,

    // Text Palette
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_tertiary: Color32,
    pub text_gutter: Color32,

    // Status & Switch Colors
    pub switch_on: Color32,
    pub switch_off: Color32,
    pub danger: Color32,
    pub danger_bg: Color32,
    pub success: Color32,
    pub success_bg: Color32,
    pub warning: Color32,
    pub warning_bg: Color32,
    pub info_bg: Color32,
    pub shadow_sm: Color32,
    pub shadow_md: Color32,
}

impl SwitchHostsTheme {
    pub fn new(dark_mode: bool) -> Self {
        if dark_mode {
            Self {
                bg_rail: Color32::from_rgb(20, 20, 20),
                bg_sidebar: Color32::from_rgb(30, 30, 34),
                bg_editor: Color32::from_rgb(24, 24, 27),
                bg_gutter: Color32::from_rgb(19, 19, 22),
                bg_container: Color32::from_rgb(36, 36, 40),
                bg_elevated: Color32::from_rgb(44, 44, 50),
                bg_hover: Color32::from_rgb(56, 56, 64),
                border: Color32::from_rgb(58, 58, 64),
                border_subtle: Color32::from_rgb(45, 45, 50),

                primary: Color32::from_rgb(59, 130, 246), // Vibrant Tech Blue #3B82F6
                primary_hover: Color32::from_rgb(96, 165, 250), // #60A5FA
                primary_active: Color32::from_rgb(37, 99, 235), // #2563EB

                text_primary: Color32::from_rgb(243, 244, 246), // High contrast white-gray
                text_secondary: Color32::from_rgb(209, 213, 219), // High contrast slate-gray
                text_tertiary: Color32::from_rgb(156, 163, 175), // High contrast medium gray (WCAG AA > 4.5:1)
                text_gutter: Color32::from_rgb(107, 114, 128),

                switch_on: Color32::from_rgb(48, 209, 88), // Green #30D158
                switch_off: Color32::from_rgb(70, 70, 75),

                danger: Color32::from_rgb(255, 107, 107),
                danger_bg: Color32::from_rgb(53, 22, 24),
                success: Color32::from_rgb(52, 211, 153),
                success_bg: Color32::from_rgb(18, 44, 26),
                warning: Color32::from_rgb(251, 191, 36),
                warning_bg: Color32::from_rgb(51, 39, 16),
                info_bg: Color32::from_rgb(25, 40, 60),
                shadow_sm: Color32::from_rgba_unmultiplied(0, 0, 0, 90),
                shadow_md: Color32::from_rgba_unmultiplied(0, 0, 0, 140),
            }
        } else {
            Self {
                bg_rail: Color32::from_rgb(229, 231, 235),
                bg_sidebar: Color32::from_rgb(243, 244, 246),
                bg_editor: Color32::from_rgb(255, 255, 255),
                bg_gutter: Color32::from_rgb(249, 250, 251),
                bg_container: Color32::from_rgb(255, 255, 255),
                bg_elevated: Color32::from_rgb(255, 255, 255),
                bg_hover: Color32::from_rgb(229, 231, 235),
                border: Color32::from_rgb(209, 213, 219),
                border_subtle: Color32::from_rgb(229, 231, 235),

                primary: Color32::from_rgb(22, 119, 255), // Modern Tech Blue #1677FF (Ant Design 5)
                primary_hover: Color32::from_rgb(64, 150, 255), // #4096FF
                primary_active: Color32::from_rgb(9, 88, 217), // #0958D9

                text_primary: Color32::from_rgb(17, 24, 39), // High contrast charcoal black
                text_secondary: Color32::from_rgb(55, 65, 81), // Dark slate gray (WCAG AA > 9.5:1)
                text_tertiary: Color32::from_rgb(75, 85, 99), // Medium slate gray (WCAG AA > 6.8:1)
                text_gutter: Color32::from_rgb(156, 163, 175),

                switch_on: Color32::from_rgb(22, 163, 74), // Accessible Green #16A34A
                switch_off: Color32::from_rgb(209, 213, 219),

                danger: Color32::from_rgb(220, 38, 38),
                danger_bg: Color32::from_rgb(254, 242, 242),
                success: Color32::from_rgb(22, 163, 74),
                success_bg: Color32::from_rgb(240, 253, 244),
                warning: Color32::from_rgb(217, 119, 6),
                warning_bg: Color32::from_rgb(254, 252, 232),
                info_bg: Color32::from_rgb(239, 246, 255),
                shadow_sm: Color32::from_rgba_unmultiplied(0, 0, 0, 15),
                shadow_md: Color32::from_rgba_unmultiplied(0, 0, 0, 35),
            }
        }
    }
}

// Keep AntTheme alias for backward compatibility during refactor
pub type AntTheme = SwitchHostsTheme;

// ==========================================
//  SwitchHosts Pill Toggle Switch Widget
// ==========================================
pub fn toggle_switch(ui: &mut Ui, on: &mut bool) -> Response {
    let width = 36.0;
    let height = 20.0;
    let desired_size = vec2(width, height);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());

    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_with_time(response.id, *on, 0.12);

        let is_dark = ui.visuals().dark_mode;
        let off_bg = if is_dark {
            Color32::from_rgb(60, 60, 64)
        } else {
            Color32::from_rgb(209, 209, 214)
        };
        let on_bg = Color32::from_rgb(52, 199, 89); // Apple / SwitchHosts Green
        let bg_color = if *on { on_bg } else { off_bg };

        // Smooth pill track
        let corner_radius = CornerRadius::same(10);
        ui.painter().rect_filled(rect, corner_radius, bg_color);

        // White Knob
        let radius = 8.0;
        let left_center = rect.min.x + 2.0 + radius;
        let right_center = rect.max.x - 2.0 - radius;
        let knob_x = egui::lerp(left_center..=right_center, how_on);
        let knob_center = pos2(knob_x, rect.center().y);

        // Knob shadow
        ui.painter().circle_filled(
            knob_center + vec2(0.0, 1.0),
            radius + 0.5,
            Color32::from_rgba_unmultiplied(0, 0, 0, 30),
        );

        ui.painter()
            .circle_filled(knob_center, radius, Color32::WHITE);
    }

    response
}

// ==========================================
//  Left Navigation Rail Icon Button
// ==========================================
pub fn rail_icon_button(
    ui: &mut Ui,
    icon: &str,
    tooltip: &str,
    is_active: bool,
    theme: &SwitchHostsTheme,
) -> Response {
    let size = vec2(36.0, 36.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());

    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    if ui.is_rect_visible(rect) {
        let (bg, icon_color) = if is_active {
            (theme.primary, Color32::WHITE)
        } else if response.hovered() {
            (theme.bg_hover, theme.text_primary)
        } else {
            (Color32::TRANSPARENT, theme.text_secondary)
        };

        if bg != Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, CornerRadius::same(8), bg);
        }

        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            icon,
            FontId::proportional(17.0),
            icon_color,
        );
    }

    response.on_hover_text(tooltip)
}

// ==========================================
//  SwitchHosts Styled Buttons
// ==========================================
pub fn switch_hosts_button_primary(ui: &mut Ui, text: &str, theme: &SwitchHostsTheme) -> Response {
    let font_id = FontId::proportional(13.0);
    let padding_x = 14.0;
    let height = 28.0;

    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), font_id, Color32::WHITE);
    let width = (galley.size().x + padding_x * 2.0).max(72.0);
    let desired_size = vec2(width, height);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    if ui.is_rect_visible(rect) {
        let bg = if response.is_pointer_button_down_on() {
            theme.primary_active
        } else if response.hovered() {
            theme.primary_hover
        } else {
            theme.primary
        };

        ui.painter().rect_filled(rect, CornerRadius::same(6), bg);
        let text_pos = rect.center() - galley.size() / 2.0;
        ui.painter().galley(text_pos, galley, Color32::WHITE);
    }

    response
}

pub fn switch_hosts_button_default(ui: &mut Ui, text: &str, theme: &SwitchHostsTheme) -> Response {
    let font_id = FontId::proportional(13.0);
    let padding_x = 14.0;
    let height = 28.0;

    let galley_init =
        ui.painter()
            .layout_no_wrap(text.to_string(), font_id.clone(), theme.text_primary);
    let width = (galley_init.size().x + padding_x * 2.0).max(72.0);
    let desired_size = vec2(width, height);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    if ui.is_rect_visible(rect) {
        let (bg, text_color) = if response.is_pointer_button_down_on() || response.hovered() {
            (theme.bg_hover, theme.text_primary)
        } else {
            (theme.bg_container, theme.text_primary)
        };

        ui.painter().rect_filled(rect, CornerRadius::same(6), bg);
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(6),
            Stroke::new(1.0, theme.border),
            StrokeKind::Outside,
        );

        let galley = ui
            .painter()
            .layout_no_wrap(text.to_string(), font_id, text_color);
        let text_pos = rect.center() - galley.size() / 2.0;
        ui.painter().galley(text_pos, galley, text_color);
    }

    response
}

// Aliases for compatibility
pub fn antd_button_primary(ui: &mut Ui, text: &str, theme: &SwitchHostsTheme) -> Response {
    switch_hosts_button_primary(ui, text, theme)
}

pub fn antd_button_default(ui: &mut Ui, text: &str, theme: &SwitchHostsTheme) -> Response {
    switch_hosts_button_default(ui, text, theme)
}

#[allow(dead_code)]
pub fn antd_button_danger(ui: &mut Ui, text: &str, theme: &SwitchHostsTheme) -> Response {
    let font_id = FontId::proportional(13.0);
    let padding_x = 14.0;
    let height = 28.0;

    let galley_init = ui
        .painter()
        .layout_no_wrap(text.to_string(), font_id.clone(), theme.danger);
    let width = (galley_init.size().x + padding_x * 2.0).max(72.0);
    let desired_size = vec2(width, height);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    if ui.is_rect_visible(rect) {
        let (bg, text_color) = if response.hovered() {
            (theme.danger_bg, theme.danger)
        } else {
            (theme.bg_container, theme.danger)
        };

        ui.painter().rect_filled(rect, CornerRadius::same(6), bg);
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(6),
            Stroke::new(1.0, theme.border),
            StrokeKind::Outside,
        );

        let galley = ui
            .painter()
            .layout_no_wrap(text.to_string(), font_id, text_color);
        let text_pos = rect.center() - galley.size() / 2.0;
        ui.painter().galley(text_pos, galley, text_color);
    }

    response
}

#[allow(dead_code)]
pub fn antd_tabs(
    _ui: &mut Ui,
    _selected_tab: &mut crate::ui::ActiveTab,
    _theme: &SwitchHostsTheme,
    _lang: &Language,
) {
}

pub fn antd_badge(ui: &mut Ui, text: &str, bg: Color32, text_color: Color32) {
    egui::Frame::new()
        .fill(bg)
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(text)
                    .font(FontId::proportional(11.0))
                    .color(text_color),
            );
        });
}

// ==========================================
//  Modern Floating Toast Component
// ==========================================
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToastKind {
    Success,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub kind: ToastKind,
    pub created_at: std::time::Instant,
    pub duration: std::time::Duration,
}

impl Toast {
    pub fn new(message: impl Into<String>, kind: ToastKind) -> Self {
        let duration = match kind {
            ToastKind::Error => std::time::Duration::from_millis(3500),
            _ => std::time::Duration::from_millis(2500),
        };
        Self {
            message: message.into(),
            kind,
            created_at: std::time::Instant::now(),
            duration,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }
}

pub fn render_toast_overlay(ctx: &egui::Context, toast: &Option<Toast>, theme: &SwitchHostsTheme) {
    let Some(t) = toast else {
        return;
    };

    let elapsed = t.created_at.elapsed();
    if elapsed >= t.duration {
        return;
    }

    // Request repaint so toast smoothly disappears when duration elapses
    let remaining = t.duration.saturating_sub(elapsed);
    ctx.request_repaint_after(remaining.min(std::time::Duration::from_millis(40)));

    // Calculate fade in / fade out alpha multiplier
    let total_ms = t.duration.as_millis() as f32;
    let elapsed_ms = elapsed.as_millis() as f32;
    let alpha_factor = if elapsed_ms < 150.0 {
        elapsed_ms / 150.0
    } else if total_ms - elapsed_ms < 250.0 {
        ((total_ms - elapsed_ms) / 250.0).clamp(0.0, 1.0)
    } else {
        1.0
    };

    let (icon_symbol, icon_color, border_color) = match t.kind {
        ToastKind::Success => ("✔", theme.success, theme.success),
        ToastKind::Info => ("ℹ", theme.primary, theme.primary),
        ToastKind::Warning => ("⚠", theme.warning, theme.warning),
        ToastKind::Error => ("✖", theme.danger, theme.danger),
    };

    let bg_color = Color32::from_rgba_unmultiplied(
        theme.bg_elevated.r(),
        theme.bg_elevated.g(),
        theme.bg_elevated.b(),
        (250.0 * alpha_factor).round() as u8,
    );
    let border_stroke_color = Color32::from_rgba_unmultiplied(
        border_color.r(),
        border_color.g(),
        border_color.b(),
        (180.0 * alpha_factor).round() as u8,
    );
    let text_color = Color32::from_rgba_unmultiplied(
        theme.text_primary.r(),
        theme.text_primary.g(),
        theme.text_primary.b(),
        (255.0 * alpha_factor).round() as u8,
    );
    let icon_draw_color = Color32::from_rgba_unmultiplied(
        icon_color.r(),
        icon_color.g(),
        icon_color.b(),
        (255.0 * alpha_factor).round() as u8,
    );

    egui::Area::new(egui::Id::new("app_toast_overlay"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 32.0))
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(bg_color)
                .stroke(Stroke::new(1.0, border_stroke_color))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(18, 10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.label(
                            egui::RichText::new(icon_symbol)
                                .font(FontId::proportional(15.0))
                                .color(icon_draw_color)
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(&t.message)
                                .font(FontId::proportional(13.5))
                                .color(text_color)
                                .strong(),
                        );
                    });
                });
        });
}
