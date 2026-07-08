//! Tauri IPC Commands - 前端 ↔ 后端

use crate::models::{CustomCommand, SearchResult, Settings};
use crate::services::app_state::AppState;
use crate::models::SearchAction;
use crate::platform::windows::shell;
use std::sync::Arc;
use tauri::{LogicalSize, Manager, State};

const WINDOW_DEFAULT_WIDTH: f64 = 680.0;

#[tauri::command]
pub async fn search_cmd(
    state: State<'_, Arc<AppState>>,
    query: String,
) -> Result<Vec<SearchResult>, String> {
    let mut results: Vec<SearchResult> = Vec::new();
    let limit = 20u32;
    results.extend(state.app_search.search(&query, limit));
    results.extend(state.file_search.search(&query, limit));
    results.extend(state.command_search.search(&query, limit));
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(limit as usize);
    Ok(results)
}

#[tauri::command]
pub async fn execute_result(
    _app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    item: SearchResult,
) -> Result<(), String> {
    shell::launch_str(&item).map_err(|e| e.to_string())?;
    state.app_search.record_launch(&item.title);
    if let SearchAction::Launch(path) = &item.action {
        state.stats_repo.record_launch(path, &item.title, chrono::Utc::now().timestamp());
    }
    if let Some(h) = state.window.handle_for("search") {
        h.hide();
    }
    Ok(())
}

#[tauri::command]
pub async fn show_window(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.window.show().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn hide_window(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.window.hide().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_window(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.window.toggle(&app).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn register_hotkey_cmd(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    hotkey: String,
) -> Result<String, String> {
    state
        .hotkey
        .register(&hotkey, &app)
        .await
        .map_err(|e| e.to_string())?;
    Ok(hotkey)
}

#[tauri::command]
pub async fn unregister_hotkey(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    hotkey: String,
) -> Result<(), String> {
    state
        .hotkey
        .unregister(&hotkey, &app)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_current_hotkey(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    Ok(state.hotkey.current().unwrap_or_default())
}

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
pub async fn list_commands(state: State<'_, Arc<AppState>>) -> Result<Vec<CustomCommand>, String> {
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
pub async fn run_command(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
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
    use crate::models::ThemeMode;
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
    let res = state
        .settings_repo
        .update(Box::new(move |s| {
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
    app: tauri::AppHandle,
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
    }
    Ok(())
}

#[tauri::command]
pub async fn set_window_height(
    app: tauri::AppHandle,
    height: u32,
) -> Result<(), String> {
    let Some(w) = app.get_webview_window("search") else {
        return Ok(());
    };
    if height < 180 || height > 900 {
        return Ok(());
    }
    // 只改高度，宽度固定来自配置，永远不重新读取当前 width
    let _ = w.set_size(LogicalSize::new(WINDOW_DEFAULT_WIDTH, height as f64));
    Ok(())
}

/// 应用层提供的"开始拖拽窗口"命令——前端 header 空白区域会触发。
#[tauri::command]
pub async fn start_dragging(window: tauri::WebviewWindow) -> Result<(), String> {
    window
        .start_dragging()
        .map_err(|e| format!("Failed to start dragging: {e}"))?;
    Ok(())
}

/// 退出整个应用（被 menu 关闭 / Quit 等调用）。
#[tauri::command]
pub async fn quit_app(app: tauri::AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}
