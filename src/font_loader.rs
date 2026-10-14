use std::path::Path;

pub fn load_cjk_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let candidate_paths = [
        // Linux candidates (prioritize lightweight fonts to reduce memory footprint)
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/NotoSansCJK-Regular.ttc",
        // Windows candidates
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\msyh.ttf",
        r"C:\Windows\Fonts\msyhl.ttc",
        r"C:\Windows\Fonts\simsun.ttc",
        r"C:\Windows\Fonts\simhei.ttf",
        // macOS candidates
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/System/Library/Fonts/STHeiti Medium.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/System/Library/Fonts/Supplemental/Songti.ttc",
        "/System/Library/Fonts/Supplemental/PingFang.ttc",
        "/Library/Fonts/Arial Unicode.ttf",
    ];

    let mut loaded_data = None;
    for path_str in candidate_paths {
        let p = Path::new(path_str);
        if p.exists() {
            if let Ok(data) = std::fs::read(p) {
                loaded_data = Some(data);
                break;
            }
        }
    }

    #[cfg(target_os = "windows")]
    if loaded_data.is_none() {
        let win_dir = std::env::var("SystemRoot")
            .or_else(|_| std::env::var("windir"))
            .unwrap_or_else(|_| r"C:\Windows".to_string());
        let font_dir = Path::new(&win_dir).join("Fonts");
        for font_name in &[
            "msyh.ttc",
            "msyh.ttf",
            "msyhl.ttc",
            "simsun.ttc",
            "simhei.ttf",
        ] {
            let p = font_dir.join(font_name);
            if p.exists() {
                if let Ok(data) = std::fs::read(&p) {
                    loaded_data = Some(data);
                    break;
                }
            }
        }
    }

    if loaded_data.is_none() && cfg!(target_os = "linux") {
        if let Ok(output) = std::process::Command::new("fc-match")
            .args(["-f", "%{file}", ":lang=zh"])
            .output()
        {
            if output.status.success() {
                let font_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if let Ok(data) = std::fs::read(&font_path) {
                    loaded_data = Some(data);
                }
            }
        }
    }

    if let Some(font_bytes) = loaded_data {
        fonts.font_data.insert(
            "cjk_fallback".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(font_bytes)),
        );

        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "cjk_fallback".to_owned());

        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("cjk_fallback".to_owned());

        ctx.set_fonts(fonts);
    }
}
