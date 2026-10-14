use crate::i18n::{tr, Language};
use crate::model::{AppConfig, HostItem, HostType, UpdateInterval};
use crate::storage::save_config;
use crate::ui::widgets::{antd_button_default, antd_button_primary, AntTheme};
use egui::{CornerRadius, Margin, RichText, Stroke};
use std::fs;

#[derive(Debug, Clone, PartialEq)]
pub enum DialogMode {
    None,
    AddLocal,
    AddRemote,
    ImportHosts,
    ExportHosts,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportMode {
    CreateNew,
    Overwrite,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExportTarget {
    All,
    Specific(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExportFormat {
    HostsText,
    Json,
}

#[derive(Debug, Clone)]
pub struct DialogState {
    pub mode: DialogMode,
    pub title_input: String,
    pub url_input: String,
    pub content_input: String,
    pub interval_input: UpdateInterval,
    pub error_msg: Option<String>,
    pub success_msg: Option<String>,

    // Import Modal State
    pub import_file_path: String,
    pub import_file_name: String,
    pub import_content: String,
    pub import_mode: ImportMode,
    pub import_title_input: String,
    pub import_target_host_id: Option<String>,

    // Export Modal State
    pub export_target: ExportTarget,
    pub export_format: ExportFormat,
}

impl Default for DialogState {
    fn default() -> Self {
        Self {
            mode: DialogMode::None,
            title_input: String::new(),
            url_input: String::new(),
            content_input: String::new(),
            interval_input: UpdateInterval::Hour1,
            error_msg: None,
            success_msg: None,
            import_file_path: String::new(),
            import_file_name: String::new(),
            import_content: String::new(),
            import_mode: ImportMode::CreateNew,
            import_title_input: String::new(),
            import_target_host_id: None,
            export_target: ExportTarget::All,
            export_format: ExportFormat::HostsText,
        }
    }
}

impl DialogState {
    pub fn open_add_local(&mut self) {
        self.mode = DialogMode::AddLocal;
        self.title_input = "New Local Hosts".to_string();
        self.content_input = "# Local Hosts\n127.0.0.1 dev.test.com\n".to_string();
        self.error_msg = None;
        self.success_msg = None;
    }

    pub fn open_add_remote(&mut self) {
        self.mode = DialogMode::AddRemote;
        self.title_input = "New Remote Hosts".to_string();
        self.url_input = String::new();
        self.interval_input = UpdateInterval::Hour1;
        self.error_msg = None;
        self.success_msg = None;
    }

    pub fn open_import(
        &mut self,
        file_path: String,
        file_name: String,
        content: String,
        config: &AppConfig,
    ) {
        self.mode = DialogMode::ImportHosts;
        self.import_file_path = file_path;
        self.import_file_name = file_name.clone();
        self.import_content = content;
        self.import_mode = ImportMode::CreateNew;
        let default_title = if !file_name.is_empty() {
            let p = std::path::Path::new(&file_name);
            p.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        } else {
            "Imported Hosts".to_string()
        };
        self.import_title_input = default_title;
        self.import_target_host_id = config
            .hosts
            .iter()
            .find(|h| matches!(h.host_type, HostType::Local))
            .map(|h| h.id.clone());
        self.error_msg = None;
        self.success_msg = None;
    }

    pub fn open_export(&mut self, initial_selected_id: Option<String>) {
        self.mode = DialogMode::ExportHosts;
        if let Some(id) = initial_selected_id {
            self.export_target = ExportTarget::Specific(id);
        } else {
            self.export_target = ExportTarget::All;
        }
        self.export_format = ExportFormat::HostsText;
        self.error_msg = None;
        self.success_msg = None;
    }

    pub fn close(&mut self) {
        self.mode = DialogMode::None;
        self.error_msg = None;
        self.success_msg = None;
    }
}

/// Helper function to safely truncate string at valid UTF-8 character boundaries
pub fn safe_truncate_str(s: &str, max_bytes: usize) -> String {
    if s.len() > max_bytes {
        let mut end = max_bytes;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...\n\n[Total bytes: {}]", &s[..end], s.len())
    } else {
        s.to_string()
    }
}

/// Helper function to extract plain hosts text from content string
pub fn extract_hosts_text(input: &str) -> String {
    let trimmed = input.trim();
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(content) = val.get("content").and_then(|c| c.as_str()) {
            return content.to_string();
        }
    }
    input.to_string()
}

/// Helper function to build exported content based on selected scope and format
pub fn get_export_content(
    config: &AppConfig,
    target: &ExportTarget,
    format: &ExportFormat,
) -> String {
    match target {
        ExportTarget::All => match format {
            ExportFormat::HostsText => {
                let active: Vec<&HostItem> = config.hosts.iter().filter(|h| h.enabled).collect();
                crate::hosts_manager::build_combined_hosts(config.master_enabled, &active)
            }
            ExportFormat::Json => serde_json::to_string_pretty(config).unwrap_or_default(),
        },
        ExportTarget::Specific(id) => {
            if let Some(host) = config.hosts.iter().find(|h| &h.id == id) {
                match format {
                    ExportFormat::HostsText => host.content.clone(),
                    ExportFormat::Json => serde_json::to_string_pretty(host).unwrap_or_default(),
                }
            } else {
                String::new()
            }
        }
    }
}

/// Parse and import content into AppConfig with custom default title for plain text
pub fn parse_and_import_hosts_custom(
    input: &str,
    default_title: String,
    config: &mut AppConfig,
) -> Result<usize, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("导入内容为空 / Import content is empty".to_string());
    }

    if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
        let new_item = HostItem::new_local(default_title, trimmed.to_string());
        let id = new_item.id.clone();
        config.hosts.push(new_item);
        config.selected_host_id = Some(id);
        return Ok(1);
    }

    parse_and_import_hosts(input, config)
}

pub fn parse_and_import_hosts(input: &str, config: &mut AppConfig) -> Result<usize, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("导入内容为空 / Import content is empty".to_string());
    }

    // 1. Try parsing full AppConfig struct
    if let Ok(full_cfg) = serde_json::from_str::<AppConfig>(trimmed) {
        let count = full_cfg.hosts.len();
        *config = full_cfg;
        if config.selected_host_id.is_none()
            || !config
                .hosts
                .iter()
                .any(|h| Some(&h.id) == config.selected_host_id.as_ref())
        {
            config.selected_host_id = config.hosts.first().map(|h| h.id.clone());
        }
        return Ok(count);
    }

    // 2. Try parsing Vec<HostItem> directly
    if let Ok(items) = serde_json::from_str::<Vec<HostItem>>(trimmed) {
        if !items.is_empty() {
            let count = items.len();
            for item in items {
                if !config.hosts.iter().any(|h| h.id == item.id) {
                    config.hosts.push(item);
                } else {
                    let mut new_item = item;
                    new_item.id = uuid::Uuid::new_v4().to_string();
                    config.hosts.push(new_item);
                }
            }
            if config.selected_host_id.is_none() {
                config.selected_host_id = config.hosts.first().map(|h| h.id.clone());
            }
            return Ok(count);
        }
    }

    // 3. Try parsing generic JSON (serde_json::Value)
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
        let items_arr = val
            .as_array()
            .or_else(|| val.get("items").and_then(|v| v.as_array()))
            .or_else(|| val.get("hosts").and_then(|v| v.as_array()));

        if let Some(arr) = items_arr {
            let mut imported_count = 0;
            for v in arr {
                let title = v
                    .get("title")
                    .or_else(|| v.get("name"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("Imported Host")
                    .to_string();

                let content = v
                    .get("content")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();

                let enabled = v
                    .get("enabled")
                    .or_else(|| v.get("is_on"))
                    .or_else(|| v.get("on"))
                    .and_then(|b| b.as_bool())
                    .unwrap_or(true);

                let host_type_str = v
                    .get("type")
                    .or_else(|| v.get("host_type"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("local");

                let url = v
                    .get("url")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();

                let new_item = if host_type_str.eq_ignore_ascii_case("remote") && !url.is_empty() {
                    let mut item = HostItem::new_remote(title, url, UpdateInterval::Hour1);
                    item.enabled = enabled;
                    if !content.is_empty() {
                        item.content = content;
                    }
                    item
                } else {
                    let mut item = HostItem::new_local(title, content);
                    item.enabled = enabled;
                    item
                };

                config.hosts.push(new_item);
                imported_count += 1;
            }

            if imported_count > 0 {
                if config.selected_host_id.is_none() {
                    config.selected_host_id = config.hosts.first().map(|h| h.id.clone());
                }
                return Ok(imported_count);
            }
        }
    }

    // 4. Fallback: If plain text format, import as plain hosts file
    if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
        let new_item = HostItem::new_local("Imported Hosts".to_string(), trimmed.to_string());
        let id = new_item.id.clone();
        config.hosts.push(new_item);
        if config.selected_host_id.is_none() {
            config.selected_host_id = Some(id);
        }
        return Ok(1);
    }

    Err("无法解析文件内容 / Invalid Hosts file.".to_string())
}

pub fn render_custom_dialog_overlay(
    ctx: &egui::Context,
    state: &mut DialogState,
    config: &mut AppConfig,
    lang: &Language,
    dark_mode: bool,
    on_remote_added: &mut Option<String>,
    apply_system_hosts_needed: &mut bool,
) {
    if state.mode == DialogMode::None {
        return;
    }

    let theme = AntTheme::new(dark_mode);

    let (title_str, viewport_id_str, default_size) = match state.mode {
        DialogMode::AddLocal => (
            tr(lang, "modal_add_local_title"),
            "add_local_viewport",
            egui::vec2(540.0, 390.0),
        ),
        DialogMode::AddRemote => (
            tr(lang, "modal_add_remote_title"),
            "add_remote_viewport",
            egui::vec2(540.0, 390.0),
        ),
        DialogMode::ImportHosts => (
            tr(lang, "modal_import_title"),
            "import_hosts_viewport",
            egui::vec2(580.0, 460.0),
        ),
        DialogMode::ExportHosts => (
            tr(lang, "modal_export_title"),
            "export_hosts_viewport",
            egui::vec2(580.0, 480.0),
        ),
        _ => ("", "dialog_viewport", egui::vec2(500.0, 360.0)),
    };

    let viewport_id = egui::ViewportId::from_hash_of(viewport_id_str);
    let builder = egui::ViewportBuilder::default()
        .with_title(title_str)
        .with_inner_size([default_size.x, default_size.y])
        .with_resizable(true)
        .with_always_on_top();

    ctx.show_viewport_immediate(viewport_id, builder, |ctx, _class| {
        let mut visuals = if dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.panel_fill = theme.bg_editor;
        visuals.window_fill = theme.bg_elevated;
        ctx.set_visuals(visuals);

        if ctx.input(|i| i.viewport().close_requested()) {
            state.close();
            return;
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme.bg_editor)
                    .inner_margin(Margin::same(16)),
            )
            .show(ctx, |ui| {
                if let Some(err) = &state.error_msg {
                    ui.label(RichText::new(err).color(theme.danger).size(13.0));
                    ui.add_space(6.0);
                }
                if let Some(succ) = &state.success_msg {
                    ui.label(RichText::new(succ).color(theme.text_primary).size(13.0));
                    ui.add_space(6.0);
                }

                match state.mode {
                    DialogMode::AddLocal => {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 10.0;

                            ui.label(
                                RichText::new(tr(lang, "title_label"))
                                    .strong()
                                    .color(theme.text_primary)
                                    .size(14.0),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut state.title_input)
                                    .desired_width(f32::INFINITY)
                                    .margin(egui::vec2(10.0, 6.0)),
                            );

                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(tr(lang, "initial_content_label"))
                                    .strong()
                                    .color(theme.text_primary)
                                    .size(14.0),
                            );

                            egui::Frame::new()
                                .fill(theme.bg_gutter)
                                .stroke(Stroke::new(1.0, theme.border))
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::symmetric(10, 8))
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut state.content_input)
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(6),
                                    );
                                });
                        });

                        ui.add_space(14.0);
                        ui.separator();
                        ui.add_space(10.0);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if antd_button_primary(ui, tr(lang, "btn_create_local"), &theme)
                                .clicked()
                                && !state.title_input.trim().is_empty()
                            {
                                let new_item = HostItem::new_local(
                                    state.title_input.trim().to_string(),
                                    state.content_input.clone(),
                                );
                                let id = new_item.id.clone();
                                config.hosts.push(new_item);
                                config.selected_host_id = Some(id);
                                let _ = save_config(config);
                                if config.auto_apply_on_change {
                                    *apply_system_hosts_needed = true;
                                }
                                state.close();
                            }
                            ui.add_space(10.0);
                            if antd_button_default(ui, tr(lang, "btn_cancel"), &theme).clicked() {
                                state.close();
                            }
                        });
                    }
                    DialogMode::AddRemote => {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 10.0;

                            ui.label(
                                RichText::new(tr(lang, "title_label"))
                                    .strong()
                                    .color(theme.text_primary)
                                    .size(14.0),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut state.title_input)
                                    .desired_width(f32::INFINITY)
                                    .margin(egui::vec2(10.0, 6.0)),
                            );

                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(tr(lang, "remote_url_modal_label"))
                                    .strong()
                                    .color(theme.text_primary)
                                    .size(14.0),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut state.url_input)
                                    .hint_text("https://example.com/hosts.txt")
                                    .desired_width(f32::INFINITY)
                                    .margin(egui::vec2(10.0, 6.0)),
                            );

                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(tr(lang, "sync_interval_label"))
                                        .strong()
                                        .color(theme.text_primary)
                                        .size(14.0),
                                );
                                ui.add_space(12.0);
                                egui::ComboBox::from_id_salt("custom_modal_interval")
                                    .selected_text(state.interval_input.display_name(lang))
                                    .show_ui(ui, |ui| {
                                        for opt in UpdateInterval::all() {
                                            ui.selectable_value(
                                                &mut state.interval_input,
                                                opt.clone(),
                                                opt.display_name(lang),
                                            );
                                        }
                                    });
                            });
                        });

                        ui.add_space(14.0);
                        ui.separator();
                        ui.add_space(10.0);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if antd_button_primary(ui, tr(lang, "btn_create_remote"), &theme)
                                .clicked()
                                && !state.title_input.trim().is_empty()
                                && !state.url_input.trim().is_empty()
                            {
                                let new_item = HostItem::new_remote(
                                    state.title_input.trim().to_string(),
                                    state.url_input.trim().to_string(),
                                    state.interval_input.clone(),
                                );
                                let id = new_item.id.clone();
                                config.hosts.push(new_item);
                                config.selected_host_id = Some(id.clone());
                                let _ = save_config(config);
                                *on_remote_added = Some(id);
                                state.close();
                            }
                            ui.add_space(10.0);
                            if antd_button_default(ui, tr(lang, "btn_cancel"), &theme).clicked() {
                                state.close();
                            }
                        });
                    }
                    DialogMode::ImportHosts => {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 8.0;

                            // Source file path display
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(tr(lang, "import_source_file"))
                                        .strong()
                                        .color(theme.text_primary)
                                        .size(13.5),
                                );
                                ui.label(
                                    RichText::new(&state.import_file_name)
                                        .color(theme.text_secondary)
                                        .size(13.5),
                                );
                            });

                            ui.add_space(2.0);

                            // Mode selection (Create New vs Overwrite Local)
                            ui.label(
                                RichText::new(tr(lang, "import_mode_label"))
                                    .strong()
                                    .color(theme.text_primary)
                                    .size(13.5),
                            );

                            let local_hosts: Vec<(String, String)> = config
                                .hosts
                                .iter()
                                .filter(|h| matches!(h.host_type, HostType::Local))
                                .map(|h| (h.id.clone(), h.title.clone()))
                                .collect();

                            let has_local_hosts = !local_hosts.is_empty();

                            ui.horizontal(|ui| {
                                ui.selectable_value(
                                    &mut state.import_mode,
                                    ImportMode::CreateNew,
                                    tr(lang, "import_mode_new"),
                                );
                                ui.add_space(12.0);

                                let overwrite_radio = ui.add_enabled(
                                    has_local_hosts,
                                    egui::RadioButton::new(
                                        state.import_mode == ImportMode::Overwrite,
                                        tr(lang, "import_mode_overwrite"),
                                    ),
                                );
                                if overwrite_radio.clicked() && has_local_hosts {
                                    state.import_mode = ImportMode::Overwrite;
                                }
                            });

                            match state.import_mode {
                                ImportMode::CreateNew => {
                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(tr(lang, "title_label"))
                                                .strong()
                                                .color(theme.text_primary)
                                                .size(13.5),
                                        );
                                        ui.add(
                                            egui::TextEdit::singleline(
                                                &mut state.import_title_input,
                                            )
                                            .desired_width(f32::INFINITY)
                                            .margin(egui::vec2(8.0, 4.0)),
                                        );
                                    });
                                }
                                ImportMode::Overwrite => {
                                    ui.add_space(4.0);
                                    if has_local_hosts {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(tr(lang, "import_target_label"))
                                                    .strong()
                                                    .color(theme.text_primary)
                                                    .size(13.5),
                                            );

                                            if state.import_target_host_id.is_none() {
                                                state.import_target_host_id =
                                                    local_hosts.first().map(|(id, _)| id.clone());
                                            }

                                            let current_title = local_hosts
                                                .iter()
                                                .find(|(id, _)| {
                                                    Some(id) == state.import_target_host_id.as_ref()
                                                })
                                                .map(|(_, t)| t.as_str())
                                                .unwrap_or("Select host...");

                                            egui::ComboBox::from_id_salt(
                                                "import_target_local_combo",
                                            )
                                            .selected_text(
                                                RichText::new(current_title)
                                                    .color(theme.text_primary),
                                            )
                                            .show_ui(
                                                ui,
                                                |ui| {
                                                    for (id, title) in &local_hosts {
                                                        if ui
                                                            .selectable_label(
                                                                state
                                                                    .import_target_host_id
                                                                    .as_ref()
                                                                    == Some(id),
                                                                title,
                                                            )
                                                            .clicked()
                                                        {
                                                            state.import_target_host_id =
                                                                Some(id.clone());
                                                        }
                                                    }
                                                },
                                            );
                                        });
                                    } else {
                                        ui.label(
                                            RichText::new(tr(lang, "no_local_hosts_to_overwrite"))
                                                .color(theme.danger)
                                                .size(12.0),
                                        );
                                    }
                                }
                            }

                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(tr(lang, "preview_hosts_snippet"))
                                    .strong()
                                    .color(theme.text_primary)
                                    .size(13.5),
                            );

                            egui::Frame::new()
                                .fill(theme.bg_gutter)
                                .stroke(Stroke::new(1.0, theme.border))
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::symmetric(8, 6))
                                .show(ui, |ui| {
                                    let mut preview_str =
                                        safe_truncate_str(&state.import_content, 800);
                                    ui.add(
                                        egui::TextEdit::multiline(&mut preview_str)
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(5),
                                    );
                                });
                        });

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if antd_button_primary(ui, tr(lang, "btn_confirm_import"), &theme)
                                .clicked()
                            {
                                match state.import_mode {
                                    ImportMode::CreateNew => {
                                        let title = if state.import_title_input.trim().is_empty() {
                                            "Imported Hosts".to_string()
                                        } else {
                                            state.import_title_input.trim().to_string()
                                        };

                                        match parse_and_import_hosts_custom(
                                            &state.import_content,
                                            title,
                                            config,
                                        ) {
                                            Ok(_count) => {
                                                let _ = save_config(config);
                                                if config.auto_apply_on_change {
                                                    *apply_system_hosts_needed = true;
                                                }
                                                state.close();
                                            }
                                            Err(e) => {
                                                state.error_msg = Some(e);
                                            }
                                        }
                                    }
                                    ImportMode::Overwrite => {
                                        if let Some(target_id) = &state.import_target_host_id {
                                            if let Some(host) =
                                                config.hosts.iter_mut().find(|h| &h.id == target_id)
                                            {
                                                if matches!(host.host_type, HostType::Local) {
                                                    let text_to_set =
                                                        extract_hosts_text(&state.import_content);
                                                    host.content = text_to_set;
                                                    host.updated_at = chrono::Utc::now();
                                                    config.selected_host_id = Some(host.id.clone());
                                                    let _ = save_config(config);
                                                    if config.auto_apply_on_change {
                                                        *apply_system_hosts_needed = true;
                                                    }
                                                    state.close();
                                                } else {
                                                    state.error_msg = Some(
                                                        "只能覆盖本地类型的配置！".to_string(),
                                                    );
                                                }
                                            } else {
                                                state.error_msg =
                                                    Some("目标配置不存在".to_string());
                                            }
                                        } else {
                                            state.error_msg =
                                                Some("请选择要覆盖的本地配置".to_string());
                                        }
                                    }
                                }
                            }
                            ui.add_space(10.0);
                            if antd_button_default(ui, tr(lang, "btn_cancel"), &theme).clicked() {
                                state.close();
                            }
                        });
                    }
                    DialogMode::ExportHosts => {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 10.0;

                            // Scope radio selection (All vs Specific)
                            ui.label(
                                RichText::new(tr(lang, "export_target_label"))
                                    .strong()
                                    .color(theme.text_primary)
                                    .size(13.5),
                            );

                            ui.horizontal(|ui| {
                                if ui
                                    .selectable_label(
                                        state.export_target == ExportTarget::All,
                                        tr(lang, "export_target_all"),
                                    )
                                    .clicked()
                                {
                                    state.export_target = ExportTarget::All;
                                }
                                ui.add_space(12.0);
                                let is_specific =
                                    matches!(state.export_target, ExportTarget::Specific(_));
                                if ui
                                    .selectable_label(
                                        is_specific,
                                        tr(lang, "export_target_specific"),
                                    )
                                    .clicked()
                                    && !is_specific
                                {
                                    if let Some(first) = config.hosts.first() {
                                        state.export_target =
                                            ExportTarget::Specific(first.id.clone());
                                    }
                                }
                            });

                            if let ExportTarget::Specific(selected_id) = &mut state.export_target {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(tr(lang, "export_select_host"))
                                            .strong()
                                            .color(theme.text_primary)
                                            .size(13.5),
                                    );

                                    let cur_host =
                                        config.hosts.iter().find(|h| &h.id == selected_id);
                                    let cur_title = cur_host
                                        .map(|h| h.title.as_str())
                                        .unwrap_or("Select host...");

                                    egui::ComboBox::from_id_salt("export_specific_host_combo")
                                        .selected_text(
                                            RichText::new(cur_title).color(theme.text_primary),
                                        )
                                        .show_ui(ui, |ui| {
                                            for h in &config.hosts {
                                                let icon = match &h.host_type {
                                                    HostType::System => "💻",
                                                    HostType::Local => "📄",
                                                    HostType::Remote { .. } => "🌐",
                                                };
                                                let label = format!("{} {}", icon, h.title);
                                                if ui
                                                    .selectable_label(selected_id == &h.id, label)
                                                    .clicked()
                                                {
                                                    *selected_id = h.id.clone();
                                                }
                                            }
                                        });
                                });
                            }

                            ui.add_space(4.0);

                            // Export Format (Hosts Text vs JSON)
                            ui.label(
                                RichText::new(tr(lang, "export_format_label"))
                                    .strong()
                                    .color(theme.text_primary)
                                    .size(13.5),
                            );
                            ui.horizontal(|ui| {
                                ui.selectable_value(
                                    &mut state.export_format,
                                    ExportFormat::HostsText,
                                    tr(lang, "export_format_hosts"),
                                );
                                ui.add_space(12.0);
                                ui.selectable_value(
                                    &mut state.export_format,
                                    ExportFormat::Json,
                                    tr(lang, "export_format_json"),
                                );
                            });

                            ui.add_space(4.0);

                            // Export Preview
                            let export_content = get_export_content(
                                config,
                                &state.export_target,
                                &state.export_format,
                            );
                            let preview_lines = export_content.lines().count();
                            let preview_bytes = export_content.len();

                            ui.label(
                                RichText::new(format!(
                                    "{} ({} lines, {} bytes)",
                                    tr(lang, "export_preview_label"),
                                    preview_lines,
                                    preview_bytes
                                ))
                                .strong()
                                .color(theme.text_primary)
                                .size(13.5),
                            );

                            egui::Frame::new()
                                .fill(theme.bg_gutter)
                                .stroke(Stroke::new(1.0, theme.border))
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::symmetric(8, 6))
                                .show(ui, |ui| {
                                    let mut preview_str = safe_truncate_str(&export_content, 600);
                                    ui.add(
                                        egui::TextEdit::multiline(&mut preview_str)
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(5),
                                    );
                                });
                        });

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if antd_button_primary(ui, tr(lang, "btn_select_export_path"), &theme)
                                .clicked()
                            {
                                let content = get_export_content(
                                    config,
                                    &state.export_target,
                                    &state.export_format,
                                );
                                let ext = match state.export_format {
                                    ExportFormat::HostsText => "txt",
                                    ExportFormat::Json => "json",
                                };
                                let default_name = match &state.export_target {
                                    ExportTarget::All => format!("hosts_all.{}", ext),
                                    ExportTarget::Specific(id) => {
                                        if let Some(h) = config.hosts.iter().find(|h| &h.id == id) {
                                            format!("{}.{}", h.title.replace(' ', "_"), ext)
                                        } else {
                                            format!("hosts.{}", ext)
                                        }
                                    }
                                };

                                let mut dialog =
                                    rfd::FileDialog::new().set_file_name(&default_name);
                                dialog = match state.export_format {
                                    ExportFormat::HostsText => dialog.add_filter(
                                        "Hosts 文本文件 (*.txt, *.hosts)",
                                        &["txt", "hosts"],
                                    ),
                                    ExportFormat::Json => {
                                        dialog.add_filter("JSON 配置文件 (*.json)", &["json"])
                                    }
                                };

                                if let Some(path) = dialog.save_file() {
                                    match fs::write(&path, content) {
                                        Ok(_) => {
                                            state.close();
                                        }
                                        Err(e) => {
                                            state.error_msg = Some(format!("导出失败: {}", e));
                                        }
                                    }
                                }
                            }
                            ui.add_space(10.0);
                            if antd_button_default(ui, tr(lang, "btn_cancel"), &theme).clicked() {
                                state.close();
                            }
                        });
                    }
                    _ => {}
                }
            });
    });
}
