use crate::hosts_manager::{is_system_hosts_writable, parse_hosts_stats};
use crate::i18n::{tr, Language};
use crate::model::HostItem;
use crate::ui::editor::highlight_hosts_syntax;
use crate::ui::widgets::{antd_badge, antd_button_default, antd_button_primary, AntTheme};
use egui::{CornerRadius, Margin, Stroke, Ui};

pub struct PreviewAction {
    pub apply_to_system: bool,
    pub copy_to_clipboard: bool,
}

pub fn render_preview_view(
    ui: &mut Ui,
    combined_content: &str,
    active_hosts: &[&HostItem],
    master_enabled: bool,
    search_query: &mut String,
    lang: &Language,
    dark_mode: bool,
) -> PreviewAction {
    let mut action = PreviewAction {
        apply_to_system: false,
        copy_to_clipboard: false,
    };

    let theme = AntTheme::new(dark_mode);
    let stats = parse_hosts_stats(combined_content);
    let writable = is_system_hosts_writable();

    ui.vertical(|ui| {
        ui.add_space(8.0);

        // --- AntD Top Bar ---
        egui::Frame::new()
            .fill(theme.bg_elevated)
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(16, 12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("👁").size(22.0));
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(tr(lang, "preview_title"))
                            .font(egui::FontId::proportional(18.0))
                            .strong()
                            .color(theme.text_primary),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let apply_text = if writable {
                            tr(lang, "apply_now")
                        } else {
                            tr(lang, "elevate_and_apply")
                        };

                        if antd_button_primary(ui, apply_text, &theme).clicked() {
                            action.apply_to_system = true;
                        }

                        ui.add_space(8.0);
                        if antd_button_default(ui, tr(lang, "copy_all"), &theme).clicked() {
                            action.copy_to_clipboard = true;
                        }
                    });
                });
            });

        ui.add_space(12.0);

        // --- AntD Card Stats & Status Bar ---
        egui::Frame::new()
            .fill(theme.bg_elevated)
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;

                    // Active profiles badge
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("📊").size(14.0));
                        ui.label(
                            egui::RichText::new(format!("{}: ", tr(lang, "active_profiles")))
                                .color(theme.text_secondary)
                                .size(13.0),
                        );
                        ui.label(
                            egui::RichText::new(format!("{}", active_hosts.len()))
                                .strong()
                                .color(theme.primary)
                                .size(14.0),
                        );
                    });

                    // Active rules
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("✓").size(14.0).color(theme.success));
                        ui.label(
                            egui::RichText::new(format!("{}: ", tr(lang, "active_rules")))
                                .color(theme.text_secondary)
                                .size(13.0),
                        );
                        ui.label(
                            egui::RichText::new(format!("{}", stats.active_rules))
                                .strong()
                                .color(theme.success)
                                .size(14.0),
                        );
                    });

                    // Total lines
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("📄").size(14.0));
                        ui.label(
                            egui::RichText::new(format!("{}: ", tr(lang, "total_lines")))
                                .color(theme.text_secondary)
                                .size(13.0),
                        );
                        ui.label(
                            egui::RichText::new(format!("{}", stats.total_lines))
                                .strong()
                                .color(theme.text_primary)
                                .size(14.0),
                        );
                    });

                    // Permission status
                    if writable {
                        antd_badge(
                            ui,
                            tr(lang, "direct_write"),
                            theme.success_bg,
                            theme.success,
                        );
                    } else {
                        antd_badge(
                            ui,
                            tr(lang, "elevation_needed"),
                            theme.warning_bg,
                            theme.warning,
                        );
                    }

                    // Engine status
                    if !master_enabled {
                        antd_badge(ui, tr(lang, "engine_off"), theme.danger_bg, theme.danger);
                    }
                });
            });

        ui.add_space(12.0);

        // --- Search bar ---
        egui::Frame::new()
            .fill(theme.bg_elevated)
            .stroke(Stroke::new(1.0, theme.border))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(10, 7))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🔍").color(theme.text_tertiary));
                    ui.add(
                        egui::TextEdit::singleline(search_query)
                            .hint_text(tr(lang, "filter_preview_hint"))
                            .desired_width(f32::INFINITY),
                    );
                    if !search_query.is_empty() && ui.small_button("×").clicked() {
                        search_query.clear();
                    }
                });
            });

        ui.add_space(12.0);

        // --- Preview Code Display ---
        let content_to_display = if search_query.is_empty() {
            combined_content.to_string()
        } else {
            let q = search_query.to_lowercase();
            combined_content
                .lines()
                .filter(|line| line.to_lowercase().contains(&q))
                .collect::<Vec<&str>>()
                .join("\n")
        };

        let preview_height = (ui.available_height() - 10.0).max(120.0);

        egui::Frame::new()
            .fill(theme.bg_editor)
            .stroke(Stroke::new(1.0, theme.border))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), preview_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        egui::ScrollArea::both()
                            .auto_shrink([false; 2])
                            .max_height(preview_height)
                            .show(ui, |ui| {
                                let mut dummy_content = content_to_display;
                                let mut layouter =
                                    |ui: &egui::Ui,
                                     text_buf: &dyn egui::TextBuffer,
                                     wrap_width: f32| {
                                        let mut layout_job = highlight_hosts_syntax(
                                            text_buf.as_str(),
                                            13.0,
                                            dark_mode,
                                        );
                                        layout_job.wrap.max_width = wrap_width;
                                        ui.painter().layout_job(layout_job)
                                    };

                                ui.add(
                                    egui::TextEdit::multiline(&mut dummy_content)
                                        .font(egui::TextStyle::Monospace)
                                        .code_editor()
                                        .frame(egui::Frame::NONE)
                                        .desired_width(f32::INFINITY)
                                        .min_size(egui::vec2(
                                            ui.available_width(),
                                            (preview_height - 20.0).max(40.0),
                                        ))
                                        .interactive(false)
                                        .layouter(&mut layouter),
                                );
                            });
                    },
                );
            });

        ui.add_space(8.0);
    });

    action
}
