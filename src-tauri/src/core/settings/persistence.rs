//! 设置持久化 — SQLite 读写
//!
//! 在应用启动时从 SQLite 加载设置合并到内存中,
//! 每次 save 时全量写入 SQLite。

use crate::core::settings::Settings;
use crate::services::StorageService;
use serde_json;

/// 从 StorageService 加载所有设置, 合并到默认值之上
pub fn load_settings(store: &StorageService) -> Settings {
    let mut settings = Settings::default();

    let raw: Option<String> = match store.get_setting::<String>("__settings_full__", String::new()) {
        Ok(v) if !v.is_empty() => Some(v),
        _ => None,
    };

    if let Some(json) = raw {
        if let Ok(loaded) = serde_json::from_str::<Settings>(&json) {
            settings = loaded;
        }
    }

    settings
}

/// 全量保存设置到 StorageService
pub fn save_settings(store: &StorageService, settings: &Settings) {
    if let Ok(json) = serde_json::to_string(settings) {
        let _ = store.set_setting("__settings_full__", &json);
    }
}
