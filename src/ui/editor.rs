use crate::i18n::{tr, Language};
use crate::model::{HostItem, HostType, UpdateInterval};
use crate::ui::widgets::SwitchHostsTheme;
use egui::{
    text::{CCursor, CCursorRange, LayoutJob},
    vec2, Align2, Color32, CornerRadius, FontId, Margin, Sense, Stroke, TextFormat, Ui,
};

#[derive(Debug, Clone, Default)]
pub struct EditorSearchState {
    pub is_open: bool,
    pub query: String,
    pub case_sensitive: bool,
    pub current_match_idx: usize,
    pub focus_input: bool,
    pub scroll_to_target_line: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorSearchMatch {
    pub byte_start: usize,
    pub byte_end: usize,
    pub char_start: usize,
    pub char_end: usize,
    pub line_index: usize,
}

pub fn find_search_matches(
    content: &str,
    query: &str,
    case_sensitive: bool,
) -> Vec<EditorSearchMatch> {
    if query.is_empty() {
        return Vec::new();
    }

    let mut matches = Vec::new();
    let query_lower = query.to_lowercase();
    let query_char_len = query.chars().count();

    let mut cur_byte = 0;
    let mut cur_char = 0;

    for (line_idx, line) in content.split('\n').enumerate() {
        if case_sensitive {
            let mut offset = 0;
            while let Some(pos) = line[offset..].find(query) {
                let match_start_in_line = offset + pos;
                let match_end_in_line = match_start_in_line + query.len();

                let chars_before = line[..match_start_in_line].chars().count();
                let char_start = cur_char + chars_before;
                let char_end = char_start + query_char_len;

                matches.push(EditorSearchMatch {
                    byte_start: cur_byte + match_start_in_line,
                    byte_end: cur_byte + match_end_in_line,
                    char_start,
                    char_end,
                    line_index: line_idx,
                });

                offset = match_end_in_line.max(offset + 1);
            }
        } else {
            let line_lower = line.to_lowercase();
            let mut offset = 0;
            while let Some(pos) = line_lower[offset..].find(&query_lower) {
                let lower_start = offset + pos;
                let lower_end = lower_start + query_lower.len();

                let chars_before = line_lower[..lower_start].chars().count();
                let char_start = cur_char + chars_before;
                let char_end = char_start + query_char_len;

                let byte_start_in_line = line
                    .char_indices()
                    .nth(chars_before)
                    .map(|(i, _)| i)
                    .unwrap_or(line.len());
                let byte_len = line[byte_start_in_line..]
                    .chars()
                    .take(query_char_len)
                    .map(|c| c.len_utf8())
                    .sum::<usize>();
                let byte_end_in_line = byte_start_in_line + byte_len;

                matches.push(EditorSearchMatch {
                    byte_start: cur_byte + byte_start_in_line,
                    byte_end: cur_byte + byte_end_in_line,
                    char_start,
                    char_end,
                    line_index: line_idx,
                });

                offset = lower_end.max(offset + 1);
            }
        }

        cur_byte += line.len() + 1; // +1 for '\n'
        cur_char += line.chars().count() + 1;
    }

    matches
}

pub struct EditorViewResponse {
    pub content_changed: bool,
    pub fetch_remote_requested: bool,
    pub delete_requested: bool,
    pub title_changed: bool,
    pub toggle_preview_requested: bool,
    pub export_requested: bool,
}

#[derive(Clone, Copy)]
enum SearchNavIcon {
    Up,
    Down,
    Close,
}

fn search_nav_button(
    ui: &mut Ui,
    icon: SearchNavIcon,
    tooltip: &str,
    theme: &SwitchHostsTheme,
) -> egui::Response {
    let size = vec2(22.0, 22.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());

    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    let bg_color = if response.is_pointer_button_down_on() || response.hovered() {
        theme.bg_hover
    } else {
        Color32::TRANSPARENT
    };

    if bg_color != Color32::TRANSPARENT {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(4), bg_color);
    }

    let icon_color = if response.hovered() {
        theme.text_primary
    } else {
        theme.text_secondary
    };
    let stroke = Stroke::new(1.6, icon_color);
    let c = rect.center();

    match icon {
        SearchNavIcon::Up => {
            let p1 = c + vec2(-4.0, 2.0);
            let p2 = c + vec2(0.0, -2.5);
            let p3 = c + vec2(4.0, 2.0);
            ui.painter().line_segment([p1, p2], stroke);
            ui.painter().line_segment([p2, p3], stroke);
        }
        SearchNavIcon::Down => {
            let p1 = c + vec2(-4.0, -2.0);
            let p2 = c + vec2(0.0, 2.5);
            let p3 = c + vec2(4.0, -2.0);
            ui.painter().line_segment([p1, p2], stroke);
            ui.painter().line_segment([p2, p3], stroke);
        }
        SearchNavIcon::Close => {
            let r = 3.5;
            let p1 = c + vec2(-r, -r);
            let p2 = c + vec2(r, r);
            let p3 = c + vec2(r, -r);
            let p4 = c + vec2(-r, r);
            ui.painter().line_segment([p1, p2], stroke);
            ui.painter().line_segment([p3, p4], stroke);
        }
    }

    response.on_hover_text(tooltip)
}

fn draw_search_icon(ui: &mut Ui, theme: &SwitchHostsTheme) {
    let size = vec2(16.0, 16.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let c = rect.center();
    let circle_c = c + vec2(-1.5, -1.5);
    let stroke = Stroke::new(1.5, theme.text_secondary);
    ui.painter().circle_stroke(circle_c, 3.8, stroke);
    let p1 = circle_c + vec2(2.7, 2.7);
    let p2 = circle_c + vec2(5.5, 5.5);
    ui.painter()
        .line_segment([p1, p2], Stroke::new(1.8, theme.text_secondary));
}

pub fn render_editor_view(
    ui: &mut Ui,
    host: &mut HostItem,
    search_state: &mut EditorSearchState,
    font_size: f32,
    lang: &Language,
    dark_mode: bool,
) -> EditorViewResponse {
    let mut response = EditorViewResponse {
        content_changed: false,
        fetch_remote_requested: false,
        delete_requested: false,
        title_changed: false,
        toggle_preview_requested: false,
        export_requested: false,
    };

    let theme = SwitchHostsTheme::new(dark_mode);

    ui.vertical(|ui| {
        ui.add_space(4.0);

        // Shortcut detection for Ctrl+F / Cmd+F and Esc
        let ctrl_f = ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F));
        if ctrl_f {
            search_state.is_open = true;
            search_state.focus_input = true;
        }

        if search_state.is_open && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            search_state.is_open = false;
        }

        // --- SwitchHosts Top Editor Header Bar ---
        ui.horizontal(|ui| {
            ui.add_space(4.0);

            let icon = match &host.host_type {
                HostType::System => "💻",
                HostType::Local => "📄",
                HostType::Remote { .. } => "🌐",
            };

            ui.label(egui::RichText::new(icon).size(18.0));
            ui.add_space(4.0);

            // Title Editor / Display
            let title_edit = ui.add(
                egui::TextEdit::singleline(&mut host.title)
                    .font(FontId::proportional(15.0))
                    .desired_width(220.0),
            );
            if title_edit.changed() {
                response.title_changed = true;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Delete button
                if !host.is_system() {
                    if ui
                        .button(egui::RichText::new("🗑").size(15.0))
                        .on_hover_text(tr(lang, "btn_delete"))
                        .clicked()
                    {
                        response.delete_requested = true;
                    }
                    ui.add_space(6.0);
                }

                // Export button
                if ui
                    .button(egui::RichText::new("📤").size(15.0))
                    .on_hover_text(tr(lang, "btn_export_hosts"))
                    .clicked()
                {
                    response.export_requested = true;
                }
                ui.add_space(6.0);

                // Combined preview button
                if ui
                    .button(egui::RichText::new("👁").size(15.0))
                    .on_hover_text(tr(lang, "tab_preview"))
                    .clicked()
                {
                    response.toggle_preview_requested = true;
                }
                ui.add_space(6.0);

                // Search button
                let search_icon_text = if search_state.is_open {
                    egui::RichText::new("🔍").size(15.0).color(theme.primary)
                } else {
                    egui::RichText::new("🔍").size(15.0)
                };
                if ui
                    .button(search_icon_text)
                    .on_hover_text(tr(lang, "btn_search"))
                    .clicked()
                {
                    search_state.is_open = !search_state.is_open;
                    if search_state.is_open {
                        search_state.focus_input = true;
                    }
                }
                ui.add_space(6.0);

                // Sync button (for Remote Host)
                if let HostType::Remote { is_fetching, .. } = &host.host_type {
                    if *is_fetching {
                        ui.spinner();
                    } else if ui
                        .button(egui::RichText::new("🔄").size(15.0))
                        .on_hover_text(tr(lang, "btn_sync_remote"))
                        .clicked()
                    {
                        response.fetch_remote_requested = true;
                    }
                    ui.add_space(6.0);
                }
            });
        });

        ui.add_space(6.0);

        // Divider
        let cursor_y = ui.cursor().min.y;
        ui.painter().hline(
            ui.max_rect().x_range(),
            cursor_y,
            Stroke::new(1.0, theme.border_subtle),
        );

        // Remote URL & Auto-refresh banner if Remote Host
        if let HostType::Remote {
            url,
            interval,
            last_updated,
            last_error,
            ..
        } = &mut host.host_type
        {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(tr(lang, "remote_url_label"))
                        .color(theme.text_secondary)
                        .size(12.0),
                );
                let url_edit = ui.add(
                    egui::TextEdit::singleline(url).desired_width(ui.available_width() - 200.0),
                );
                if url_edit.changed() {
                    response.content_changed = true;
                }

                ui.add_space(8.0);
                egui::ComboBox::from_id_salt("remote_interval_combo_editor")
                    .selected_text(interval.display_name(lang))
                    .show_ui(ui, |ui| {
                        for opt in UpdateInterval::all() {
                            if ui
                                .selectable_value(interval, opt.clone(), opt.display_name(lang))
                                .clicked()
                            {
                                response.content_changed = true;
                            }
                        }
                    });
            });

            if let Some(err) = last_error {
                ui.label(
                    egui::RichText::new(format!("⚠️ {}", err))
                        .color(theme.danger)
                        .size(11.0),
                );
            } else if let Some(last) = last_updated {
                ui.label(
                    egui::RichText::new(format!(
                        "{} {}",
                        tr(lang, "last_sync_prefix"),
                        last.format("%H:%M:%S")
                    ))
                    .color(theme.text_tertiary)
                    .size(11.0),
                );
            }
        }

        ui.add_space(4.0);

        let is_read_only = host.read_only || matches!(host.host_type, HostType::Remote { .. });

        // Calculate search matches for current hosts content
        let mut matches = if search_state.is_open && !search_state.query.is_empty() {
            find_search_matches(
                &host.content,
                &search_state.query,
                search_state.case_sensitive,
            )
        } else {
            Vec::new()
        };
        let mut total_matches = matches.len();

        if total_matches == 0 || search_state.current_match_idx >= total_matches {
            search_state.current_match_idx = 0;
        }

        // --- Search Bar Toolbar Panel ---
        if search_state.is_open {
            egui::Frame::new()
                .fill(theme.bg_container)
                .stroke(Stroke::new(1.0, theme.border))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        draw_search_icon(ui, &theme);
                        ui.add_space(2.0);

                        let input_width = (ui.available_width() - 250.0).clamp(140.0, 320.0);
                        let search_edit = egui::TextEdit::singleline(&mut search_state.query)
                            .hint_text(tr(lang, "search_content_hint"))
                            .desired_width(input_width);

                        let input_res = ui.add(search_edit);

                        if search_state.focus_input {
                            input_res.request_focus();
                            search_state.focus_input = false;
                        }

                        let mut prev_match_clicked = false;
                        let mut next_match_clicked = false;

                        if input_res.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            if ui.input(|i| i.modifiers.shift) {
                                prev_match_clicked = true;
                            } else {
                                next_match_clicked = true;
                            }
                        }

                        if input_res.changed() {
                            matches = if !search_state.query.is_empty() {
                                find_search_matches(
                                    &host.content,
                                    &search_state.query,
                                    search_state.case_sensitive,
                                )
                            } else {
                                Vec::new()
                            };
                            total_matches = matches.len();
                            search_state.current_match_idx = 0;
                            if let Some(m) = matches.first() {
                                search_state.scroll_to_target_line = Some(m.line_index);
                            } else {
                                search_state.scroll_to_target_line = None;
                            }
                        }

                        let count_text = if search_state.query.is_empty() {
                            egui::RichText::new("").color(theme.text_tertiary)
                        } else if total_matches == 0 {
                            egui::RichText::new(tr(lang, "search_no_matches"))
                                .color(theme.danger)
                                .size(12.0)
                        } else {
                            egui::RichText::new(format!(
                                "{}/{}",
                                search_state.current_match_idx + 1,
                                total_matches
                            ))
                            .color(theme.text_secondary)
                            .strong()
                            .size(12.5)
                        };
                        ui.label(count_text);

                        ui.add_space(4.0);

                        if search_nav_button(ui, SearchNavIcon::Up, tr(lang, "search_prev"), &theme)
                            .clicked()
                        {
                            prev_match_clicked = true;
                        }

                        if search_nav_button(
                            ui,
                            SearchNavIcon::Down,
                            tr(lang, "search_next"),
                            &theme,
                        )
                        .clicked()
                        {
                            next_match_clicked = true;
                        }

                        ui.add_space(2.0);

                        let (case_rect, case_resp) =
                            ui.allocate_exact_size(vec2(24.0, 22.0), Sense::click());
                        if case_resp.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        let case_bg = if search_state.case_sensitive {
                            theme.primary.linear_multiply(0.18)
                        } else if case_resp.hovered() {
                            theme.bg_hover
                        } else {
                            Color32::TRANSPARENT
                        };
                        if case_bg != Color32::TRANSPARENT {
                            ui.painter()
                                .rect_filled(case_rect, CornerRadius::same(4), case_bg);
                        }
                        let case_color = if search_state.case_sensitive {
                            theme.primary
                        } else if case_resp.hovered() {
                            theme.text_primary
                        } else {
                            theme.text_tertiary
                        };
                        ui.painter().text(
                            case_rect.center(),
                            Align2::CENTER_CENTER,
                            "Aa",
                            FontId::monospace(11.5),
                            case_color,
                        );
                        if case_resp
                            .on_hover_text(tr(lang, "search_case_sensitive"))
                            .clicked()
                        {
                            search_state.case_sensitive = !search_state.case_sensitive;
                            matches = if !search_state.query.is_empty() {
                                find_search_matches(
                                    &host.content,
                                    &search_state.query,
                                    search_state.case_sensitive,
                                )
                            } else {
                                Vec::new()
                            };
                            total_matches = matches.len();
                            search_state.current_match_idx = 0;
                            if let Some(m) = matches.first() {
                                search_state.scroll_to_target_line = Some(m.line_index);
                            } else {
                                search_state.scroll_to_target_line = None;
                            }
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if search_nav_button(
                                ui,
                                SearchNavIcon::Close,
                                tr(lang, "search_close"),
                                &theme,
                            )
                            .clicked()
                            {
                                search_state.is_open = false;
                            }
                        });

                        if total_matches > 0 {
                            if next_match_clicked {
                                search_state.current_match_idx =
                                    (search_state.current_match_idx + 1) % total_matches;
                                if let Some(m) = matches.get(search_state.current_match_idx) {
                                    search_state.scroll_to_target_line = Some(m.line_index);
                                }
                            } else if prev_match_clicked {
                                search_state.current_match_idx =
                                    if search_state.current_match_idx == 0 {
                                        total_matches - 1
                                    } else {
                                        search_state.current_match_idx - 1
                                    };
                                if let Some(m) = matches.get(search_state.current_match_idx) {
                                    search_state.scroll_to_target_line = Some(m.line_index);
                                }
                            }
                        }
                    });
                });
            ui.add_space(4.0);
        }

        // Extract line metadata before mutable borrow (accounting for all lines including trailing newline)
        let raw_lines: Vec<&str> = host.content.split('\n').collect();
        let line_flags: Vec<(bool, bool)> = raw_lines
            .iter()
            .map(|l| {
                let t = l.trim_start();
                (t.starts_with('#'), t.is_empty())
            })
            .collect();
        let line_count = line_flags.len().max(1);
        let byte_count = host.content.len();

        let active_idx =
            if search_state.is_open && !search_state.query.is_empty() && !matches.is_empty() {
                Some(search_state.current_match_idx)
            } else {
                None
            };

        // --- Line-Numbered Code Editor View ---
        let mut layouter = |ui: &egui::Ui, text_buf: &dyn egui::TextBuffer, _wrap_width: f32| {
            let mut layout_job = highlight_hosts_syntax_with_search(
                text_buf.as_str(),
                font_size,
                dark_mode,
                &matches,
                active_idx,
            );
            // Disable wrap so code editor behaves like VS Code / CodeMirror: 1:1 gutter alignment
            layout_job.wrap.max_width = f32::INFINITY;
            ui.painter().layout_job(layout_job)
        };

        let font_id = FontId::monospace(font_size);
        let mut toggle_comment_line_idx: Option<usize> = None;
        let text_edit_id = ui.make_persistent_id("hosts_editor_content");

        egui::Frame::new()
            .fill(theme.bg_editor)
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .corner_radius(CornerRadius::same(0))
            .inner_margin(Margin::same(0))
            .show(ui, |ui| {
                let available_height = ui.available_height() - 26.0; // Reserve space for bottom status bar
                let sample_galley =
                    ui.painter()
                        .layout_no_wrap("0".to_string(), font_id.clone(), Color32::WHITE);
                let row_height = sample_galley.size().y.max(16.0);
                let char_w = sample_galley.size().x.max(8.0);
                let num_digits = line_count.to_string().len().max(2);
                let gutter_width = ((num_digits + 2) as f32 * char_w + 14.0).max(42.0);
                let top_padding = 4.0;
                let total_content_height =
                    (line_count as f32 * row_height + top_padding * 2.0).max(available_height);

                let scroll_target_line = search_state.scroll_to_target_line.take();

                let mut scroll_area = egui::ScrollArea::both()
                    .auto_shrink([false; 2])
                    .max_height(available_height);

                if let Some(target_line) = scroll_target_line {
                    let target_y = top_padding + (target_line as f32) * row_height;
                    let center_offset =
                        (target_y - available_height * 0.5 + row_height * 0.5).max(0.0);
                    scroll_area = scroll_area.vertical_scroll_offset(center_offset);

                    // Select matching range in TextEditState if available
                    if let Some(m) = matches.get(search_state.current_match_idx) {
                        let mut state =
                            egui::TextEdit::load_state(ui.ctx(), text_edit_id).unwrap_or_default();
                        let min = CCursor::new(m.char_start);
                        let max = CCursor::new(m.char_end);
                        state
                            .cursor
                            .set_char_range(Some(CCursorRange::two(min, max)));
                        state.store(ui.ctx(), text_edit_id);
                    }
                }

                scroll_area.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // 1. Left Interactive Line Numbers Gutter (SwitchHosts feature)
                        ui.allocate_ui_with_layout(
                            vec2(gutter_width, total_content_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_min_width(gutter_width);
                                ui.spacing_mut().item_spacing.y = 0.0;

                                // Gutter background
                                let gutter_rect = ui.available_rect_before_wrap();
                                ui.painter().rect_filled(
                                    egui::Rect::from_min_size(
                                        gutter_rect.min,
                                        vec2(gutter_width, total_content_height),
                                    ),
                                    CornerRadius::ZERO,
                                    theme.bg_gutter,
                                );

                                ui.add_space(top_padding);

                                for idx in 0..line_count {
                                    let line_num = idx + 1;
                                    let (rect, line_resp) = ui.allocate_exact_size(
                                        vec2(gutter_width - 8.0, row_height),
                                        Sense::click(),
                                    );

                                    if !ui.is_rect_visible(rect) {
                                        continue;
                                    }

                                    let (is_commented, is_blank) =
                                        line_flags.get(idx).copied().unwrap_or((false, true));

                                    let num_color = if line_resp.hovered() && !is_read_only {
                                        theme.primary
                                    } else if is_blank {
                                        theme.text_gutter.linear_multiply(0.4)
                                    } else if is_commented {
                                        theme.text_gutter.linear_multiply(0.6)
                                    } else {
                                        theme.text_secondary
                                    };

                                    if line_resp.hovered() && !is_read_only {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    }

                                    if line_resp.clicked() && !is_read_only {
                                        toggle_comment_line_idx = Some(idx);
                                    }

                                    let mut line_label = line_num.to_string();
                                    if line_resp.hovered() && !is_read_only {
                                        line_label = if is_commented {
                                            format!("▶{}", line_num)
                                        } else {
                                            format!("#{}", line_num)
                                        };
                                    }

                                    ui.painter().text(
                                        rect.right_center(),
                                        Align2::RIGHT_CENTER,
                                        line_label,
                                        font_id.clone(),
                                        num_color,
                                    );
                                }
                            },
                        );

                        // Vertical divider after line numbers
                        let vline_x = ui.cursor().min.x;
                        ui.painter().vline(
                            vline_x,
                            ui.min_rect().min.y..=(ui.min_rect().min.y + total_content_height),
                            Stroke::new(1.0, theme.border_subtle),
                        );

                        ui.add_space(6.0);

                        // 2. Multiline Text Editor
                        // For read-only hosts (including Remote Hosts), passing an immutable &str allows
                        // mouse drag selection and native system copy (Ctrl+C), but prevents editing content.
                        let mut content_slice = host.content.as_str();
                        let text_edit = if is_read_only {
                            egui::TextEdit::multiline(&mut content_slice)
                        } else {
                            egui::TextEdit::multiline(&mut host.content)
                        };

                        let text_edit = text_edit
                            .id(text_edit_id)
                            .font(font_id.clone())
                            .code_editor()
                            .frame(egui::Frame::NONE)
                            .desired_width(f32::INFINITY)
                            .min_size(vec2(
                                (ui.available_width() - 6.0).max(100.0),
                                (total_content_height - top_padding * 2.0).max(row_height),
                            ))
                            .margin(Margin::symmetric(4, top_padding as i8))
                            .layouter(&mut layouter)
                            .interactive(true);

                        let edit_response = ui.add(text_edit);
                        if edit_response.changed() && !is_read_only {
                            response.content_changed = true;
                        }
                    });
                });
            });

        // Handle clicking gutter line numbers to toggle # comment
        if let Some(target_idx) = toggle_comment_line_idx {
            let mut lines: Vec<String> = host.content.split('\n').map(|s| s.to_string()).collect();
            if target_idx < lines.len() {
                let current_line = &lines[target_idx];
                let trimmed = current_line.trim_start();
                if let Some(without_hash) = trimmed.strip_prefix('#') {
                    // Uncomment: remove '#' and any single space after it
                    let indent_len = current_line.len() - trimmed.len();
                    let indent = &current_line[..indent_len];
                    let uncommented = without_hash.strip_prefix(' ').unwrap_or(without_hash);
                    lines[target_idx] = format!("{}{}", indent, uncommented);
                } else if !trimmed.is_empty() {
                    // Comment: add "# "
                    let indent_len = current_line.len() - trimmed.len();
                    let indent = &current_line[..indent_len];
                    lines[target_idx] = format!("{}# {}", indent, trimmed);
                }
                host.content = lines.join("\n");
                response.content_changed = true;
            }
        }

        // --- SwitchHosts Bottom Status Bar ---
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(
                egui::RichText::new(format!("{} lines   {} B", line_count, byte_count))
                    .color(theme.text_tertiary)
                    .size(12.0),
            );

            if !is_read_only {
                ui.add_space(12.0);
                let hint_str = if lang == &Language::Zh {
                    "💡 单击行号可快速切换单行注释 (#)"
                } else {
                    "💡 Click line number to toggle comment (#)"
                };
                ui.label(
                    egui::RichText::new(hint_str)
                        .color(theme.text_tertiary.linear_multiply(0.8))
                        .size(11.0),
                );
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(12.0);
                if is_read_only {
                    ui.label(
                        egui::RichText::new(tr(lang, "read_only_hint"))
                            .color(theme.text_tertiary)
                            .size(11.5),
                    );
                }
            });
        });
    });

    response
}

// Hosts format syntax highlighter
pub fn highlight_hosts_syntax(text: &str, font_size: f32, dark_mode: bool) -> LayoutJob {
    highlight_hosts_syntax_with_search(text, font_size, dark_mode, &[], None)
}

#[derive(Clone, Copy)]
struct SearchHighlightParams<'a> {
    matches: &'a [EditorSearchMatch],
    active_match_idx: Option<usize>,
    normal_match_bg: Color32,
    active_match_bg: Color32,
}

fn append_highlighted_section(
    job: &mut LayoutJob,
    section_text: &str,
    section_byte_start: usize,
    base_format: &TextFormat,
    params: &SearchHighlightParams<'_>,
) {
    if section_text.is_empty() {
        return;
    }
    let section_byte_end = section_byte_start + section_text.len();

    let overlapping: Vec<(usize, usize, bool)> = params
        .matches
        .iter()
        .enumerate()
        .filter_map(|(idx, m)| {
            if m.byte_end <= section_byte_start || m.byte_start >= section_byte_end {
                None
            } else {
                let start_in_sec = m.byte_start.max(section_byte_start) - section_byte_start;
                let end_in_sec = m.byte_end.min(section_byte_end) - section_byte_start;
                let is_active = Some(idx) == params.active_match_idx;
                Some((start_in_sec, end_in_sec, is_active))
            }
        })
        .collect();

    if overlapping.is_empty() {
        job.append(section_text, 0.0, base_format.clone());
        return;
    }

    let mut cursor = 0;
    for (m_start, m_end, is_active) in overlapping {
        if m_start > cursor {
            job.append(&section_text[cursor..m_start], 0.0, base_format.clone());
        }

        let mut match_format = base_format.clone();
        if is_active {
            match_format.background = params.active_match_bg;
            match_format.color = Color32::WHITE;
        } else {
            match_format.background = params.normal_match_bg;
        }

        job.append(&section_text[m_start..m_end], 0.0, match_format);
        cursor = m_end;
    }

    if cursor < section_text.len() {
        job.append(&section_text[cursor..], 0.0, base_format.clone());
    }
}

pub fn highlight_hosts_syntax_with_search(
    text: &str,
    font_size: f32,
    dark_mode: bool,
    matches: &[EditorSearchMatch],
    active_match_idx: Option<usize>,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    let font_id = FontId::monospace(font_size);

    let (comment_color, ip_color, host_color, normal_match_bg, active_match_bg) = if dark_mode {
        (
            Color32::from_rgb(74, 222, 128), // Bright Mint Green #4ADE80 (Contrast > 10:1)
            Color32::from_rgb(56, 189, 248), // Sky Blue #38BDF8 (Contrast > 10:1)
            Color32::from_rgb(243, 244, 246),
            Color32::from_rgba_unmultiplied(202, 138, 4, 110), // Warm gold highlight for matches
            Color32::from_rgb(249, 115, 22),                   // Bright orange for active match
        )
    } else {
        (
            Color32::from_rgb(21, 128, 61), // Accessible Forest Green #15803D (WCAG AA > 4.6:1)
            Color32::from_rgb(2, 132, 199), // Accessible Blue #0284C7 (WCAG AA > 4.8:1)
            Color32::from_rgb(17, 24, 39),
            Color32::from_rgba_unmultiplied(254, 240, 138, 220), // Light yellow highlight for matches
            Color32::from_rgb(249, 115, 22),                     // Bright orange for active match
        )
    };

    let params = SearchHighlightParams {
        matches,
        active_match_idx,
        normal_match_bg,
        active_match_bg,
    };

    let comment_format = TextFormat {
        font_id: font_id.clone(),
        color: comment_color,
        ..Default::default()
    };
    let ip_format = TextFormat {
        font_id: font_id.clone(),
        color: ip_color,
        ..Default::default()
    };
    let host_format = TextFormat {
        font_id: font_id.clone(),
        color: host_color,
        ..Default::default()
    };

    let lines: Vec<&str> = text.split('\n').collect();
    let total_lines = lines.len();
    let mut line_byte_start = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            append_highlighted_section(&mut job, line, line_byte_start, &comment_format, &params);
        } else {
            let parts: Vec<&str> = line.splitn(2, '#').collect();
            let code_part = parts[0];
            let inline_comment = parts.get(1);

            let tokens: Vec<&str> = code_part.split_whitespace().collect();
            if tokens.is_empty() {
                append_highlighted_section(&mut job, line, line_byte_start, &host_format, &params);
            } else {
                let ip_str = tokens[0];
                if let Some(pos) = line.find(ip_str) {
                    append_highlighted_section(
                        &mut job,
                        &line[..pos],
                        line_byte_start,
                        &host_format,
                        &params,
                    );
                    append_highlighted_section(
                        &mut job,
                        ip_str,
                        line_byte_start + pos,
                        &ip_format,
                        &params,
                    );
                    let rest = &line[pos + ip_str.len()..];
                    let rest_byte_start = line_byte_start + pos + ip_str.len();
                    if inline_comment.is_some() {
                        let comment_pos = rest.find('#').unwrap_or(rest.len());
                        append_highlighted_section(
                            &mut job,
                            &rest[..comment_pos],
                            rest_byte_start,
                            &host_format,
                            &params,
                        );
                        append_highlighted_section(
                            &mut job,
                            &rest[comment_pos..],
                            rest_byte_start + comment_pos,
                            &comment_format,
                            &params,
                        );
                    } else {
                        append_highlighted_section(
                            &mut job,
                            rest,
                            rest_byte_start,
                            &host_format,
                            &params,
                        );
                    }
                } else {
                    append_highlighted_section(
                        &mut job,
                        line,
                        line_byte_start,
                        &host_format,
                        &params,
                    );
                }
            }
        }

        if i < total_lines - 1 {
            job.append("\n", 0.0, host_format.clone());
        }

        line_byte_start += line.len() + 1; // +1 for '\n'
    }

    job
}
