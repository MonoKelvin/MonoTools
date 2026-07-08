use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Light,
    Dark,
    Auto,
}

impl Default for ThemeMode {
    fn default() -> Self {
        ThemeMode::Dark
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// 全局快捷键字符串（如 "Alt+Space"）
    pub hotkey: String,
    /// 主题
    pub theme: ThemeMode,
    /// 强调色（hex）
    pub accent_color: String,
    /// 用户自定义启动应用路径列表
    pub custom_app_paths: Vec<PathBuf>,
    /// 是否启用 USN 文件搜索
    pub file_search_enabled: bool,
    /// 文件搜索收录目录
    pub file_search_roots: Vec<PathBuf>,
    /// 启动延迟基础值
    pub default_delay_seconds: u32,
    /// 启动时是否自动索引
    pub auto_index_on_startup: bool,
    /// 同时搜索的类别
    pub enabled_categories: Vec<String>,
    /// 窗口是否始终置顶
    #[serde(default = "default_pin_to_top")]
    pub pin_to_top: bool,
}

fn default_pin_to_top() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Alt+Space".into(),
            theme: ThemeMode::Dark,
            accent_color: "#ffffff".into(),
            custom_app_paths: vec![],
            file_search_enabled: true,
            file_search_roots: vec![],
            default_delay_seconds: 0,
            auto_index_on_startup: true,
            enabled_categories: vec![
                "apps".into(),
                "files".into(),
                "commands".into(),
                "startup".into(),
            ],
            pin_to_top: true,
        }
    }
}

impl Settings {
    /// 将 JSON 值写入字段
    pub fn set_field(&mut self, key: &str, value: serde_json::Value) {
        self.apply_field(key, &value);
    }

    /// 应用单个字段
    pub fn apply_field(&mut self, key: &str, value: &serde_json::Value) {
        match key {
            "hotkey" => {
                if let Some(s) = value.as_str() {
                    self.hotkey = s.into();
                }
            }
            "theme" => {
                if let Some(s) = value.as_str() {
                    self.theme = match s {
                        "light" => ThemeMode::Light,
                        "auto" => ThemeMode::Auto,
                        _ => ThemeMode::Dark,
                    };
                }
            }
            "accentColor" | "accent_color" => {
                if let Some(s) = value.as_str() {
                    self.accent_color = s.into();
                }
            }
            "fileSearchEnabled" | "file_search_enabled" => {
                if let Some(b) = value.as_bool() {
                    self.file_search_enabled = b;
                }
            }
            "defaultDelaySeconds" | "default_delay_seconds" => {
                if let Some(n) = value.as_u64() {
                    self.default_delay_seconds = n as u32;
                }
            }
            "autoIndexOnStartup" | "auto_index_on_startup" => {
                if let Some(b) = value.as_bool() {
                    self.auto_index_on_startup = b;
                }
            }
            "pinToTop" | "pin_to_top" => {
                if let Some(b) = value.as_bool() {
                    self.pin_to_top = b;
                }
            }
            _ => {
                // 忽略未知字段
            }
        }
    }

    /// 兼容 CLI 中将 "string value" 直接写为字符串
    pub fn set_field_raw(&mut self, key: &str, raw: &str) {
        self.set_field(
            key,
            serde_json::Value::String(raw.to_string()),
        );
    }
}
