use crate::i18n::{tr, Language};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UpdateInterval {
    Never,
    Minutes15,
    Minutes30,
    Hour1,
    Hours6,
    Hours12,
    Day1,
}

impl UpdateInterval {
    pub fn to_minutes(&self) -> u32 {
        match self {
            UpdateInterval::Never => 0,
            UpdateInterval::Minutes15 => 15,
            UpdateInterval::Minutes30 => 30,
            UpdateInterval::Hour1 => 60,
            UpdateInterval::Hours6 => 360,
            UpdateInterval::Hours12 => 720,
            UpdateInterval::Day1 => 1440,
        }
    }

    pub fn display_name(&self, lang: &Language) -> &'static str {
        match self {
            UpdateInterval::Never => tr(lang, "interval_never"),
            UpdateInterval::Minutes15 => tr(lang, "interval_15m"),
            UpdateInterval::Minutes30 => tr(lang, "interval_30m"),
            UpdateInterval::Hour1 => tr(lang, "interval_1h"),
            UpdateInterval::Hours6 => tr(lang, "interval_6h"),
            UpdateInterval::Hours12 => tr(lang, "interval_12h"),
            UpdateInterval::Day1 => tr(lang, "interval_24h"),
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            UpdateInterval::Never,
            UpdateInterval::Minutes15,
            UpdateInterval::Minutes30,
            UpdateInterval::Hour1,
            UpdateInterval::Hours6,
            UpdateInterval::Hours12,
            UpdateInterval::Day1,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HostType {
    Local,
    Remote {
        url: String,
        interval: UpdateInterval,
        last_updated: Option<DateTime<Utc>>,
        last_error: Option<String>,
        is_fetching: bool,
    },
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostItem {
    pub id: String,
    pub title: String,
    pub host_type: HostType,
    pub enabled: bool,
    pub content: String,
    pub read_only: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl HostItem {
    pub fn new_local(title: String, content: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            host_type: HostType::Local,
            enabled: true,
            content,
            read_only: false,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_remote(title: String, url: String, interval: UpdateInterval) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            host_type: HostType::Remote {
                url,
                interval,
                last_updated: None,
                last_error: None,
                is_fetching: false,
            },
            enabled: true,
            content: "# Remote host content will be fetched shortly...\n".to_string(),
            read_only: false,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn is_system(&self) -> bool {
        matches!(self.host_type, HostType::System)
    }

    #[allow(dead_code)]
    pub fn is_remote(&self) -> bool {
        matches!(self.host_type, HostType::Remote { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum RailTab {
    #[default]
    Hosts,
    Trash,
    History,
    Settings,
}

fn default_sidebar_width() -> f32 {
    230.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub hosts: Vec<HostItem>,
    #[serde(default)]
    pub trash: Vec<HostItem>,
    pub master_enabled: bool,
    pub auto_apply_on_change: bool,
    pub selected_host_id: Option<String>,
    pub dark_mode: bool,
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub active_rail_tab: RailTab,
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        let sample_local = HostItem::new_local(
            "本地开发环境 (Dev)".to_string(),
            "# 本地开发 Hosts 映射\n127.0.0.1 dev.local\n::1 dev.local\n".to_string(),
        );

        let selected_host_id = Some(sample_local.id.clone());

        Self {
            hosts: vec![sample_local],
            trash: vec![],
            master_enabled: true,
            auto_apply_on_change: true,
            selected_host_id,
            dark_mode: false, // Default to LIGHT mode matching SwitchHosts light screenshot
            language: Language::System,
            active_rail_tab: RailTab::Hosts,
            sidebar_width: 230.0,
        }
    }
}
