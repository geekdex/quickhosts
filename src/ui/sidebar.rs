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
    pub reorder: Option<(usize, usize)>,
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
        reorder: None,
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

        let drag_source_id = ui.make_persistent_id("hosts_sidebar_drag_source");
        let hover_target_id = ui.make_persistent_id("hosts_sidebar_hover_target");

        let pointer_pos = ui.ctx().pointer_interact_pos();
        let pointer_released = ui.input(|i| i.pointer.any_released());
        let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));

        if escape_pressed {
            ui.ctx().data_mut(|d| {
                let _: Option<(String, usize)> = d.remove_temp(drag_source_id);
                let _: Option<usize> = d.remove_temp(hover_target_id);
            });
        }

        let active_drag: Option<(String, usize)> = ui.ctx().data(|d| d.get_temp(drag_source_id));
        let is_dragging = active_drag.is_some() && !escape_pressed;

        if is_dragging {
            ui.ctx().request_repaint();
        }

        // --- Hosts Item List (SwitchHosts Tree/List Style) ---
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let filter_q = search_query.to_lowercase();
                let hosts_len = hosts.len();

                let mut current_hover_target: Option<usize> = None;

                for (idx, host) in hosts.iter_mut().enumerate() {
                    if !filter_q.is_empty() && !host.title.to_lowercase().contains(&filter_q) {
                        continue;
                    }

                    let can_move_up = idx > 0;
                    let can_move_down = idx + 1 < hosts_len;

                    let is_selected = selected_id.as_deref() == Some(&host.id);
                    let is_this_dragged =
                        active_drag.as_ref().is_some_and(|(id, _)| id == &host.id);

                    let item_id = ui.make_persistent_id(&host.id);
                    let prev_rect = ui.ctx().data(|d| d.get_temp::<egui::Rect>(item_id));
                    let row_rect = ui.available_rect_before_wrap();
                    let target_width = (row_rect.width() - 4.0).max(0.0);
                    let target_rect = prev_rect.unwrap_or_else(|| {
                        egui::Rect::from_min_size(row_rect.min, egui::vec2(target_width, 36.0))
                    });

                    // Check if pointer is hovering over this row during drag
                    if is_dragging && !is_this_dragged {
                        if let Some(p) = pointer_pos {
                            if target_rect.contains(p) {
                                current_hover_target = Some(idx);
                            }
                        }
                    }

                    // Create row background interaction supporting click and drag
                    let row_interact = ui.interact(target_rect, item_id, Sense::click_and_drag());
                    let is_hovered = row_interact.hovered() && !ui.input(|i| i.pointer.any_down());

                    if row_interact.drag_started() {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(drag_source_id, (host.id.clone(), idx));
                        });
                        action.selected_id = Some(host.id.clone());
                    }

                    if is_dragging {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    } else if is_hovered {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                    }

                    if row_interact.clicked() {
                        action.selected_id = Some(host.id.clone());
                    }

                    // Context menu (right-click) for position adjustment
                    row_interact.context_menu(|ui| {
                        ui.set_min_width(120.0);
                        if ui
                            .add_enabled(can_move_up, egui::Button::new(tr(lang, "menu_move_up")))
                            .clicked()
                        {
                            action.move_up_id = Some(host.id.clone());
                            action.selected_id = Some(host.id.clone());
                            ui.close_kind(egui::UiKind::Menu);
                        }
                        if ui
                            .add_enabled(
                                can_move_down,
                                egui::Button::new(tr(lang, "menu_move_down")),
                            )
                            .clicked()
                        {
                            action.move_down_id = Some(host.id.clone());
                            action.selected_id = Some(host.id.clone());
                            ui.close_kind(egui::UiKind::Menu);
                        }
                    });

                    let is_target = is_dragging && current_hover_target == Some(idx);

                    let bg_color = if is_this_dragged {
                        theme.primary.linear_multiply(0.2)
                    } else if is_selected {
                        theme.primary
                    } else if is_hovered || is_target {
                        theme.bg_hover
                    } else {
                        Color32::TRANSPARENT
                    };

                    let title_color = if is_this_dragged {
                        theme.text_secondary
                    } else if is_selected {
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

                            if is_selected && !is_this_dragged {
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

                    // Draw drop indicator when this item is the target of a drag
                    if is_target {
                        if let Some((_, from_idx)) = &active_drag {
                            let (y_line, dot_x) = if idx <= *from_idx {
                                (target_rect.top(), target_rect.left() + 4.0)
                            } else {
                                (target_rect.bottom(), target_rect.left() + 4.0)
                            };
                            let indicator_color = theme.primary;
                            ui.painter().hline(
                                target_rect.x_range(),
                                y_line,
                                Stroke::new(2.5, indicator_color),
                            );
                            ui.painter().circle_filled(
                                egui::pos2(dot_x, y_line),
                                3.5,
                                indicator_color,
                            );
                        }
                    }

                    // Render floating drag preview under mouse cursor
                    if is_this_dragged {
                        if let Some(p) = pointer_pos {
                            let painter = ui.ctx().layer_painter(egui::LayerId::new(
                                egui::Order::Tooltip,
                                ui.make_persistent_id("hosts_dnd_ghost_layer"),
                            ));
                            let badge_size =
                                egui::vec2((target_rect.width() * 0.8).clamp(120.0, 200.0), 28.0);
                            let badge_rect =
                                egui::Rect::from_min_size(p + egui::vec2(12.0, 10.0), badge_size);
                            let ghost_bg = if dark_mode {
                                Color32::from_rgba_unmultiplied(40, 44, 52, 230)
                            } else {
                                Color32::from_rgba_unmultiplied(245, 247, 250, 235)
                            };
                            painter.rect_filled(badge_rect, CornerRadius::same(6), ghost_bg);
                            painter.rect_stroke(
                                badge_rect,
                                CornerRadius::same(6),
                                Stroke::new(1.5, theme.primary),
                                egui::StrokeKind::Outside,
                            );
                            painter.text(
                                badge_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                format!("{} {}", icon, host.title),
                                FontId::proportional(12.5),
                                theme.text_primary,
                            );
                        }
                    }

                    ui.ctx()
                        .data_mut(|d| d.insert_temp(item_id, frame_res.response.rect));
                    row_interact.on_hover_text(&host.title);

                    ui.add_space(2.0);
                }

                // If currently dragging, update stored hover target
                if is_dragging {
                    if let Some(target) = current_hover_target {
                        ui.ctx()
                            .data_mut(|d| d.insert_temp(hover_target_id, target));
                    }
                }

                // On mouse release: execute reorder if dropped on a valid target
                if pointer_released && is_dragging {
                    let drag_data: Option<(String, usize)> =
                        ui.ctx().data_mut(|d| d.remove_temp(drag_source_id));
                    let hover_data: Option<usize> =
                        ui.ctx().data_mut(|d| d.remove_temp(hover_target_id));

                    if let (Some((drag_id, from_idx)), Some(to_idx)) = (drag_data, hover_data) {
                        if from_idx != to_idx && from_idx < hosts_len && to_idx < hosts_len {
                            action.reorder = Some((from_idx, to_idx));
                            action.selected_id = Some(drag_id);
                            action.state_changed = true;
                        }
                    }
                }
            });
    });

    action
}
