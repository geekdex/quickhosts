use crate::i18n::{tr, Language};
use crate::model::{HostItem, HostType};
use crate::ui::widgets::{toggle_switch, SwitchHostsTheme};
use egui::{Color32, CornerRadius, FontId, Margin, Sense, Stroke, Ui};

#[allow(dead_code)]
pub struct SidebarAction {
    pub selected_id: Option<String>,
    pub open_add_local: bool,
    pub open_add_remote: bool,
    pub trigger_export_hosts_file: bool,
    pub trigger_import_hosts_file: bool,
    pub state_changed: bool,
    pub move_up_id: Option<String>,
    pub move_down_id: Option<String>,
}

pub fn render_sidebar(
    ui: &mut Ui,
    hosts: &mut [HostItem],
    selected_id: &Option<String>,
    search_query: &mut String,
    lang: &Language,
    dark_mode: bool,
) -> SidebarAction {
    let mut action = SidebarAction {
        selected_id: None,
        open_add_local: false,
        open_add_remote: false,
        trigger_export_hosts_file: false,
        trigger_import_hosts_file: false,
        state_changed: false,
        move_up_id: None,
        move_down_id: None,
    };

    let theme = SwitchHostsTheme::new(dark_mode);

    ui.vertical(|ui| {
        ui.add_space(8.0);

        // --- SwitchHosts Top Sidebar Header Bar ---
        ui.horizontal(|ui| {
            ui.add_space(4.0);

            // Add (+) Menu Button
            ui.menu_button(
                egui::RichText::new("➕")
                    .size(15.0)
                    .color(theme.text_primary),
                |ui| {
                    ui.set_min_width(170.0);
                    if ui.button(tr(lang, "btn_add_local")).clicked() {
                        action.open_add_local = true;
                        ui.close_kind(egui::UiKind::Menu);
                    }
                    if ui.button(tr(lang, "btn_add_remote")).clicked() {
                        action.open_add_remote = true;
                        ui.close_kind(egui::UiKind::Menu);
                    }
                    ui.separator();
                    if ui.button(tr(lang, "btn_export_hosts")).clicked() {
                        action.trigger_export_hosts_file = true;
                        ui.close_kind(egui::UiKind::Menu);
                    }
                    if ui.button(tr(lang, "btn_import_hosts")).clicked() {
                        action.trigger_import_hosts_file = true;
                        ui.close_kind(egui::UiKind::Menu);
                    }
                },
            )
            .response
            .on_hover_text(tr(lang, "btn_add_local"));

            ui.add_space(4.0);

            // Search Bar Input
            ui.add(
                egui::TextEdit::singleline(search_query)
                    .hint_text(tr(lang, "search_hosts_hint"))
                    .desired_width(ui.available_width() - 4.0),
            );
        });

        ui.add_space(8.0);

        // Divider line under sidebar top header
        let cursor_y = ui.cursor().min.y;
        ui.painter().hline(
            ui.max_rect().x_range(),
            cursor_y,
            Stroke::new(1.0, theme.border_subtle),
        );

        ui.add_space(6.0);

        // --- Hosts Item List (SwitchHosts Tree/List Style) ---
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let filter_q = search_query.to_lowercase();

                for host in hosts.iter_mut() {
                    if !filter_q.is_empty() && !host.title.to_lowercase().contains(&filter_q) {
                        continue;
                    }

                    let is_selected = selected_id.as_deref() == Some(&host.id);

                    let item_id = ui.make_persistent_id(&host.id);
                    let prev_rect = ui.ctx().data(|d| d.get_temp::<egui::Rect>(item_id));
                    let row_rect = ui.available_rect_before_wrap();
                    let target_width = (row_rect.width() - 4.0).max(0.0);
                    let target_rect = prev_rect.unwrap_or_else(|| {
                        egui::Rect::from_min_size(row_rect.min, egui::vec2(target_width, 36.0))
                    });

                    // Create row background interaction BEFORE contents so toggle_switch (created after) sits on top
                    let row_interact = ui.interact(target_rect, item_id, Sense::click());
                    let is_hovered = row_interact.hovered() && !ui.input(|i| i.pointer.any_down());

                    if is_hovered {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }

                    if row_interact.clicked() {
                        action.selected_id = Some(host.id.clone());
                    }

                    let bg_color = if is_selected {
                        theme.primary
                    } else if is_hovered {
                        theme.bg_hover
                    } else {
                        Color32::TRANSPARENT
                    };

                    let title_color = if is_selected {
                        Color32::WHITE
                    } else {
                        theme.text_primary
                    };

                    let icon = match &host.host_type {
                        HostType::System => "💻",
                        HostType::Local => "📄",
                        HostType::Remote {
                            is_fetching,
                            last_error,
                            ..
                        } => {
                            if *is_fetching {
                                "🔄"
                            } else if last_error.is_some() {
                                "⚠️"
                            } else {
                                "🌐"
                            }
                        }
                    };

                    let item_frame = egui::Frame::new()
                        .fill(bg_color)
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::symmetric(10, 8));

                    let frame_res = item_frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let available_w = ui.available_width();
                            let switch_w = 36.0;
                            let spacing = 8.0;
                            let label_max_w = (available_w - switch_w - spacing).max(10.0);

                            let mut label_text =
                                egui::RichText::new(format!("{} {}", icon, host.title))
                                    .font(FontId::proportional(13.5))
                                    .color(title_color);

                            if is_selected {
                                label_text = label_text.strong();
                            }

                            ui.allocate_ui_with_layout(
                                egui::vec2(label_max_w, ui.available_height()),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.add(
                                        egui::Label::new(label_text)
                                            .truncate()
                                            .selectable(false)
                                            .sense(Sense::empty()),
                                    );
                                },
                            );

                            // Right-aligned Toggle Switch
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let mut enabled = host.enabled;
                                    let switch_res = toggle_switch(ui, &mut enabled);
                                    if switch_res.changed() {
                                        host.enabled = enabled;
                                        action.state_changed = true;
                                        action.selected_id = Some(host.id.clone());
                                    }
                                },
                            );
                        });
                    });

                    ui.ctx()
                        .data_mut(|d| d.insert_temp(item_id, frame_res.response.rect));
                    row_interact.on_hover_text(&host.title);

                    ui.add_space(2.0);
                }
            });
    });

    action
}
