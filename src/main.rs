#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod fetcher;
mod font_loader;
mod hosts_manager;
mod i18n;
mod model;
mod single_instance;
mod storage;
mod ui;

use eframe::NativeOptions;
use ui::QuickHostsApp;

fn load_app_icon() -> Option<egui::IconData> {
    let icon_bytes =
        include_bytes!("../assets/icons/256x256/apps/io.github.geekdex.quickhosts.png");
    if let Ok(img) = image::load_from_memory(icon_bytes) {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Some(egui::IconData {
            rgba: rgba.into_raw(),
            width,
            height,
        })
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
fn ensure_linux_desktop_integration() {
    let Some(data_dir) = dirs::data_dir() else {
        return;
    };
    let app_dir = data_dir.join("applications");
    let desktop_file = app_dir.join("io.github.geekdex.quickhosts.desktop");
    let icon_256_path =
        data_dir.join("icons/hicolor/256x256/apps/io.github.geekdex.quickhosts.png");

    let exe_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "quickhosts".to_string());

    let desktop_content = format!(
        "[Desktop Entry]\n\
         Name=QuickHosts\n\
         Comment=A fast, lightweight Hosts switcher and manager\n\
         Exec=\"{}\" %u\n\
         Icon=io.github.geekdex.quickhosts\n\
         Terminal=false\n\
         Type=Application\n\
         Categories=Network;Development;Utility;\n\
         StartupWMClass=io.github.geekdex.quickhosts\n\
         Keywords=hosts;dns;switch;network;\n",
        exe_path
    );

    // If desktop file and icons already match, avoid redundant I/O and process spawns
    if desktop_file.exists() && icon_256_path.exists() {
        if let Ok(existing) = std::fs::read_to_string(&desktop_file) {
            if existing == desktop_content {
                return;
            }
        }
    }

    let icons_256 = data_dir.join("icons/hicolor/256x256/apps");
    let icons_64 = data_dir.join("icons/hicolor/64x64/apps");
    let icons_32 = data_dir.join("icons/hicolor/32x32/apps");
    let pixmaps = data_dir.join("pixmaps");

    let _ = std::fs::create_dir_all(&app_dir);
    let _ = std::fs::create_dir_all(&icons_256);
    let _ = std::fs::create_dir_all(&icons_64);
    let _ = std::fs::create_dir_all(&icons_32);
    let _ = std::fs::create_dir_all(&pixmaps);

    let icon_256_bytes =
        include_bytes!("../assets/icons/256x256/apps/io.github.geekdex.quickhosts.png");
    let icon_64_bytes =
        include_bytes!("../assets/icons/64x64/apps/io.github.geekdex.quickhosts.png");
    let icon_32_bytes =
        include_bytes!("../assets/icons/32x32/apps/io.github.geekdex.quickhosts.png");

    let _ = std::fs::write(&icon_256_path, icon_256_bytes);
    let _ = std::fs::write(
        icons_64.join("io.github.geekdex.quickhosts.png"),
        icon_64_bytes,
    );
    let _ = std::fs::write(
        icons_32.join("io.github.geekdex.quickhosts.png"),
        icon_32_bytes,
    );
    let _ = std::fs::write(
        pixmaps.join("io.github.geekdex.quickhosts.png"),
        icon_256_bytes,
    );

    let _ = std::fs::write(&desktop_file, &desktop_content);

    // Silently refresh application database in background without polluting terminal output
    std::thread::spawn(move || {
        let _ = std::process::Command::new("update-desktop-database")
            .arg(&app_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    });
}

fn main() -> eframe::Result<()> {
    #[cfg(target_os = "linux")]
    ensure_linux_desktop_integration();

    let (instance_guard, focus_rx) = match single_instance::acquire() {
        Ok(res) => (Some(res.0), Some(res.1)),
        Err(single_instance::SingleInstanceError::AlreadyRunning) => {
            let lang = i18n::detect_system_language();
            let title = i18n::tr(&lang, "already_running_title");
            let desc = i18n::tr(&lang, "already_running_desc");

            println!("{title}: {desc}");

            // On Linux desktop, trigger native desktop notification
            #[cfg(target_os = "linux")]
            {
                let _ = std::process::Command::new("notify-send")
                    .arg("-a")
                    .arg("QuickHosts")
                    .arg("-i")
                    .arg("io.github.geekdex.quickhosts")
                    .arg(title)
                    .arg(desc)
                    .spawn();
            }

            // Show native message dialog to gracefully inform user
            rfd::MessageDialog::new()
                .set_title(title)
                .set_description(desc)
                .set_buttons(rfd::MessageButtons::Ok)
                .set_level(rfd::MessageLevel::Info)
                .show();

            return Ok(());
        }
        Err(err) => {
            eprintln!("Warning: Failed to acquire single-instance lock: {err}");
            (None, None)
        }
    };

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("QuickHosts - Ultra-fast Hosts Switcher")
        .with_app_id("io.github.geekdex.quickhosts")
        .with_inner_size([980.0, 680.0])
        .with_min_inner_size([720.0, 500.0]);

    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "QuickHosts",
        options,
        Box::new(move |cc| Ok(Box::new(QuickHostsApp::new(cc, instance_guard, focus_rx)))),
    )
}
