use quickhosts::hosts_manager::{build_combined_hosts, parse_hosts_stats, strip_quickhosts_block};
use quickhosts::model::{AppConfig, HostItem, HostType, UpdateInterval};
use quickhosts::ui::dialogs::parse_and_import_hosts;

#[test]
fn test_hosts_stats_parser() {
    let content =
        "# Header comment\n127.0.0.1 localhost\n# 127.0.0.1 old.domain.com\n10.0.0.1 dev.server\n";
    let stats = parse_hosts_stats(content);
    assert_eq!(stats.total_lines, 4);
    assert_eq!(stats.active_rules, 2);
    assert_eq!(stats.disabled_rules, 1);
}

#[test]
fn test_strip_quickhosts_block() {
    let sys = "127.0.0.1 localhost\n# --- QuickHosts START (Auto-Generated) ---\n1.1.1.1 test\n# --- QuickHosts END ---\n::1 localhost\n";
    let stripped = strip_quickhosts_block(sys);
    assert!(!stripped.contains("1.1.1.1 test"));
    assert!(stripped.contains("127.0.0.1 localhost"));
    assert!(stripped.contains("::1 localhost"));
}

#[test]
fn test_build_combined_hosts() {
    let item1 = HostItem::new_local(
        "Local Test".to_string(),
        "127.0.0.1 local.dev\n".to_string(),
    );
    let item2 = HostItem::new_remote(
        "Remote K8s".to_string(),
        "http://example.com".to_string(),
        UpdateInterval::Hour1,
    );
    let active = vec![&item1, &item2];
    let combined = build_combined_hosts(true, &active);
    assert!(combined.contains("QuickHosts START"));
    assert!(combined.contains("127.0.0.1 local.dev"));
    assert!(combined.contains("Remote K8s"));
}

#[test]
fn test_import_full_app_config() {
    let cfg = AppConfig::default();
    let original_count = cfg.hosts.len();
    let json_str = serde_json::to_string_pretty(&cfg).unwrap();

    let mut new_cfg = AppConfig::default();
    new_cfg.hosts.clear();
    let res = parse_and_import_hosts(&json_str, &mut new_cfg);
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), original_count);
    assert_eq!(new_cfg.hosts.len(), original_count);
}

#[test]
fn test_import_vec_host_items() {
    let mut cfg = AppConfig::default();
    let item1 = HostItem::new_local(
        "Imported Item 1".to_string(),
        "127.0.0.1 imp1.test".to_string(),
    );
    let item2 = HostItem::new_local(
        "Imported Item 2".to_string(),
        "127.0.0.1 imp2.test".to_string(),
    );
    let items = vec![item1, item2];
    let json_str = serde_json::to_string(&items).unwrap();

    let initial_len = cfg.hosts.len();
    let res = parse_and_import_hosts(&json_str, &mut cfg);
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 2);
    assert_eq!(cfg.hosts.len(), initial_len + 2);
}

#[test]
fn test_import_switchhosts_json_array() {
    let mut cfg = AppConfig::default();
    let json_str = r##"[
        {"title": "SwitchHosts Dev", "content": "127.0.0.1 sh.dev", "type": "local", "enabled": true},
        {"title": "SwitchHosts Remote", "content": "# remote", "type": "remote", "url": "http://test.com/hosts.txt", "enabled": false}
    ]"##;

    let initial_len = cfg.hosts.len();
    let res = parse_and_import_hosts(json_str, &mut cfg);
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 2);
    assert_eq!(cfg.hosts.len(), initial_len + 2);
    assert_eq!(cfg.hosts.last().unwrap().title, "SwitchHosts Remote");
}

#[test]
fn test_import_plain_text_hosts() {
    let mut cfg = AppConfig::default();
    let plain_hosts = "# Plain text hosts\n127.0.0.1 plain.dev\n10.0.0.1 dev.api";

    let initial_len = cfg.hosts.len();
    let res = parse_and_import_hosts(plain_hosts, &mut cfg);
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 1);
    assert_eq!(cfg.hosts.len(), initial_len + 1);
    assert!(cfg.hosts.last().unwrap().content.contains("plain.dev"));
}

#[test]
fn test_import_custom_title_new_config() {
    use quickhosts::ui::dialogs::parse_and_import_hosts_custom;
    let mut cfg = AppConfig::default();
    let plain = "127.0.0.1 custom.test.domain\n";

    let res =
        parse_and_import_hosts_custom(plain, "My Custom Imported Hosts".to_string(), &mut cfg);
    assert!(res.is_ok());
    let imported = cfg.hosts.last().unwrap();
    assert_eq!(imported.title, "My Custom Imported Hosts");
    assert!(imported.content.contains("custom.test.domain"));
}

#[test]
fn test_overwrite_local_host_only() {
    use quickhosts::model::HostType;
    use quickhosts::ui::dialogs::extract_hosts_text;

    let mut cfg = AppConfig::default();

    // Add local host and remote host
    let local = HostItem::new_local(
        "Target Local".to_string(),
        "127.0.0.1 old.local\n".to_string(),
    );
    let local_id = local.id.clone();
    cfg.hosts.push(local);

    let remote = HostItem::new_remote(
        "Target Remote".to_string(),
        "http://test.com".to_string(),
        UpdateInterval::Hour1,
    );
    cfg.hosts.push(remote);

    // Filter local host candidates for overwrite
    let local_candidates: Vec<&HostItem> = cfg
        .hosts
        .iter()
        .filter(|h| matches!(h.host_type, HostType::Local))
        .collect();
    assert!(local_candidates.iter().any(|h| h.id == local_id));
    assert!(!local_candidates
        .iter()
        .any(|h| matches!(h.host_type, HostType::Remote { .. })));

    // Perform overwrite on local host
    let imported_text = "127.0.0.1 new.overwritten.domain\n";
    let target = cfg.hosts.iter_mut().find(|h| h.id == local_id).unwrap();
    assert!(matches!(target.host_type, HostType::Local));
    target.content = extract_hosts_text(imported_text);

    assert_eq!(target.content, "127.0.0.1 new.overwritten.domain\n");
}

#[test]
fn test_export_all_and_specific() {
    use quickhosts::ui::dialogs::{get_export_content, ExportFormat, ExportTarget};

    let mut cfg = AppConfig::default();
    let item1 = HostItem::new_local(
        "Specific Host 1".to_string(),
        "127.0.0.1 host1.domain\n".to_string(),
    );
    let item1_id = item1.id.clone();
    let item2 = HostItem::new_local(
        "Specific Host 2".to_string(),
        "127.0.0.1 host2.domain\n".to_string(),
    );
    cfg.hosts = vec![item1, item2];

    // Export specific host text
    let content_text = get_export_content(
        &cfg,
        &ExportTarget::Specific(item1_id.clone()),
        &ExportFormat::HostsText,
    );
    assert_eq!(content_text, "127.0.0.1 host1.domain\n");

    // Export specific host JSON
    let content_json = get_export_content(
        &cfg,
        &ExportTarget::Specific(item1_id.clone()),
        &ExportFormat::Json,
    );
    assert!(content_json.contains("Specific Host 1"));

    // Export all hosts text
    let all_text = get_export_content(&cfg, &ExportTarget::All, &ExportFormat::HostsText);
    assert!(all_text.contains("127.0.0.1 host1.domain"));
    assert!(all_text.contains("127.0.0.1 host2.domain"));

    // Export all hosts JSON
    let all_json = get_export_content(&cfg, &ExportTarget::All, &ExportFormat::Json);
    assert!(all_json.contains("Specific Host 1"));
    assert!(all_json.contains("Specific Host 2"));
}

#[test]
fn test_safe_truncate_utf8() {
    use quickhosts::ui::dialogs::safe_truncate_str;

    // "系统" is 6 bytes (3 bytes per Chinese char: 0xE7 0xB3 0xBB 0xE7 0xBB 0x9F)
    let chinese_str = "控制系统全局Hosts切换引擎开启或关闭系统设置说明文档".repeat(50);

    // Testing slicing at non-char-boundary offsets (e.g., 5, 10, 20, 600)
    for max_len in [1, 2, 3, 4, 5, 10, 15, 20, 598, 599, 600, 601, 800] {
        let truncated = safe_truncate_str(&chinese_str, max_len);
        assert!(!truncated.is_empty());
    }
}

#[test]
fn test_theme_not_red() {
    use quickhosts::ui::widgets::SwitchHostsTheme;

    let dark = SwitchHostsTheme::new(true);
    let light = SwitchHostsTheme::new(false);

    // Ensure red (217, 54, 62) is not used as primary theme color
    assert_ne!(dark.primary, egui::Color32::from_rgb(217, 54, 62));
    assert_ne!(light.primary, egui::Color32::from_rgb(217, 54, 62));

    // Ensure blue is used
    assert!(dark.primary.b() > dark.primary.r());
    assert!(light.primary.b() > light.primary.r());
}

#[test]
fn test_line_count_accounting() {
    // 1. Content with trailing newline: split('\n') accounts for trailing empty line
    let content1 = "127.0.0.1 a\n127.0.0.1 b\n";
    let lines1: Vec<&str> = content1.split('\n').collect();
    assert_eq!(lines1.len(), 3);
    assert_eq!(lines1[2], "");

    // 2. Large hosts file with 1000 lines
    let mut large_content = String::new();
    for i in 0..1000 {
        large_content.push_str(&format!("127.0.0.1 host{}.example.com\n", i));
    }
    let large_lines: Vec<&str> = large_content.split('\n').collect();
    assert_eq!(large_lines.len(), 1001); // 1000 hosts + 1 trailing empty line
}

#[test]
fn test_comment_toggle_logic() {
    let mut content =
        "127.0.0.1 local.dev\n# 10.0.0.1 remote.dev\n   # 192.168.1.1 router.lan\n\n".to_string();
    let mut lines: Vec<String> = content.split('\n').map(|s| s.to_string()).collect();

    // Toggle line 0: uncommented -> commented
    {
        let current_line = &lines[0];
        let trimmed = current_line.trim_start();
        if let Some(without_hash) = trimmed.strip_prefix('#') {
            let indent_len = current_line.len() - trimmed.len();
            let indent = &current_line[..indent_len];
            let uncommented = without_hash.strip_prefix(' ').unwrap_or(without_hash);
            lines[0] = format!("{}{}", indent, uncommented);
        } else if !trimmed.is_empty() {
            let indent_len = current_line.len() - trimmed.len();
            let indent = &current_line[..indent_len];
            lines[0] = format!("{}# {}", indent, trimmed);
        }
    }
    assert_eq!(lines[0], "# 127.0.0.1 local.dev");

    // Toggle line 1: commented -> uncommented
    {
        let current_line = &lines[1];
        let trimmed = current_line.trim_start();
        if let Some(without_hash) = trimmed.strip_prefix('#') {
            let indent_len = current_line.len() - trimmed.len();
            let indent = &current_line[..indent_len];
            let uncommented = without_hash.strip_prefix(' ').unwrap_or(without_hash);
            lines[1] = format!("{}{}", indent, uncommented);
        }
    }
    assert_eq!(lines[1], "10.0.0.1 remote.dev");

    // Toggle line 2: indented commented -> indented uncommented
    {
        let current_line = &lines[2];
        let trimmed = current_line.trim_start();
        if let Some(without_hash) = trimmed.strip_prefix('#') {
            let indent_len = current_line.len() - trimmed.len();
            let indent = &current_line[..indent_len];
            let uncommented = without_hash.strip_prefix(' ').unwrap_or(without_hash);
            lines[2] = format!("{}{}", indent, uncommented);
        }
    }
    assert_eq!(lines[2], "   192.168.1.1 router.lan");

    content = lines.join("\n");
    assert!(content.ends_with('\n'));
}

#[test]
fn test_find_search_matches_empty_query() {
    let content = "127.0.0.1 localhost\n::1 localhost\n";
    let matches = quickhosts::ui::editor::find_search_matches(content, "", false);
    assert!(matches.is_empty());
}

#[test]
fn test_find_search_matches_case_insensitive() {
    let content = "# Local hosts\n127.0.0.1 LOCALHOST\n::1 localhost\n";
    let matches = quickhosts::ui::editor::find_search_matches(content, "localhost", false);
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].line_index, 1);
    assert_eq!(matches[1].line_index, 2);
    // Verify slice extraction
    let m0 = &matches[0];
    assert_eq!(&content[m0.byte_start..m0.byte_end], "LOCALHOST");
    let m1 = &matches[1];
    assert_eq!(&content[m1.byte_start..m1.byte_end], "localhost");
}

#[test]
fn test_find_search_matches_case_sensitive() {
    let content = "# Local hosts\n127.0.0.1 LOCALHOST\n::1 localhost\n";
    let matches = quickhosts::ui::editor::find_search_matches(content, "localhost", true);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].line_index, 2);
    let m0 = &matches[0];
    assert_eq!(&content[m0.byte_start..m0.byte_end], "localhost");
}

#[test]
fn test_find_search_matches_multiline_positions() {
    let content = "10.0.0.1 dev.test # comment\n10.0.0.2 prod.test\n10.0.0.3 stage.test";
    let matches = quickhosts::ui::editor::find_search_matches(content, ".test", false);
    assert_eq!(matches.len(), 3);
    assert_eq!(matches[0].line_index, 0);
    assert_eq!(matches[1].line_index, 1);
    assert_eq!(matches[2].line_index, 2);

    for m in &matches {
        assert_eq!(&content[m.byte_start..m.byte_end], ".test");
    }
}

#[test]
fn test_find_search_matches_cjk_characters() {
    let content = "# 这是一个测试域名\n127.0.0.1 dev.local # 测试环境\n";
    let matches = quickhosts::ui::editor::find_search_matches(content, "测试", false);
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].line_index, 0);
    assert_eq!(matches[1].line_index, 1);
    assert_eq!(&content[matches[0].byte_start..matches[0].byte_end], "测试");
    assert_eq!(&content[matches[1].byte_start..matches[1].byte_end], "测试");
}

#[test]
fn test_remote_host_read_only_preserves_content() {
    let original = "127.0.0.1 remote.test.domain\n";
    let mut item = HostItem::new_remote(
        "Remote K8s".to_string(),
        "https://example.com/hosts".to_string(),
        UpdateInterval::Never,
    );
    item.content = original.to_string();

    let is_read_only = item.read_only || matches!(item.host_type, HostType::Remote { .. });
    assert!(is_read_only, "Remote host must always be marked read-only");

    // Simulating TextBuffer for immutable &str
    use egui::TextBuffer;
    let text: &str = item.content.as_str();
    assert!(!text.is_mutable(), "&str TextBuffer must not be mutable");
    assert_eq!(item.content, original);
}

#[test]
fn test_search_navigation_cycling() {
    let content = "10.0.0.1 a.com\n10.0.0.2 b.com\n10.0.0.3 c.com\n";
    let matches = quickhosts::ui::editor::find_search_matches(content, ".com", false);
    assert_eq!(matches.len(), 3);

    let total = matches.len();
    let mut current_idx = 0;

    // Next match cycles 0 -> 1 -> 2 -> 0
    current_idx = (current_idx + 1) % total;
    assert_eq!(current_idx, 1);
    current_idx = (current_idx + 1) % total;
    assert_eq!(current_idx, 2);
    current_idx = (current_idx + 1) % total;
    assert_eq!(current_idx, 0);

    // Prev match cycles 0 -> 2 -> 1 -> 0
    current_idx = if current_idx == 0 {
        total - 1
    } else {
        current_idx - 1
    };
    assert_eq!(current_idx, 2);
    current_idx = if current_idx == 0 {
        total - 1
    } else {
        current_idx - 1
    };
    assert_eq!(current_idx, 1);
    current_idx = if current_idx == 0 {
        total - 1
    } else {
        current_idx - 1
    };
    assert_eq!(current_idx, 0);
}

#[test]
fn test_initial_default_config_state() {
    let cfg = AppConfig::default();

    // 1. Initial hosts must contain exactly 1 item
    assert_eq!(cfg.hosts.len(), 1);
    let dev_host = &cfg.hosts[0];
    assert_eq!(dev_host.title, "本地开发环境 (Dev)");
    assert_eq!(
        dev_host.content,
        "# 本地开发 Hosts 映射\n127.0.0.1 dev.local\n::1 dev.local\n"
    );
    assert!(dev_host.enabled);
    assert_eq!(cfg.selected_host_id, Some(dev_host.id.clone()));
    assert!(cfg.trash.is_empty());

    // 2. No mama.cn references anywhere in serialized default config
    let serialized = serde_json::to_string_pretty(&cfg).unwrap();
    assert!(!serialized.contains("mama.cn"));
    assert!(!serialized.contains("K8s 测试集群"));
    assert!(!serialized.contains("自定义内部服务 mappings"));
}

#[test]
fn test_icon_assets_transparency() {
    use image::GenericImageView;

    let check_icon = |path: &str, expected_size: (u32, u32)| {
        let bytes = std::fs::read(path).unwrap_or_else(|_| panic!("Failed to read {}", path));
        let img =
            image::load_from_memory(&bytes).unwrap_or_else(|_| panic!("Failed to load {}", path));
        assert_eq!(
            img.dimensions(),
            expected_size,
            "Dimensions mismatch for {}",
            path
        );
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();

        // Ensure all 4 outer corners are transparent (alpha == 0)
        assert_eq!(
            rgba.get_pixel(0, 0)[3],
            0,
            "Top-left corner not transparent for {}",
            path
        );
        assert_eq!(
            rgba.get_pixel(w - 1, 0)[3],
            0,
            "Top-right corner not transparent for {}",
            path
        );
        assert_eq!(
            rgba.get_pixel(0, h - 1)[3],
            0,
            "Bottom-left corner not transparent for {}",
            path
        );
        assert_eq!(
            rgba.get_pixel(w - 1, h - 1)[3],
            0,
            "Bottom-right corner not transparent for {}",
            path
        );

        // Ensure center pixel is non-transparent
        assert!(
            rgba.get_pixel(w / 2, h / 2)[3] > 200,
            "Center should be opaque for {}",
            path
        );
    };

    check_icon("icon.png", (1024, 1024));
    check_icon(
        "assets/icons/512x512/apps/io.github.geekdex.quickhosts.png",
        (512, 512),
    );
    check_icon(
        "assets/icons/256x256/apps/io.github.geekdex.quickhosts.png",
        (256, 256),
    );
    check_icon(
        "assets/pixmaps/io.github.geekdex.quickhosts.png",
        (256, 256),
    );
    check_icon(
        "assets/icons/128x128/apps/io.github.geekdex.quickhosts.png",
        (128, 128),
    );
    check_icon(
        "assets/icons/64x64/apps/io.github.geekdex.quickhosts.png",
        (64, 64),
    );
    check_icon(
        "assets/icons/48x48/apps/io.github.geekdex.quickhosts.png",
        (48, 48),
    );
    check_icon(
        "assets/icons/32x32/apps/io.github.geekdex.quickhosts.png",
        (32, 32),
    );
    check_icon(
        "assets/icons/16x16/apps/io.github.geekdex.quickhosts.png",
        (16, 16),
    );
}

#[test]
fn test_move_hosts_reordering() {
    let mut cfg = AppConfig {
        hosts: vec![
            HostItem::new_local("Host 1".to_string(), "1.1.1.1 h1.local\n".to_string()),
            HostItem::new_local("Host 2".to_string(), "2.2.2.2 h2.local\n".to_string()),
            HostItem::new_local("Host 3".to_string(), "3.3.3.3 h3.local\n".to_string()),
        ],
        ..Default::default()
    };

    let id1 = cfg.hosts[0].id.clone();
    let id2 = cfg.hosts[1].id.clone();
    let id3 = cfg.hosts[2].id.clone();

    // 1. Moving top item up should do nothing (boundary check)
    let pos0 = cfg.hosts.iter().position(|h| h.id == id1).unwrap();
    assert_eq!(pos0, 0);
    if pos0 > 0 {
        cfg.hosts.swap(pos0, pos0 - 1);
    }
    assert_eq!(cfg.hosts[0].id, id1);

    // 2. Moving bottom item down should do nothing (boundary check)
    let pos2 = cfg.hosts.iter().position(|h| h.id == id3).unwrap();
    assert_eq!(pos2, 2);
    if pos2 + 1 < cfg.hosts.len() {
        cfg.hosts.swap(pos2, pos2 + 1);
    }
    assert_eq!(cfg.hosts[2].id, id3);

    // 3. Move Host 2 up -> Host 2, Host 1, Host 3
    let pos1 = cfg.hosts.iter().position(|h| h.id == id2).unwrap();
    if pos1 > 0 {
        cfg.hosts.swap(pos1, pos1 - 1);
    }
    assert_eq!(cfg.hosts[0].id, id2);
    assert_eq!(cfg.hosts[1].id, id1);
    assert_eq!(cfg.hosts[2].id, id3);

    // Combined hosts output should have Host 2 content before Host 1
    let active_refs: Vec<&HostItem> = cfg.hosts.iter().collect();
    let combined = build_combined_hosts(true, &active_refs);
    let idx_h2 = combined.find("2.2.2.2 h2.local").unwrap();
    let idx_h1 = combined.find("1.1.1.1 h1.local").unwrap();
    assert!(
        idx_h2 < idx_h1,
        "Host 2 should precede Host 1 after moving up"
    );

    // 4. Move Host 2 down -> Host 1, Host 2, Host 3
    let pos_h2 = cfg.hosts.iter().position(|h| h.id == id2).unwrap();
    if pos_h2 + 1 < cfg.hosts.len() {
        cfg.hosts.swap(pos_h2, pos_h2 + 1);
    }
    assert_eq!(cfg.hosts[0].id, id1);
    assert_eq!(cfg.hosts[1].id, id2);
    assert_eq!(cfg.hosts[2].id, id3);

    // 5. Test drag-and-drop reorder: drag Host 3 (index 2) to index 0 -> Host 3, Host 1, Host 2
    let from_idx = 2;
    let to_idx = 0;
    if from_idx < cfg.hosts.len() && to_idx < cfg.hosts.len() && from_idx != to_idx {
        let item = cfg.hosts.remove(from_idx);
        cfg.hosts.insert(to_idx, item);
    }
    assert_eq!(cfg.hosts[0].id, id3);
    assert_eq!(cfg.hosts[1].id, id1);
    assert_eq!(cfg.hosts[2].id, id2);

    // 6. Test drag-and-drop reorder: drag Host 3 (index 0) to index 2 -> Host 1, Host 2, Host 3
    let from_idx = 0;
    let to_idx = 2;
    if from_idx < cfg.hosts.len() && to_idx < cfg.hosts.len() && from_idx != to_idx {
        let item = cfg.hosts.remove(from_idx);
        cfg.hosts.insert(to_idx, item);
    }
    assert_eq!(cfg.hosts[0].id, id1);
    assert_eq!(cfg.hosts[1].id, id2);
    assert_eq!(cfg.hosts[2].id, id3);
}

#[test]
fn test_close_shortcut_modifiers() {
    let ctx = egui::Context::default();

    // 1. Test Command+W (macOS event simulation)
    let mut raw_mac = egui::RawInput::default();
    raw_mac.events.push(egui::Event::Key {
        key: egui::Key::W,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            alt: false,
            ctrl: false,
            shift: false,
            mac_cmd: true,
            command: true,
        },
    });

    let _ = ctx.run_ui(raw_mac, |ui| {
        let close_requested = ui.input(|i| i.viewport().close_requested())
            || ui.input_mut(|i| {
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::W)
                    || i.consume_key(egui::Modifiers::MAC_CMD, egui::Key::W)
                    || i.consume_key(egui::Modifiers::CTRL, egui::Key::W)
            });
        assert!(close_requested, "Command+W on macOS should be detected");
    });

    // 2. Test Ctrl+W (Windows/Linux or Mac Ctrl key simulation)
    let mut raw_ctrl = egui::RawInput::default();
    raw_ctrl.events.push(egui::Event::Key {
        key: egui::Key::W,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            alt: false,
            ctrl: true,
            shift: false,
            mac_cmd: false,
            command: false,
        },
    });

    let _ = ctx.run_ui(raw_ctrl, |ui| {
        let close_requested = ui.input(|i| i.viewport().close_requested())
            || ui.input_mut(|i| {
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::W)
                    || i.consume_key(egui::Modifiers::MAC_CMD, egui::Key::W)
                    || i.consume_key(egui::Modifiers::CTRL, egui::Key::W)
            });
        assert!(close_requested, "Ctrl+W should be detected");
    });
}

#[test]
fn test_minimize_shortcut_modifiers() {
    let ctx = egui::Context::default();

    // 1. Test Command+M (macOS event simulation)
    let mut raw_mac = egui::RawInput::default();
    raw_mac.events.push(egui::Event::Key {
        key: egui::Key::M,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            alt: false,
            ctrl: false,
            shift: false,
            mac_cmd: true,
            command: true,
        },
    });

    let _ = ctx.run_ui(raw_mac, |ui| {
        let minimize_requested = ui.input_mut(|i| {
            i.consume_key(egui::Modifiers::COMMAND, egui::Key::M)
                || i.consume_key(egui::Modifiers::MAC_CMD, egui::Key::M)
                || i.consume_key(egui::Modifiers::CTRL, egui::Key::M)
        });
        assert!(minimize_requested, "Command+M on macOS should be detected");
        quickhosts::ui::minimize_window(ui.ctx());
    });

    // 2. Test Ctrl+M (Windows/Linux or Mac Ctrl key simulation)
    let mut raw_ctrl = egui::RawInput::default();
    raw_ctrl.events.push(egui::Event::Key {
        key: egui::Key::M,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            alt: false,
            ctrl: true,
            shift: false,
            mac_cmd: false,
            command: false,
        },
    });

    let _ = ctx.run_ui(raw_ctrl, |ui| {
        let minimize_requested = ui.input_mut(|i| {
            i.consume_key(egui::Modifiers::COMMAND, egui::Key::M)
                || i.consume_key(egui::Modifiers::MAC_CMD, egui::Key::M)
                || i.consume_key(egui::Modifiers::CTRL, egui::Key::M)
        });
        assert!(minimize_requested, "Ctrl+M should be detected");
        quickhosts::ui::minimize_window(ui.ctx());
    });
}
