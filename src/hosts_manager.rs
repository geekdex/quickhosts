use crate::model::HostItem;
use crate::storage::{backup_system_hosts_if_needed, get_system_hosts_path, read_system_hosts};
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

pub const START_MARKER: &str = "# --- QuickHosts START (Auto-Generated) ---";
pub const END_MARKER: &str = "# --- QuickHosts END ---";

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HostRule {
    pub ip: String,
    pub domain: String,
    pub comment: Option<String>,
    pub raw_line: String,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Default)]
pub struct HostsStats {
    pub total_lines: usize,
    pub active_rules: usize,
    pub disabled_rules: usize,
    pub comment_lines: usize,
}

pub fn parse_hosts_stats(content: &str) -> HostsStats {
    let mut stats = HostsStats::default();
    for line in content.lines() {
        let trimmed = line.trim();
        stats.total_lines += 1;
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('#') {
            stats.comment_lines += 1;
            if trimmed.starts_with("# #")
                || trimmed.starts_with("#127")
                || trimmed.starts_with("# 127")
                || trimmed.starts_with("#10.")
                || trimmed.starts_with("# 10.")
            {
                stats.disabled_rules += 1;
            }
        } else {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                stats.active_rules += 1;
            }
        }
    }
    stats
}

pub fn strip_quickhosts_block(sys_content: &str) -> String {
    if let Some(start_idx) = sys_content.find(START_MARKER) {
        if let Some(end_idx) = sys_content.find(END_MARKER) {
            let after_end = end_idx + END_MARKER.len();
            let mut base = String::new();
            base.push_str(&sys_content[..start_idx]);
            if after_end < sys_content.len() {
                base.push_str(sys_content[after_end..].trim_start_matches('\n'));
            }
            return base.trim_end().to_string() + "\n";
        }
    }
    sys_content.to_string()
}

pub fn build_combined_hosts(master_enabled: bool, active_hosts: &[&HostItem]) -> String {
    let base_system = match read_system_hosts() {
        Ok(content) => strip_quickhosts_block(&content),
        Err(_) => {
            "# Original system hosts unavailable\n127.0.0.1 localhost\n::1 localhost\n".to_string()
        }
    };

    if !master_enabled || active_hosts.is_empty() {
        return base_system;
    }

    let mut result = base_system;
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result.push('\n');
    result.push_str(START_MARKER);
    result.push('\n');
    result.push_str("# Managed by QuickHosts. Toggle rules in the app anytime.\n");

    for host in active_hosts {
        result.push_str(&format!("\n# === [{}] ===\n", host.title));
        for line in host.content.lines() {
            result.push_str(line);
            result.push('\n');
        }
    }

    result.push('\n');
    result.push_str(END_MARKER);
    result.push('\n');

    result
}

pub fn is_system_hosts_writable() -> bool {
    let path = get_system_hosts_path();
    fs::OpenOptions::new().write(true).open(path).is_ok()
}

pub fn flush_dns_cache() {
    std::thread::spawn(|| {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            let _ = Command::new("ipconfig")
                .arg("/flushdns")
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("dscacheutil").arg("-flushcache").output();
            let _ = Command::new("killall")
                .args(["-HUP", "mDNSResponder"])
                .output();
        }
        #[cfg(target_os = "linux")]
        {
            let res = Command::new("resolvectl").arg("flush-caches").output();
            if res.is_err() || !res.as_ref().unwrap().status.success() {
                let _ = Command::new("systemd-resolve")
                    .arg("--flush-caches")
                    .output();
            }
        }
    });
}

fn atomic_direct_write(path: &str, content: &str) -> Result<(), std::io::Error> {
    let target = std::path::Path::new(path);
    let parent = target.parent().unwrap_or_else(|| std::path::Path::new("."));
    let temp_file = parent.join(format!(".quickhosts_tmp_{}", uuid::Uuid::new_v4()));

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_file)?;

    file.write_all(content.as_bytes())?;
    file.sync_all()?;
    drop(file);

    #[cfg(target_os = "windows")]
    {
        // On Windows, fs::rename fails if target already exists.
        // If target exists, avoid rename error and fallback to direct write.
        if target.exists() {
            let _ = fs::remove_file(&temp_file);
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "Target file exists; falling back to direct write on Windows",
            ));
        }
    }

    if let Err(err) = fs::rename(&temp_file, target) {
        let _ = fs::remove_file(&temp_file);
        return Err(err);
    }
    Ok(())
}

pub fn write_system_hosts(combined_content: &str) -> Result<(), String> {
    let _ = backup_system_hosts_if_needed();
    let path = get_system_hosts_path();

    // Try direct atomic write first
    match atomic_direct_write(path, combined_content).or_else(|_| fs::write(path, combined_content))
    {
        Ok(_) => {
            flush_dns_cache();
            Ok(())
        }
        Err(err) => {
            // Attempt elevation write
            let res = elevated_write_system_hosts(combined_content, path);
            if res.is_ok() {
                flush_dns_cache();
            }
            res.map_err(|elev_err| {
                format!(
                    "Direct write failed ({}), Elevated write also failed: {}",
                    err, elev_err
                )
            })
        }
    }
}

fn elevated_write_system_hosts(content: &str, target_path: &str) -> Result<(), String> {
    if cfg!(target_os = "windows") {
        // On Windows, use PowerShell Start-Process elevated to write file
        let temp_file_name = format!("quickhosts_temp_{}.txt", uuid::Uuid::new_v4());
        let temp_path = std::env::temp_dir().join(&temp_file_name);
        fs::write(&temp_path, content).map_err(|e| format!("Failed to write temp file: {}", e))?;

        let escaped_temp = temp_path.to_string_lossy().replace('\'', "''");
        let escaped_target = target_path.replace('\'', "''");
        let arg_list = format!(
            "-NoProfile -NonInteractive -WindowStyle Hidden -Command Copy-Item -LiteralPath '{}' -Destination '{}' -Force",
            escaped_temp, escaped_target
        );
        let ps_script = format!(
            "Start-Process powershell -Verb RunAs -Wait -WindowStyle Hidden -ArgumentList \"{}\"",
            arg_list.replace('"', "\\\"")
        );

        let mut cmd = Command::new("powershell");
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        let output = cmd
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(&ps_script)
            .output();

        let _ = fs::remove_file(&temp_path);

        let output = output.map_err(|e| format!("Powershell elevation command failed: {}", e))?;

        if output.status.success() {
            if let Ok(written) = fs::read_to_string(target_path) {
                if written == content {
                    return Ok(());
                }
            }
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    } else if cfg!(target_os = "macos") {
        // On macOS, use osascript for admin dialog
        let temp_file_name = format!("quickhosts_temp_{}.txt", uuid::Uuid::new_v4());
        let temp_path = std::env::temp_dir().join(&temp_file_name);
        fs::write(&temp_path, content).map_err(|e| format!("Failed to write temp file: {}", e))?;

        let escaped_temp = temp_path.to_string_lossy().replace('\'', "'\\''");
        let escaped_target = target_path.replace('\'', "'\\''");
        let osa_cmd = format!(
            "do shell script \"cp '{}' '{}' && chmod 666 '{}'\" with administrator privileges",
            escaped_temp, escaped_target, escaped_target
        );

        let output = Command::new("osascript").arg("-e").arg(&osa_cmd).output();

        let _ = fs::remove_file(&temp_path);

        let output = output.map_err(|e| format!("osascript elevation failed: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    } else {
        // Linux: pkexec sh -c "tee /etc/hosts > /dev/null && sync"
        let cmd_str = format!("tee '{}' > /dev/null && sync", target_path);

        let mut child = Command::new("pkexec")
            .arg("sh")
            .arg("-c")
            .arg(&cmd_str)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .or_else(|_| {
                Command::new("sudo")
                    .arg("sh")
                    .arg("-c")
                    .arg(&cmd_str)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::null())
                    .stderr(Stdio::piped())
                    .spawn()
            })
            .map_err(|e| format!("Failed to launch pkexec/sudo command: {}", e))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(content.as_bytes())
                .map_err(|e| format!("Failed to write to stdin: {}", e))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait for pkexec: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }
}
