use crate::model::AppConfig;
use std::fs;
use std::path::PathBuf;

pub fn get_app_dir() -> PathBuf {
    if let Some(config_dir) = dirs::config_dir() {
        let app_dir = config_dir.join("quickhosts");
        if !app_dir.exists() {
            let _ = fs::create_dir_all(&app_dir);
        }
        app_dir
    } else {
        PathBuf::from("./.quickhosts")
    }
}

pub fn get_config_path() -> PathBuf {
    get_app_dir().join("config.json")
}

pub fn get_backup_hosts_path() -> PathBuf {
    get_app_dir().join("hosts.backup")
}

pub fn load_config() -> AppConfig {
    let path = get_config_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                return config;
            }
        }
    }
    let default_cfg = AppConfig::default();
    let _ = save_config(&default_cfg);
    default_cfg
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = get_config_path();
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    let temp_path = path.with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));
    fs::write(&temp_path, &content)
        .map_err(|e| format!("Failed to write temporary config: {}", e))?;

    #[cfg(windows)]
    if path.exists() {
        let _ = fs::remove_file(&path);
    }

    if let Err(rename_err) = fs::rename(&temp_path, &path) {
        let _ = fs::remove_file(&temp_path);
        return fs::write(path, content).map_err(|e| {
            format!(
                "Failed to save config file (rename error: {}): {}",
                rename_err, e
            )
        });
    }
    Ok(())
}

pub fn get_system_hosts_path() -> &'static str {
    static PATH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    PATH.get_or_init(|| {
        #[cfg(target_os = "windows")]
        {
            if let Ok(sys_root) = std::env::var("SystemRoot") {
                return format!(r"{}\System32\drivers\etc\hosts", sys_root);
            }
            if let Ok(windir) = std::env::var("windir") {
                return format!(r"{}\System32\drivers\etc\hosts", windir);
            }
            r"C:\Windows\System32\drivers\etc\hosts".to_string()
        }
        #[cfg(not(target_os = "windows"))]
        {
            "/etc/hosts".to_string()
        }
    })
}

pub fn read_system_hosts() -> Result<String, String> {
    let path = get_system_hosts_path();
    fs::read_to_string(path).map_err(|e| format!("Could not read system hosts ({}): {}", path, e))
}

pub fn backup_system_hosts_if_needed() -> Result<(), String> {
    let backup_path = get_backup_hosts_path();
    if !backup_path.exists() {
        let sys_content = read_system_hosts()?;
        fs::write(&backup_path, sys_content)
            .map_err(|e| format!("Failed to create backup hosts file: {}", e))?;
    }
    Ok(())
}
