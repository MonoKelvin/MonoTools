//! 仓库层 IPC 命令
//!
//! 独立模块设计：数据仓库相关的 IPC 命令定义在此模块中，
//! 通过 core_ipc_commands! 宏注册到全局。
//!
//! 包含：
//! - 设置仓库 (settings_repo)
//! - 命令仓库 (command_repo)
//! - Pin 仓库 (pin_repo)

use crate::app::state::AppState;
use crate::core::command::command_custom::CustomCommand;
use crate::models::{Settings, ThemeMode};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

// ==================== 设置仓库 ====================

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
pub async fn get_appearance(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
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

// ==================== 命令仓库 ====================

#[tauri::command]
pub async fn add_command(
    state: State<'_, Arc<AppState>>,
    cmd: CustomCommand,
) -> Result<String, String> {
    let id = cmd.id.clone();
    state.command_repo.add(cmd).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub async fn list_commands(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<CustomCommand>, String> {
    Ok(state.command_repo.list())
}

#[tauri::command]
pub async fn remove_command(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    state.command_repo.remove(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_command(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    use crate::platform::windows;
    let Some(cmd) = state.command_repo.get(&id) else {
        return Err("Command not found".into());
    };
    let args = cmd.args.clone();
    if cmd.run_as_admin {
        windows::shell::launch_as_admin(&cmd.command, &args).map_err(|e| e.to_string())?;
    } else {
        windows::shell::launch(&cmd.command, &args).map_err(|e| e.to_string())?;
    }
    state
        .command_repo
        .record_used(&id, chrono::Utc::now().timestamp())
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ==================== Pin 仓库 ====================

#[tauri::command]
pub async fn list_pinned(state: State<'_, Arc<AppState>>) -> Result<Vec<String>, String> {
    Ok(state.pin_repo.list())
}

#[tauri::command]
pub async fn pin_item(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    state.pin_repo.add(id);
    Ok(())
}

#[tauri::command]
pub async fn unpin_item(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    state.pin_repo.remove(&id);
    Ok(())
}
