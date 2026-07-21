//! 设置模块 IPC 命令
//!
//! 通用设置相关的 IPC 命令定义在此。
//! 注意：这些命令依赖 AppState（由上层组装），
//! 但逻辑上属于设置模块，因此放在这里统一维护。
//!
//! 各业务模块的设置 IPC 由各模块自行提供。

use crate::app::state::AppState;
use crate::core::settings::{Settings, ThemeMode};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

// ==================== 通用设置 IPC ====================

#[tauri::command]
pub async fn get_setting(
    state: State<'_, Arc<AppState>>,
    key: String,
) -> Result<Option<serde_json::Value>, String> {
    let s = state.settings_repo.get();
    let json = serde_json::to_value(&s).unwrap_or(serde_json::Value::Null);
    Ok(json.as_object().and_then(|o| o.get(&key).cloned()))
}

#[tauri::command]
pub async fn set_setting(
    state: State<'_, Arc<AppState>>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    state
        .settings_repo
        .update(Box::new(move |s| {
            s.set_field(&key, value);
        }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_all_settings(state: State<'_, Arc<AppState>>) -> Result<Settings, String> {
    Ok(state.settings_repo.get())
}

#[tauri::command]
pub async fn set_all_settings(
    state: State<'_, Arc<AppState>>,
    value: Settings,
) -> Result<(), String> {
    state.settings_repo.save(value).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_appearance(state: State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    let s = state.settings_repo.get();
    Ok(serde_json::json!({
        "mode": s.theme,
        "accent": s.accent_color,
    }))
}

#[tauri::command]
pub async fn set_appearance(
    state: State<'_, Arc<AppState>>,
    appearance: serde_json::Value,
) -> Result<(), String> {
    let mode = match appearance
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("dark")
    {
        "light" => ThemeMode::Light,
        "auto" => ThemeMode::Auto,
        _ => ThemeMode::Dark,
    };
    let accent = appearance
        .get("accent")
        .and_then(|v| v.as_str())
        .unwrap_or("#ffffff")
        .to_string();
    let res = state.settings_repo.update(Box::new(move |s| {
        s.theme = mode;
        s.accent_color = accent;
    }));
    res.map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_pin_top(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    Ok(state.settings_repo.get().pin_to_top)
}

#[tauri::command]
pub async fn set_pin_top(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    value: bool,
) -> Result<(), String> {
    state
        .settings_repo
        .update(Box::new(move |s| {
            s.pin_to_top = value;
        }))
        .map_err(|e| e.to_string())?;
    if let Some(w) = app.get_webview_window("search") {
        let _ = w.set_always_on_top(value);
        if value {
            let _ = w.show();
            let _ = w.set_focus();
        } else {
            let _ = w.hide();
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn set_follow_system_theme(
    state: State<'_, Arc<AppState>>,
    value: bool,
) -> Result<(), String> {
    state
        .settings_repo
        .update(Box::new(move |s| {
            s.follow_system_theme = value;
        }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 注册设置模块的 IPC 命令到 Tauri builder
pub fn register_ipc_commands(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder.invoke_handler(tauri::generate_handler![
        get_setting,
        set_setting,
        get_all_settings,
        set_all_settings,
        get_appearance,
        set_appearance,
        get_pin_top,
        set_pin_top,
        set_follow_system_theme,
    ])
}
