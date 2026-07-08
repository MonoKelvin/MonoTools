//! Tauri IPC Commands - 前端 ↔ 后端

use crate::models::{CustomCommand, NewStartupItem, SearchResult, Settings, StartupItem};
use crate::services::app_state::AppState;
use crate::models::SearchAction;
use crate::platform::windows::shell;
use std::sync::Arc;
use tauri::State;

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
    results.extend(state.startup_search.search(&query, limit));
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
    // 通过 web_window_handle 获取底层窗口并隐藏
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
pub async fn list_startup(state: State<'_, Arc<AppState>>) -> Result<Vec<StartupItem>, String> {
    Ok(state.startup_search.list())
}

#[tauri::command]
pub async fn toggle_startup(
    state: State<'_, Arc<AppState>>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    let items = state.startup_search.list();
    if let Some(item) = items.into_iter().find(|i| i.id == id) {
        let result = match item.source {
            crate::models::StartupSource::RegistryRun => {
                if enabled {
                    crate::platform::windows::registry::write_run_key(false, &item.name, &item.command)
                } else {
                    crate::platform::windows::registry::write_run_key(
                        false,
                        &format!(".{}_disabled", item.name),
                        "",
                    )
                }
            }
            _ => Ok(()),
        };
        if let Err(e) = result {
            return Err(e.to_string());
        }
        state
            .startup_repo
            .update(
                &id,
                &StartupItem {
                    enabled,
                    ..item.clone()
                },
            )
            .map_err(|e| e.to_string())?;
        let _ = state.startup_search.refresh().await;
    }
    Ok(())
}

#[tauri::command]
pub async fn add_startup(
    state: State<'_, Arc<AppState>>,
    item: NewStartupItem,
) -> Result<String, String> {
    use crate::services::startup::StartupManager;
    let mgr = StartupManager::new(state.startup_repo.clone());
    let id = mgr.add(item).await.map_err(|e| e.to_string())?;
    state.startup_search.refresh().await.map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub async fn remove_startup(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    use crate::services::startup::StartupManager;
    let mgr = StartupManager::new(state.startup_repo.clone());
    mgr.remove(&id).await.map_err(|e| e.to_string())?;
    state.startup_search.refresh().await.map_err(|e| e.to_string())?;
    Ok(())
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
        .unwrap_or("#ff6b6b")
        .to_string();
    let res = state
        .settings_repo
        .update(Box::new(move |s| {
            s.theme = mode;
            s.accent_color = accent;
        }));
    res.map(|_| ()).map_err(|e| e.to_string())
}
