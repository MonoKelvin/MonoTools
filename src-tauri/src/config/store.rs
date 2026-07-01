use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigData {
    pub general: GeneralConfig,
    pub search: SearchConfig,
    pub hotkeys: HashMap<String, HotkeyConfig>,
    pub search_providers: ProviderConfig,
    pub file_search: FileSearchConfig,
    pub workspace: WorkspaceConfig,
    pub theme: ThemeConfig,
    pub plugin: PluginConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub language: String,
    pub startup: bool,
    pub silent_start: bool,
    pub check_updates: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub window_width: u32,
    pub window_height: u32,
    pub position: String,
    pub blur_on_lost_focus: bool,
    pub debounce_ms: u32,
    pub max_results: usize,
    pub show_recent_on_empty: bool,
    pub remember_last_query: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub key: String,
    pub modifiers: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub order: Vec<String>,
    pub disabled: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSearchConfig {
    pub indexed_volumes: Vec<String>,
    pub exclude_paths: Vec<String>,
    pub max_index_size_mb: usize,
    pub index_on_startup: bool,
    pub real_time_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub auto_save_interval: u32,
    pub confirm_before_restore: bool,
    pub restore_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub active: String,
    pub mode: String,
    pub accent_color: String,
    pub font_scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub auto_update: bool,
    pub developer_mode: bool,
    pub trusted_publishers: Vec<String>,
}

impl Default for ConfigData {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                language: "zh-CN".to_string(),
                startup: true,
                silent_start: true,
                check_updates: true,
            },
            search: SearchConfig {
                window_width: 800,
                window_height: 520,
                position: "center".to_string(),
                blur_on_lost_focus: true,
                debounce_ms: 80,
                max_results: 50,
                show_recent_on_empty: true,
                remember_last_query: false,
            },
            hotkeys: HashMap::new(),
            search_providers: ProviderConfig {
                order: vec![
                    "builtin:app-launcher".to_string(),
                    "builtin:file-search".to_string(),
                    "builtin:workspace-manager".to_string(),
                    "builtin:command-palette".to_string(),
                ],
                disabled: vec![],
            },
            file_search: FileSearchConfig {
                indexed_volumes: vec!["C:".to_string()],
                exclude_paths: vec![
                    "C:/Windows".to_string(),
                    "C:/Program Files/WindowsApps".to_string(),
                    "*/node_modules".to_string(),
                    "*/.git".to_string(),
                ],
                max_index_size_mb: 1024,
                index_on_startup: true,
                real_time_update: true,
            },
            workspace: WorkspaceConfig {
                auto_save_interval: 0,
                confirm_before_restore: true,
                restore_delay_ms: 500,
            },
            theme: ThemeConfig {
                active: "builtin:default-theme".to_string(),
                mode: "dark".to_string(),
                accent_color: "#5e6ad2".to_string(),
                font_scale: 1.0,
            },
            plugin: PluginConfig {
                auto_update: false,
                developer_mode: false,
                trusted_publishers: vec![],
            },
        }
    }
}

/// 配置存储
pub struct ConfigStore {
    data: ConfigData,
    config_path: PathBuf,
}

impl ConfigStore {
    pub fn new() -> Result<Self> {
        let config_dir = Self::get_config_dir()?;
        std::fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("settings.json");

        let data = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            let default = ConfigData::default();
            // 保存默认配置
            Self { data: default.clone(), config_path: config_path.clone() }.save()?;
            default
        };

        Ok(Self { data, config_path })
    }

    /// 获取配置目录
    fn get_config_dir() -> Result<PathBuf> {
        // 优先使用环境变量 MONOTOOLS_DATA_DIR
        if let Ok(dir) = std::env::var("MONOTOOLS_DATA_DIR") {
            return Ok(PathBuf::from(dir).join("config"));
        }

        // 使用 %LOCALAPPDATA%/MonoTools
        let appdata = dirs::next_local_data_dir()
            .context("Failed to get local app data directory")?
            .ok_or_else(|| anyhow::anyhow!("Could not determine local app data directory"))?;

        Ok(appdata.join("MonoTools").join("config"))
    }

    /// 保存配置到文件
    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.data)?;
        std::fs::write(&self.config_path, json)?;
        Ok(())
    }

    /// 获取配置项
    pub fn get<T: serde::de::DeserializeOwn>(&self, key: &str) -> Result<Option<T>> {
        let value = self.get_raw(key)?;
        match value {
            Some(v) => Ok(Some(serde_json::from_value(v)?)),
            None => Ok(None),
        }
    }

    /// 获取原始配置值
    pub fn get_raw(&self, key: &str) -> Result<Option<Value>> {
        let keys: Vec<&str> = key.split('.').collect();
        let mut current = serde_json::to_value(&self.data)?;

        for k in keys {
            match current {
                Value::Object(ref mut map) => {
                    current = map.remove(k).unwrap_or(Value::Null);
                }
                _ => return Ok(None),
            }
        }

        Ok(Some(current))
    }

    /// 设置配置项
    pub fn set<T: serde::Serialize>(&mut self, key: &str, value: T) -> Result<()> {
        let keys: Vec<&str> = key.split('.').collect();
        let mut current = serde_json::to_value(&self.data)?;

        // 遍历到倒数第二层
        let mut map = current.as_object_mut().unwrap();
        for (i, k) in keys.iter().enumerate() {
            if i == keys.len() - 1 {
                // 最后一层，设置值
                map.insert(k.to_string(), serde_json::to_value(value)?);
            } else {
                // 进入下一层
                if !map.contains_key(*k) {
                    map.insert(k.to_string(), serde_json::json!({}));
                }
                map = map.get_mut(*k).unwrap().as_object_mut().unwrap();
            }
        }

        // 反序列化回 ConfigData
        self.data = serde_json::from_value(current)?;
        self.save()?;
        Ok(())
    }

    /// 获取所有配置
    pub fn get_all(&self) -> &ConfigData {
        &self.data
    }

    /// 获取可变引用
    pub fn get_mut(&mut self) -> &mut ConfigData {
        &mut self.data
    }
}

impl Default for ConfigStore {
    fn default() -> Self {
        Self::new().expect("Failed to create config store")
    }
}
