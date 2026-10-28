pub mod dialogs;
pub mod editor;
pub mod preview;
pub mod sidebar;
pub mod widgets;

use crate::fetcher::{fetch_remote_content, should_update_remote};
use crate::font_loader::load_cjk_fonts;
use crate::hosts_manager::{build_combined_hosts, write_system_hosts};
use crate::i18n::{tr, Language};
use crate::model::{AppConfig, HostItem, HostType, RailTab};
use crate::single_instance::{SingleInstanceGuard, SingleInstanceMessage};
use crate::storage::{get_system_hosts_path, load_config, save_config};
use crate::ui::widgets::{
    rail_icon_button, switch_hosts_button_default, switch_hosts_button_primary, toggle_switch,
    SwitchHostsTheme,
};
use dialogs::{render_custom_dialog_overlay, DialogState};
use editor::render_editor_view;
use preview::render_preview_view;
use sidebar::render_sidebar;
use std::sync::mpsc::{channel, Receiver, Sender};
use tokio::runtime::Runtime;

pub const MIN_SIDEBAR_WIDTH: f32 = 200.0;
pub const MAX_SIDEBAR_WIDTH: f32 = 450.0;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ActiveTab {
    Editor,
    CombinedPreview,
}

#[allow(dead_code)]
pub enum AsyncMessage {
    RemoteFetchSuccess { id: String, content: String },
    RemoteFetchFailed { id: String, error: String },
    SystemWriteResult { success: bool, msg: String },
    InstanceFocusRequested,
}

pub struct QuickHostsApp {
    pub config: AppConfig,
    pub selected_tab: ActiveTab,
    pub dialog_state: DialogState,
    pub sidebar_search: String,
    pub preview_search: String,
    pub editor_search: crate::ui::editor::EditorSearchState,
    pub status_message: Option<(String, bool)>,
    pub toast: Option<widgets::Toast>,
    pub runtime: Runtime,
    pub tx: Sender<AsyncMessage>,
    pub rx: Receiver<AsyncMessage>,
    pub logs: Vec<String>,
    pub pending_auto_apply_at: Option<std::time::Instant>,
    pub app_icon_texture: Option<egui::TextureHandle>,
    pub _instance_guard: Option<SingleInstanceGuard>,
}

impl QuickHostsApp {
    pub fn show_toast(&mut self, msg: impl Into<String>, kind: widgets::ToastKind) {
        self.toast = Some(widgets::Toast::new(msg, kind));
    }

    pub fn new(
        cc: &eframe::CreationContext<'_>,
        instance_guard: Option<SingleInstanceGuard>,
        focus_rx: Option<Receiver<SingleInstanceMessage>>,
    ) -> Self {
        // Load CJK fonts immediately during startup before the first frame renders
        load_cjk_fonts(&cc.egui_ctx);

        let (tx, rx) = channel();

        if let Some(rx_focus) = focus_rx {
            let ctx = cc.egui_ctx.clone();
            let tx_app = tx.clone();
            let _ = std::thread::Builder::new()
                .name("focus-notifier".to_string())
                .spawn(move || {
                    while let Ok(SingleInstanceMessage::Focus) = rx_focus.recv() {
                        // 1. Restore if minimized
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        // 2. Temporarily set to AlwaysOnTop to ensure window manager raises it to foreground on all platforms
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                            egui::WindowLevel::AlwaysOnTop,
                        ));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                            egui::UserAttentionType::Critical,
                        ));
                        ctx.request_repaint();

                        // 3. Notify application UI to display status banner and log
                        let _ = tx_app.send(AsyncMessage::InstanceFocusRequested);

                        // 4. Reset WindowLevel back to Normal after a short delay so it doesn't stay pinned
                        let ctx_clone = ctx.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_millis(500));
                            ctx_clone.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                                egui::WindowLevel::Normal,
                            ));
                            ctx_clone.request_repaint();
                        });
                    }
                });
        }

        // Use 1 worker thread to eliminate idle thread stacks and minimize memory footprint
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .max_blocking_threads(2)
            .thread_name("quickhosts-async")
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");

        let mut config = load_config();

        if config.selected_host_id.is_none() && !config.hosts.is_empty() {
            config.selected_host_id = Some(config.hosts[0].id.clone());
        }

        let app_icon_texture = image::load_from_memory(include_bytes!(
            "../../assets/icons/64x64/apps/io.github.geekdex.quickhosts.png"
        ))
        .ok()
        .map(|img| {
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            let color_image =
                egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            cc.egui_ctx
                .load_texture("app_logo", color_image, egui::TextureOptions::LINEAR)
        });

        let mut app = Self {
            config,
            selected_tab: ActiveTab::Editor,
            dialog_state: DialogState::default(),
            sidebar_search: String::new(),
            preview_search: String::new(),
            editor_search: crate::ui::editor::EditorSearchState::default(),
            status_message: None,
            toast: None,
            runtime: rt,
            tx,
            rx,
            logs: vec!["[System] QuickHosts initialized successfully.".to_string()],
            pending_auto_apply_at: None,
            app_icon_texture,
            _instance_guard: instance_guard,
        };

        app.trigger_auto_updates_if_needed();
        app
    }

    pub fn trigger_auto_updates_if_needed(&mut self) {
        for host in &mut self.config.hosts {
            if let HostType::Remote {
                url,
                interval,
                last_updated,
                is_fetching,
                ..
            } = &mut host.host_type
            {
                if !*is_fetching && should_update_remote(*last_updated, interval) {
                    *is_fetching = true;
                    let id = host.id.clone();
                    let url_str = url.clone();
                    let tx = self.tx.clone();
                    self.runtime.spawn(async move {
                        match fetch_remote_content(&url_str).await {
                            Ok(content) => {
                                let _ = tx.send(AsyncMessage::RemoteFetchSuccess { id, content });
                            }
                            Err(error) => {
                                let _ = tx.send(AsyncMessage::RemoteFetchFailed { id, error });
                            }
                        }
                    });
                }
            }
        }
    }

    pub fn fetch_single_remote(&mut self, host_id: &str) {
        if let Some(host) = self.config.hosts.iter_mut().find(|h| h.id == host_id) {
            if let HostType::Remote {
                url, is_fetching, ..
            } = &mut host.host_type
            {
                *is_fetching = true;
                let id = host.id.clone();
                let url_str = url.clone();
                let tx = self.tx.clone();
                self.runtime.spawn(async move {
                    match fetch_remote_content(&url_str).await {
                        Ok(content) => {
                            let _ = tx.send(AsyncMessage::RemoteFetchSuccess { id, content });
                        }
                        Err(error) => {
                            let _ = tx.send(AsyncMessage::RemoteFetchFailed { id, error });
                        }
                    }
                });
            }
        }
    }

    pub fn apply_system_hosts(&mut self, notify: bool) {
        let active_hosts: Vec<&HostItem> = self.config.hosts.iter().filter(|h| h.enabled).collect();

        let combined = build_combined_hosts(self.config.master_enabled, &active_hosts);

        match write_system_hosts(&combined) {
            Ok(_) => {
                let msg = "✅ System hosts updated successfully!".to_string();
                self.status_message = Some((msg.clone(), false));
                self.logs.push(format!(
                    "[{}] {}",
                    chrono::Local::now().format("%H:%M:%S"),
                    msg
                ));
                if notify {
                    self.show_toast(
                        tr(&self.config.language, "toast_apply_success"),
                        widgets::ToastKind::Success,
                    );
                }
            }
            Err(err) => {
                let msg = format!("❌ Failed to write system hosts: {}", err);
                self.status_message = Some((msg.clone(), true));
                self.logs.push(format!(
                    "[{}] {}",
                    chrono::Local::now().format("%H:%M:%S"),
                    msg
                ));
                if notify {
                    self.show_toast(
                        format!(
                            "{}: {}",
                            tr(&self.config.language, "toast_apply_failed"),
                            err
                        ),
                        widgets::ToastKind::Error,
                    );
                }
            }
        }
    }

    pub fn process_async_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                AsyncMessage::RemoteFetchSuccess { id, content } => {
                    if let Some(host) = self.config.hosts.iter_mut().find(|h| h.id == id) {
                        host.content = content;
                        if let HostType::Remote {
                            ref mut last_updated,
                            ref mut last_error,
                            ref mut is_fetching,
                            ..
                        } = host.host_type
                        {
                            *last_updated = Some(chrono::Utc::now());
                            *last_error = None;
                            *is_fetching = false;
                        }
                        let log_entry = format!(
                            "[{}] Remote host '{}' updated.",
                            chrono::Local::now().format("%H:%M:%S"),
                            host.title
                        );
                        self.logs.push(log_entry);
                    }
                    self.status_message = Some(("🌐 Remote hosts updated!".to_string(), false));
                    self.show_toast(
                        tr(&self.config.language, "toast_remote_fetch_success"),
                        widgets::ToastKind::Success,
                    );
                    let _ = save_config(&self.config);
                    if self.config.auto_apply_on_change {
                        self.apply_system_hosts(false);
                    }
                }
                AsyncMessage::RemoteFetchFailed { id, error } => {
                    if let Some(host) = self.config.hosts.iter_mut().find(|h| h.id == id) {
                        if let HostType::Remote {
                            ref mut last_error,
                            ref mut is_fetching,
                            ..
                        } = host.host_type
                        {
                            *last_error = Some(error.clone());
                            *is_fetching = false;
                        }
                        let log_entry = format!(
                            "[{}] Fetch error for '{}': {}",
                            chrono::Local::now().format("%H:%M:%S"),
                            host.title,
                            error
                        );
                        self.logs.push(log_entry);
                    }
                    self.status_message = Some((format!("⚠️ Remote fetch error: {}", error), true));
                    self.show_toast(
                        format!(
                            "{}: {}",
                            tr(&self.config.language, "toast_remote_fetch_failed"),
                            error
                        ),
                        widgets::ToastKind::Error,
                    );
                }
                AsyncMessage::SystemWriteResult { success, msg } => {
                    self.status_message = Some((msg.clone(), !success));
                    self.show_toast(
                        msg,
                        if success {
                            widgets::ToastKind::Success
                        } else {
                            widgets::ToastKind::Error
                        },
                    );
                }
                AsyncMessage::InstanceFocusRequested => {
                    let msg = tr(&self.config.language, "already_running_desc").to_string();
                    self.status_message = Some((format!("ℹ️ {}", msg), false));
                    self.show_toast(msg.clone(), widgets::ToastKind::Info);
                    self.logs.push(format!(
                        "[{}] {}",
                        chrono::Local::now().format("%H:%M:%S"),
                        msg
                    ));
                }
            }
        }
    }
}

impl eframe::App for QuickHostsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Command+W (macOS) / Ctrl+W (Windows/Linux) shortcut to close window or dialog
        let close_requested = ui.input(|i| i.viewport().close_requested())
            || ui.input_mut(|i| {
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::W)
                    || i.consume_key(egui::Modifiers::MAC_CMD, egui::Key::W)
                    || i.consume_key(egui::Modifiers::CTRL, egui::Key::W)
            });

        if close_requested {
            if self.dialog_state.mode != dialogs::DialogMode::None {
                self.dialog_state.close();
            } else {
                let _ = save_config(&self.config);
                drop(self._instance_guard.take());
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                #[cfg(target_os = "macos")]
                {
                    std::process::exit(0);
                }
            }
        }

        // Alt+ArrowUp / Alt+ArrowDown to move selected host up/down
        let (move_up_shortcut, move_down_shortcut) = ui.input_mut(|i| {
            let up = i.consume_key(egui::Modifiers::ALT, egui::Key::ArrowUp);
            let down = i.consume_key(egui::Modifiers::ALT, egui::Key::ArrowDown);
            (up, down)
        });

        if move_up_shortcut {
            if let Some(selected_id) = &self.config.selected_host_id {
                if let Some(pos) = self.config.hosts.iter().position(|h| &h.id == selected_id) {
                    if pos > 0 {
                        self.config.hosts.swap(pos, pos - 1);
                        let _ = save_config(&self.config);
                        if self.config.auto_apply_on_change {
                            self.apply_system_hosts(false);
                        }
                    }
                }
            }
        }

        if move_down_shortcut {
            if let Some(selected_id) = &self.config.selected_host_id {
                if let Some(pos) = self.config.hosts.iter().position(|h| &h.id == selected_id) {
                    if pos + 1 < self.config.hosts.len() {
                        self.config.hosts.swap(pos, pos + 1);
                        let _ = save_config(&self.config);
                        if self.config.auto_apply_on_change {
                            self.apply_system_hosts(false);
                        }
                    }
                }
            }
        }

        // Command+M (macOS) / Ctrl+M shortcut to minimize window
        let minimize_requested = ui.input_mut(|i| {
            i.consume_key(egui::Modifiers::COMMAND, egui::Key::M)
                || i.consume_key(egui::Modifiers::MAC_CMD, egui::Key::M)
        });

        if minimize_requested {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }

        // Auto-dismiss expired toast
        if let Some(t) = &self.toast {
            if t.is_expired() {
                self.toast = None;
            }
        }

        self.process_async_messages();

        if let Some(instant) = self.pending_auto_apply_at {
            if instant.elapsed() >= std::time::Duration::from_millis(1200) {
                self.apply_system_hosts(false);
                self.pending_auto_apply_at = None;
            } else {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(150));
            }
        }

        let dark_mode = self.config.dark_mode;
        let theme = SwitchHostsTheme::new(dark_mode);

        let mut visuals = if dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };

        visuals.window_corner_radius = egui::CornerRadius::same(8);
        visuals.panel_fill = theme.bg_editor;
        visuals.window_fill = theme.bg_elevated;
        visuals.window_stroke = egui::Stroke::new(1.0, theme.border);
        visuals.menu_corner_radius = egui::CornerRadius::same(6);
        visuals.extreme_bg_color = theme.bg_gutter;
        visuals.selection.bg_fill = if dark_mode {
            egui::Color32::from_rgba_unmultiplied(59, 130, 246, 85)
        } else {
            egui::Color32::from_rgba_unmultiplied(22, 119, 255, 70)
        };
        visuals.selection.stroke = egui::Stroke::new(1.0, theme.primary);

        // Static non-interactive elements
        visuals.widgets.noninteractive.bg_fill = theme.bg_editor;
        visuals.widgets.noninteractive.weak_bg_fill = egui::Color32::TRANSPARENT;
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, theme.text_secondary);

        // Inactive widgets (normal state)
        visuals.widgets.inactive.bg_fill = theme.bg_container;
        visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, theme.border);
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, theme.text_primary);

        // Hovered widgets
        visuals.widgets.hovered.bg_fill = theme.bg_hover;
        visuals.widgets.hovered.weak_bg_fill = theme.bg_hover;
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, theme.border);
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, theme.text_primary);
        visuals.widgets.hovered.expansion = 0.0;

        // Active / pressed widgets
        visuals.widgets.active.bg_fill = theme.bg_hover;
        visuals.widgets.active.weak_bg_fill = theme.bg_hover;
        visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, theme.primary);
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, theme.text_primary);
        visuals.widgets.active.expansion = 0.0;

        // Open menus / dropdowns
        visuals.widgets.open.bg_fill = theme.bg_hover;
        visuals.widgets.open.weak_bg_fill = theme.bg_hover;
        visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, theme.primary);
        visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, theme.text_primary);

        visuals.override_text_color = Some(theme.text_primary);

        ui.ctx().set_visuals(visuals);

        let lang = self.config.language.clone();

        // ==========================================
        //  Pane 1: Left Navigation Rail (Width: 44px)
        // ==========================================
        egui::Panel::left("nav_rail")
            .resizable(false)
            .default_size(44.0)
            .frame(
                egui::Frame::new()
                    .fill(theme.bg_rail)
                    .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                    .inner_margin(egui::Margin::symmetric(4, 8)),
            )
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(2.0);

                    if let Some(texture) = &self.app_icon_texture {
                        ui.add(
                            egui::Image::new(texture)
                                .fit_to_exact_size(egui::vec2(28.0, 28.0))
                                .corner_radius(egui::CornerRadius::same(6)),
                        )
                        .on_hover_text("QuickHosts");
                        ui.add_space(8.0);
                    } else {
                        ui.add_space(4.0);
                    }

                    // 1. Hosts List Icon (📋)
                    if rail_icon_button(
                        ui,
                        "📋",
                        tr(&lang, "rail_hosts"),
                        self.config.active_rail_tab == RailTab::Hosts,
                        &theme,
                    )
                    .clicked()
                    {
                        self.config.active_rail_tab = RailTab::Hosts;
                    }

                    ui.add_space(8.0);

                    // 2. Trash / Recycle Bin Icon (🗑)
                    if rail_icon_button(
                        ui,
                        "🗑",
                        tr(&lang, "rail_trash"),
                        self.config.active_rail_tab == RailTab::Trash,
                        &theme,
                    )
                    .clicked()
                    {
                        self.config.active_rail_tab = RailTab::Trash;
                    }

                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        ui.add_space(8.0);

                        // Settings Icon (⚙)
                        if rail_icon_button(
                            ui,
                            "⚙",
                            tr(&lang, "rail_settings"),
                            self.config.active_rail_tab == RailTab::Settings,
                            &theme,
                        )
                        .clicked()
                        {
                            self.config.active_rail_tab = RailTab::Settings;
                        }

                        ui.add_space(8.0);

                        // History Icon (⏱)
                        if rail_icon_button(
                            ui,
                            "⏱",
                            tr(&lang, "rail_history"),
                            self.config.active_rail_tab == RailTab::History,
                            &theme,
                        )
                        .clicked()
                        {
                            self.config.active_rail_tab = RailTab::History;
                        }

                        ui.add_space(8.0);

                        // Light / Dark Theme Quick Toggle (🌙/☀️)
                        let theme_icon = if dark_mode { "🌙" } else { "☀️" };
                        if rail_icon_button(ui, theme_icon, "Toggle Theme", false, &theme).clicked()
                        {
                            self.config.dark_mode = !self.config.dark_mode;
                            let _ = save_config(&self.config);
                        }
                    });
                });
            });

        // ==========================================
        //  Pane 2: Middle Column (Hosts List Sidebar)
        // ==========================================
        if self.config.active_rail_tab == RailTab::Hosts {
            let current_sidebar_width = self
                .config
                .sidebar_width
                .clamp(MIN_SIDEBAR_WIDTH, MAX_SIDEBAR_WIDTH);

            let sidebar_response = egui::Panel::left("middle_hosts_sidebar")
                .resizable(false)
                .exact_size(current_sidebar_width)
                .frame(
                    egui::Frame::new()
                        .fill(theme.bg_sidebar)
                        .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                        .inner_margin(egui::Margin::symmetric(8, 0)),
                )
                .show(ui, |ui| {
                    let action = render_sidebar(
                        ui,
                        &mut self.config.hosts,
                        &self.config.selected_host_id,
                        &mut self.sidebar_search,
                        &lang,
                        dark_mode,
                    );

                    if let Some(id) = action.selected_id {
                        if self.config.selected_host_id.as_deref() != Some(&id) {
                            self.editor_search.current_match_idx = 0;
                            self.editor_search.scroll_to_target_line = None;
                        }
                        self.config.selected_host_id = Some(id);
                        let _ = save_config(&self.config);
                    }
                    if action.open_add_local {
                        self.dialog_state.open_add_local();
                    }
                    if action.open_add_remote {
                        self.dialog_state.open_add_remote();
                    }
                    if action.trigger_export_hosts_file {
                        self.dialog_state
                            .open_export(self.config.selected_host_id.clone());
                    }
                    if action.trigger_import_hosts_file {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Hosts / JSON 文件", &["json", "txt", "hosts"])
                            .pick_file()
                        {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                let file_name = path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string();
                                self.dialog_state.open_import(
                                    path.display().to_string(),
                                    file_name,
                                    content,
                                    &self.config,
                                );
                            }
                        }
                    }
                    if let Some(id) = action.move_up_id {
                        if let Some(pos) = self.config.hosts.iter().position(|h| h.id == id) {
                            if pos > 0 {
                                self.config.hosts.swap(pos, pos - 1);
                                self.config.selected_host_id = Some(id);
                                let _ = save_config(&self.config);
                                if self.config.auto_apply_on_change {
                                    self.apply_system_hosts(false);
                                }
                            }
                        }
                    }
                    if let Some(id) = action.move_down_id {
                        if let Some(pos) = self.config.hosts.iter().position(|h| h.id == id) {
                            if pos + 1 < self.config.hosts.len() {
                                self.config.hosts.swap(pos, pos + 1);
                                self.config.selected_host_id = Some(id);
                                let _ = save_config(&self.config);
                                if self.config.auto_apply_on_change {
                                    self.apply_system_hosts(false);
                                }
                            }
                        }
                    }
                    if let Some((from_idx, to_idx)) = action.reorder {
                        if from_idx < self.config.hosts.len()
                            && to_idx < self.config.hosts.len()
                            && from_idx != to_idx
                        {
                            let item = self.config.hosts.remove(from_idx);
                            self.config.hosts.insert(to_idx, item);
                            let _ = save_config(&self.config);
                            if self.config.auto_apply_on_change {
                                self.apply_system_hosts(false);
                            }
                        }
                    }
                    if action.state_changed {
                        let _ = save_config(&self.config);
                        if self.config.auto_apply_on_change {
                            self.apply_system_hosts(false);
                        }
                    }
                });

            // Handle custom resize drag interaction on the right edge of sidebar
            let panel_rect = sidebar_response.response.rect;
            let splitter_rect = egui::Rect::from_min_max(
                egui::pos2(panel_rect.max.x - 3.0, panel_rect.min.y),
                egui::pos2(panel_rect.max.x + 3.0, panel_rect.max.y),
            );

            let splitter_id = ui.make_persistent_id("middle_hosts_sidebar_splitter");
            let splitter_interact = ui.interact(splitter_rect, splitter_id, egui::Sense::drag());

            if splitter_interact.hovered() || splitter_interact.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }

            if splitter_interact.dragged() {
                let delta_x = splitter_interact.drag_delta().x;
                let new_width = (self.config.sidebar_width + delta_x)
                    .clamp(MIN_SIDEBAR_WIDTH, MAX_SIDEBAR_WIDTH);
                if (new_width - self.config.sidebar_width).abs() > 0.01 {
                    self.config.sidebar_width = new_width;
                    let _ = save_config(&self.config);
                }
            }

            // Draw clean subtle highlight line when hovering/dragging splitter
            if splitter_interact.hovered() || splitter_interact.dragged() {
                let stroke_color = if splitter_interact.dragged() {
                    theme.primary
                } else {
                    theme.primary.linear_multiply(0.6)
                };
                ui.painter().vline(
                    panel_rect.max.x,
                    panel_rect.y_range(),
                    egui::Stroke::new(2.0, stroke_color),
                );
            }
        }

        // ==========================================
        //  Pane 3: Right Column (Main View Area)
        // ==========================================
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme.bg_editor)
                    .inner_margin(egui::Margin::symmetric(12, 8))
            )
            .show(ui, |ui| {
                match self.config.active_rail_tab {
                    RailTab::Hosts => {
                        let active_hosts: Vec<&HostItem> = self
                            .config
                            .hosts
                            .iter()
                            .filter(|h| h.enabled)
                            .collect();

                        let combined_content = build_combined_hosts(self.config.master_enabled, &active_hosts);

                        match self.selected_tab {
                            ActiveTab::Editor => {
                                let mut content_changed = false;
                                let mut title_changed = false;
                                let mut fetch_remote_requested = false;
                                let mut delete_requested = false;
                                let mut toggle_preview_requested = false;
                                let mut export_requested = false;
                                let mut current_host_id: Option<String> = None;

                                if let Some(sel_id) = self.config.selected_host_id.clone() {
                                    if let Some(host) = self.config.hosts.iter_mut().find(|h| h.id == sel_id) {
                                        current_host_id = Some(host.id.clone());
                                        let resp = render_editor_view(ui, host, &mut self.editor_search, 13.5, &lang, dark_mode);
                                        content_changed = resp.content_changed;
                                        title_changed = resp.title_changed;
                                        fetch_remote_requested = resp.fetch_remote_requested;
                                        delete_requested = resp.delete_requested;
                                        toggle_preview_requested = resp.toggle_preview_requested;
                                        export_requested = resp.export_requested;
                                    } else {
                                        ui.centered_and_justified(|ui| {
                                            ui.label(egui::RichText::new("Select a host profile from the left sidebar").color(theme.text_tertiary));
                                        });
                                    }
                                } else {
                                    ui.centered_and_justified(|ui| {
                                        ui.label(egui::RichText::new("Select a host profile from the left sidebar").color(theme.text_tertiary));
                                    });
                                }

                                if export_requested {
                                    if let Some(hid) = current_host_id.clone() {
                                        self.dialog_state.open_export(Some(hid));
                                    }
                                }

                                if toggle_preview_requested {
                                    self.selected_tab = ActiveTab::CombinedPreview;
                                }

                                if content_changed || title_changed {
                                    let _ = save_config(&self.config);
                                    if self.config.auto_apply_on_change {
                                        self.pending_auto_apply_at = Some(std::time::Instant::now());
                                    }
                                }

                                if fetch_remote_requested {
                                    if let Some(hid) = current_host_id.clone() {
                                        self.fetch_single_remote(&hid);
                                    }
                                }

                                if delete_requested {
                                    if let Some(did) = current_host_id {
                                        if let Some(pos) = self.config.hosts.iter().position(|h| h.id == did) {
                                            let removed = self.config.hosts.remove(pos);
                                            self.config.trash.push(removed);
                                        }
                                        self.config.selected_host_id = self.config.hosts.first().map(|h| h.id.clone());
                                        let _ = save_config(&self.config);
                                        if self.config.auto_apply_on_change {
                                            self.apply_system_hosts(false);
                                        }
                                    }
                                }
                            }
                            ActiveTab::CombinedPreview => {
                                ui.horizontal(|ui| {
                                    if ui.button("◀ Back to Editor").clicked() {
                                        self.selected_tab = ActiveTab::Editor;
                                    }
                                });
                                ui.add_space(4.0);

                                let action = render_preview_view(
                                    ui,
                                    &combined_content,
                                    &active_hosts,
                                    self.config.master_enabled,
                                    &mut self.preview_search,
                                    &lang,
                                    dark_mode,
                                );

                                if action.apply_to_system {
                                    self.apply_system_hosts(true);
                                }
                                if action.copy_to_clipboard {
                                    if let Ok(mut cb) = arboard::Clipboard::new() {
                                        let _ = cb.set_text(combined_content);
                                        self.status_message = Some(("Combined system hosts copied to clipboard!".to_string(), false));
                                        self.show_toast(
                                            tr(&lang, "toast_copy_all_success"),
                                            widgets::ToastKind::Success,
                                        );
                                    }
                                }
                            }
                        }
                    }
                    RailTab::Trash => {
                        ui.vertical(|ui| {
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("🗑").size(20.0));
                                ui.label(egui::RichText::new(tr(&lang, "rail_trash")).font(egui::FontId::proportional(18.0)).strong());

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if !self.config.trash.is_empty()
                                        && switch_hosts_button_default(ui, tr(&lang, "btn_empty_trash"), &theme).clicked()
                                    {
                                        self.config.trash.clear();
                                        let _ = save_config(&self.config);
                                    }
                                });
                            });

                            ui.add_space(8.0);

                            if self.config.trash.is_empty() {
                                ui.centered_and_justified(|ui| {
                                    ui.label(egui::RichText::new(tr(&lang, "trash_empty")).color(theme.text_tertiary).size(15.0));
                                });
                            } else {
                                let mut restore_id = None;
                                let mut delete_perm_id = None;

                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    for item in &self.config.trash {
                                        egui::Frame::new()
                                            .fill(theme.bg_container)
                                            .stroke(egui::Stroke::new(1.0, theme.border))
                                            .corner_radius(egui::CornerRadius::same(6))
                                            .inner_margin(egui::Margin::symmetric(12, 10))
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new(&item.title).strong());
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        if switch_hosts_button_primary(ui, tr(&lang, "btn_restore"), &theme).clicked() {
                                                            restore_id = Some(item.id.clone());
                                                        }
                                                        ui.add_space(8.0);
                                                        if switch_hosts_button_default(ui, tr(&lang, "btn_delete_perm"), &theme).clicked() {
                                                            delete_perm_id = Some(item.id.clone());
                                                        }
                                                    });
                                                });
                                            });
                                        ui.add_space(6.0);
                                    }
                                });

                                if let Some(id) = restore_id {
                                    if let Some(pos) = self.config.trash.iter().position(|h| h.id == id) {
                                        let item = self.config.trash.remove(pos);
                                        self.config.hosts.push(item);
                                        let _ = save_config(&self.config);
                                    }
                                }
                                if let Some(id) = delete_perm_id {
                                    self.config.trash.retain(|h| h.id != id);
                                    let _ = save_config(&self.config);
                                }
                            }
                        });
                    }
                    RailTab::History => {
                        ui.vertical(|ui| {
                            ui.add_space(8.0);

                            // Log Header Bar with actions
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(tr(&lang, "history_title"))
                                        .font(egui::FontId::proportional(18.0))
                                        .strong()
                                        .color(theme.text_primary),
                                );
                                ui.add_space(8.0);
                                crate::ui::widgets::antd_badge(
                                    ui,
                                    &format!("{} 条记录", self.logs.len()),
                                    theme.info_bg,
                                    theme.text_primary,
                                );

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if switch_hosts_button_default(ui, "🗑️ 清空日志", &theme).clicked() {
                                        self.logs.clear();
                                        self.logs.push(format!("[{}] 日志已清空", chrono::Local::now().format("%H:%M:%S")));
                                    }
                                    ui.add_space(8.0);
                                    if switch_hosts_button_default(ui, "📋 复制日志", &theme).clicked() {
                                        if let Ok(mut cb) = arboard::Clipboard::new() {
                                            let full_log = self.logs.join("\n");
                                            let _ = cb.set_text(full_log);
                                            self.status_message = Some(("日志已复制到剪贴板！".to_string(), false));
                                            self.show_toast(tr(&lang, "toast_log_copied"), widgets::ToastKind::Success);
                                        }
                                    }
                                });
                            });

                            ui.add_space(12.0);

                            // Structured Log Terminal Frame
                            egui::Frame::new()
                                .fill(theme.bg_gutter)
                                .stroke(egui::Stroke::new(1.0, theme.border))
                                .corner_radius(egui::CornerRadius::same(8))
                                .inner_margin(egui::Margin::symmetric(14, 12))
                                .show(ui, |ui| {
                                    if self.logs.is_empty() {
                                        ui.centered_and_justified(|ui| {
                                            ui.label(
                                                egui::RichText::new("暂无系统运行日志")
                                                    .color(theme.text_tertiary)
                                                    .size(14.0),
                                            );
                                        });
                                    } else {
                                        egui::ScrollArea::vertical()
                                            .auto_shrink([false; 2])
                                            .show(ui, |ui| {
                                                for log in &self.logs {
                                                    ui.horizontal(|ui| {
                                                        if log.contains("✅") || log.contains("successfully") || log.contains("updated") {
                                                            ui.label(egui::RichText::new("🟢").size(12.0));
                                                            ui.label(
                                                                egui::RichText::new(log)
                                                                    .font(egui::FontId::monospace(13.0))
                                                                    .color(theme.success),
                                                            );
                                                        } else if log.contains("❌") || log.contains("Failed") || log.contains("error") {
                                                            ui.label(egui::RichText::new("🔴").size(12.0));
                                                            ui.label(
                                                                egui::RichText::new(log)
                                                                    .font(egui::FontId::monospace(13.0))
                                                                    .color(theme.danger),
                                                            );
                                                        } else if log.contains("[System]") || log.contains("initialized") {
                                                            ui.label(egui::RichText::new("⚡").size(12.0));
                                                            ui.label(
                                                                egui::RichText::new(log)
                                                                    .font(egui::FontId::monospace(13.0))
                                                                    .color(theme.text_primary),
                                                            );
                                                        } else {
                                                            ui.label(egui::RichText::new("ℹ️").size(12.0));
                                                            ui.label(
                                                                egui::RichText::new(log)
                                                                    .font(egui::FontId::monospace(13.0))
                                                                    .color(theme.text_secondary),
                                                            );
                                                        }
                                                    });
                                                    ui.add_space(2.0);
                                                }
                                            });
                                    }
                                });
                        });
                    }
                    RailTab::Settings => {
                        ui.vertical(|ui| {
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new(tr(&lang, "settings_title"))
                                    .font(egui::FontId::proportional(18.0))
                                    .strong()
                                    .color(theme.text_primary),
                            );
                            ui.add_space(16.0);

                            egui::ScrollArea::vertical()
                                .auto_shrink([false; 2])
                                .show(ui, |ui| {
                                    // Card 1: Switch & Engine Settings
                                    egui::Frame::new()
                                        .fill(theme.bg_container)
                                        .stroke(egui::Stroke::new(1.0, theme.border))
                                        .corner_radius(egui::CornerRadius::same(8))
                                        .inner_margin(egui::Margin::same(16))
                                        .show(ui, |ui| {
                                            ui.vertical(|ui| {
                                                ui.label(
                                                    egui::RichText::new("⚙️ 全局与同步偏好 / Engine Settings")
                                                        .font(egui::FontId::proportional(14.5))
                                                        .strong()
                                                        .color(theme.text_primary),
                                                );
                                                ui.add_space(12.0);

                                                // 1. Master Switch Setting
                                                ui.horizontal(|ui| {
                                                    ui.vertical(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(tr(&lang, "settings_master"))
                                                                .strong()
                                                                .size(13.5)
                                                                .color(theme.text_primary),
                                                        );
                                                        ui.label(
                                                            egui::RichText::new("控制全局 Hosts 切换引擎开启或关闭")
                                                                .size(11.5)
                                                                .color(theme.text_secondary),
                                                        );
                                                    });
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        if toggle_switch(ui, &mut self.config.master_enabled).changed() {
                                                            let _ = save_config(&self.config);
                                                            if self.config.auto_apply_on_change {
                                                                self.apply_system_hosts(true);
                                                            }
                                                        }
                                                    });
                                                });

                                                ui.add_space(10.0);
                                                ui.painter().hline(
                                                    ui.max_rect().x_range(),
                                                    ui.cursor().min.y,
                                                    egui::Stroke::new(1.0, theme.border_subtle),
                                                );
                                                ui.add_space(10.0);

                                                // 2. Auto Apply Setting
                                                ui.horizontal(|ui| {
                                                    ui.vertical(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(tr(&lang, "settings_auto_apply"))
                                                                .strong()
                                                                .size(13.5)
                                                                .color(theme.text_primary),
                                                        );
                                                        ui.label(
                                                            egui::RichText::new("在开/关方案或修改 Hosts 时自动同步到系统文件")
                                                                .size(11.5)
                                                                .color(theme.text_secondary),
                                                        );
                                                    });
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        if toggle_switch(ui, &mut self.config.auto_apply_on_change).changed() {
                                                            let _ = save_config(&self.config);
                                                        }
                                                    });
                                                });

                                                ui.add_space(10.0);
                                                ui.painter().hline(
                                                    ui.max_rect().x_range(),
                                                    ui.cursor().min.y,
                                                    egui::Stroke::new(1.0, theme.border_subtle),
                                                );
                                                ui.add_space(10.0);

                                                // 3. Language Setting
                                                ui.horizontal(|ui| {
                                                    ui.vertical(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(tr(&lang, "settings_language"))
                                                                .strong()
                                                                .size(13.5)
                                                                .color(theme.text_primary),
                                                        );
                                                        ui.label(
                                                            egui::RichText::new("切换应用展示的多国语言")
                                                                .size(11.5)
                                                                .color(theme.text_secondary),
                                                        );
                                                    });
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        let combo = egui::ComboBox::from_id_salt("settings_lang_combo")
                                                            .selected_text(
                                                                egui::RichText::new(self.config.language.display_name())
                                                                    .color(theme.text_primary)
                                                            )
                                                            .show_ui(ui, |ui| {
                                                                for l in Language::all() {
                                                                    if ui.selectable_value(&mut self.config.language, l.clone(), l.display_name()).clicked() {
                                                                        let _ = save_config(&self.config);
                                                                    }
                                                                }
                                                            });
                                                        if combo.response.hovered() {
                                                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                        }
                                                    });
                                                });
                                            });
                                        });

                                    ui.add_space(14.0);

                                    // Card 2: System Hosts Environment Path
                                    let sys_path = get_system_hosts_path();
                                    egui::Frame::new()
                                        .fill(theme.bg_container)
                                        .stroke(egui::Stroke::new(1.0, theme.border))
                                        .corner_radius(egui::CornerRadius::same(8))
                                        .inner_margin(egui::Margin::same(16))
                                        .show(ui, |ui| {
                                            ui.vertical(|ui| {
                                                ui.label(
                                                    egui::RichText::new("💻 系统 Hosts 文件环境 / System Path")
                                                        .font(egui::FontId::proportional(14.5))
                                                        .strong()
                                                        .color(theme.text_primary),
                                                );
                                                ui.add_space(12.0);

                                                ui.label(
                                                    egui::RichText::new(tr(&lang, "settings_sys_hosts_path"))
                                                        .strong()
                                                        .size(13.5)
                                                        .color(theme.text_primary),
                                                );
                                                ui.add_space(6.0);

                                                egui::Frame::new()
                                                    .fill(theme.bg_gutter)
                                                    .stroke(egui::Stroke::new(1.0, theme.border_subtle))
                                                    .corner_radius(egui::CornerRadius::same(6))
                                                    .inner_margin(egui::Margin::symmetric(12, 8))
                                                    .show(ui, |ui| {
                                                        ui.horizontal(|ui| {
                                                            ui.label(
                                                                egui::RichText::new(sys_path)
                                                                    .color(theme.text_primary)
                                                                    .font(egui::FontId::monospace(13.0)),
                                                            );
                                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                                if switch_hosts_button_default(ui, "复制路径", &theme).clicked() {
                                                                    if let Ok(mut cb) = arboard::Clipboard::new() {
                                                                        let _ = cb.set_text(sys_path);
                                                                        self.status_message = Some(("系统 Hosts 文件路径已复制！".to_string(), false));
                                                                        self.show_toast(tr(&lang, "toast_path_copied"), widgets::ToastKind::Success);
                                                                    }
                                                                }
                                                            });
                                                        });
                                                    });
                                            });
                                        });
                                });
                        });
                    }
                }
            });

        // --- Custom Overlay Dialogs ---
        let mut on_remote_added = None;
        let mut apply_needed = false;

        render_custom_dialog_overlay(
            ui.ctx(),
            &mut self.dialog_state,
            &mut self.config,
            &lang,
            dark_mode,
            &mut on_remote_added,
            &mut apply_needed,
        );

        if let Some(id) = on_remote_added {
            self.fetch_single_remote(&id);
        }
        if apply_needed {
            self.apply_system_hosts(true);
        }

        // --- Global Floating Toast Notifications ---
        widgets::render_toast_overlay(ui.ctx(), &self.toast, &theme);
    }
}
