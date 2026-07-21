//! 设置模型 —— 通用底层设置
//!
//! 这里只放全局通用的底层设置。
//! 各业务模块的设置项由各模块自行管理，
//! 或通过扩展机制注册到全局设置中。

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
    /// 是否启用 USN 文件搜索
    pub file_search_enabled: bool,
    /// 文件搜索收录目录
    pub file_search_roots: Vec<PathBuf>,
    /// 文件搜索的盘符列表（如 ['C', 'D', 'E']），为空则搜索所有盘符
    pub file_search_drives: Vec<char>,
    /// 同时搜索的类别
    pub enabled_categories: Vec<String>,
    /// 窗口是否始终置顶
    #[serde(default = "default_pin_to_top")]
    pub pin_to_top: bool,
    /// 是否跟随系统主题（light/dark）
    #[serde(default)]
    pub follow_system_theme: bool,
}

fn default_pin_to_top() -> bool {
    false
}

fn default_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(home) = std::env::var("USERPROFILE") {
        let home_path = PathBuf::from(home);
        let common_dirs = ["Desktop", "Documents", "Downloads", "Music", "Pictures", "Videos"];
        for dir in common_dirs {
            let path = home_path.join(dir);
            if path.exists() {
                roots.push(path);
            }
        }
    }

    if let Ok(app_data) = std::env::var("APPDATA") {
        let path = PathBuf::from(app_data);
        if path.exists() {
            roots.push(path);
        }
    }

    roots
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Alt+Space".into(),
            theme: ThemeMode::Dark,
            accent_color: "#ffffff".into(),
            file_search_enabled: true,
            file_search_roots: default_search_roots(),
            file_search_drives: Vec::new(),
            enabled_categories: vec![
                "apps".into(),
                "files".into(),
                "commands".into(),
            ],
            pin_to_top: true,
            follow_system_theme: false,
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
            "pinToTop" | "pin_to_top" => {
                if let Some(b) = value.as_bool() {
                    self.pin_to_top = b;
                }
            }
            "followSystemTheme" | "follow_system_theme" => {
                if let Some(b) = value.as_bool() {
                    self.follow_system_theme = b;
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
