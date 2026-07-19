//! Tauri IPC Commands - 前端 ↔ 后端
//!
//! 这些命令仅在 GUI 模式下使用，CLI 模式不依赖它们。
//! 纯后端功能应该通过 Command trait + CommandRegistry 实现，
//! 这样 CLI 和 GUI 都能共用。

use crate::app::state::AppState;
use crate::core::command::command_custom::CustomCommand;
use crate::core::config::{ipc_events, window};
use crate::models::Settings;
use crate::platform::windows::shell;
use crate::search_engine::models::{SearchAction, SearchResult};
use std::sync::Arc;
use tauri::{Emitter, LogicalSize, Manager, State};

/// search_cmd 设置默认返回限制, 避免一次性返回过多结果导致 IPC 序列化阻塞.
/// 客户端可传 `options.limit` 覆盖此值, 虚拟滚动列表可通过 search_more_cmd 分页加载更多.
/// 渲染侧由 vue-virtual-scroller 处理百万级数据, 不会因为 list 大而卡顿.
///
/// 注: 文件引擎空查询仍受 `search::ALL_FILES_EMPTY_QUERY_CAP` 实际限制
/// (防止索引极大时单帧 IPC 阻塞, 详见 [file_search::FileSearchEngine::all_files]).
#[tauri::command]
pub async fn search_cmd(
    state: State<'_, Arc<AppState>>,
    query: String,
    options: Option<serde_json::Value>,
) -> Result<Vec<SearchResult>, String> {
    let default_limit = if query.is_empty() {
        crate::core::config::search::EMPTY_QUERY_LIMIT
    } else {
        crate::core::config::search::DEFAULT_LIMIT
    };
    let limit = options
        .as_ref()
        .and_then(|o| o.get("limit").and_then(|v| v.as_u64()))
        .map(|n| n.min(crate::core::config::search::MAX_LIMIT as u64) as u32)
        .unwrap_or(default_limit);
    let results = state.search_engine.search(&query, limit);
    Ok(results)
}

/// 分页搜索: 兼容性保留, 给前端的"显示更多"用.
#[tauri::command]
pub async fn search_more_cmd(
    state: State<'_, Arc<AppState>>,
    query: String,
    after_id: i64,
    options: Option<serde_json::Value>,
) -> Result<Vec<SearchResult>, String> {
    let limit = options
        .as_ref()
        .and_then(|o| o.get("limit").and_then(|v| v.as_u64()))
        .map(|n| n.min(u32::MAX as u64) as u32)
        .unwrap_or(200);
    let results = state.search_engine.search_after(&query, after_id, limit);
    Ok(results)
}

#[tauri::command]
pub async fn build_file_index(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let state_clone = Arc::clone(&state);
    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        let _ = app_clone.emit(
            ipc_events::INDEX_PROGRESS,
            serde_json::json!({
                "status": "building",
                "message": "正在检测 NTFS 卷...",
                "files": 0usize,
                "volumes": 0usize,
            }),
        );

        let app_for_progress = app_clone.clone();
        let build_result = state_clone
            .file_search
            .build_index_with_volume_progress(move |volume, idx, cumulative, total_volumes| {
                let drive = crate::platform::windows::usn::drive_label(volume);
                let msg = if total_volumes == 0 {
                    format!("正在索引 {}", drive)
                } else {
                    format!(
                        "索引中 {}/{} · {} — 已索引 {} 个文件",
                        idx, total_volumes, drive, cumulative
                    )
                };
                let _ = app_for_progress.emit(
                    ipc_events::INDEX_PROGRESS,
                    serde_json::json!({
                        "status": "building",
                        "message": msg,
                        "files": cumulative,
                        "volumes": total_volumes,
                        "current_volume": drive,
                        "current_index": idx,
                    }),
                );
            })
            .await;

        match build_result {
            Err(e) => {
                log::error!("索引构建失败: {}", e);
                let _ = app_clone.emit(
                    ipc_events::INDEX_PROGRESS,
                    serde_json::json!({
                        "status": "error",
                        "message": e.to_string(),
                    }),
                );
            }
            Ok(_) => {
                let stats = state_clone.search_engine.total_indexed();
                let _ = app_clone.emit(
                    ipc_events::INDEX_PROGRESS,
                    serde_json::json!({
                        "status": "completed",
                        "files": stats.files,
                        "apps": stats.apps,
                        "commands": stats.commands,
                        "volumes": 0usize,
                        "current_volume": "",
                        "current_index": 0,
                    }),
                );
            }
        }
    });

    Ok("索引构建已启动".to_string())
}

#[tauri::command]
pub async fn get_index_status(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let stats = state.search_engine.total_indexed();
    Ok(serde_json::json!({
        "files": stats.files,
        "apps": stats.apps,
        "commands": stats.commands,
    }))
}

/// 标记前端 UI 渲染完成,可以显示窗口.
#[tauri::command]
pub async fn frontend_ready(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    log::info!("[boot] frontend_ready: 显示窗口并标记初始化完成");
    use parking_lot::Mutex;
    use std::sync::OnceLock;
    static SHOWN: OnceLock<Mutex<bool>> = OnceLock::new();
    let m = SHOWN.get_or_init(|| Mutex::new(false));
    let mut shown = m.lock();
    if !*shown {
        *shown = true;
        drop(shown);

        // 标记前端已初始化, 允许托盘点击显示窗口
        state.frontend_initialized.store(true, std::sync::atomic::Ordering::Release);
        log::info!("[boot] frontend_initialized 标志已设置");

        if let Some(w) = app.get_webview_window("search") {
            let _ = w.show();
            let _ = w.set_focus();
        }
        if !state.file_search.is_indexing() {
            let stats = state.search_engine.total_indexed();
            let _ = app.emit(
                ipc_events::INDEX_PROGRESS,
                serde_json::json!({
                    "status": "completed",
                    "files": stats.files,
                    "apps": stats.apps,
                    "commands": stats.commands,
                }),
            );
        }

        // 窗口已显示后再启动文件索引构建 (非阻塞 UI).
        let fs_clone = state.file_search.clone();
        let app_clone = app.clone();
        tauri::async_runtime::spawn(async move {
            #[cfg(debug_assertions)]
            log::info!("[boot] 文件索引构建任务已 spawn (frontend_ready 后)");
            log::info!("[boot] 启动文件索引构建...");
            let _ = app_clone.emit(
                ipc_events::INDEX_PROGRESS,
                serde_json::json!({
                    "status": "building",
                    "message": "正在检测盘符...",
                    "phase": "files",
                }),
            );
            let start = std::time::Instant::now();
            let app_for_progress = app_clone.clone();
            let res = fs_clone
                .build_index_with_volume_progress(move |volume, idx, cumulative, total_volumes| {
                    let drive = crate::platform::windows::usn::drive_label(volume);
                    let msg = if total_volumes == 0 {
                        format!("正在索引 {}", drive)
                    } else {
                        format!(
                            "正在索引 {}（{}/{}） — 已累计 {} 个文件",
                            drive, idx, total_volumes, cumulative
                        )
                    };
                    let _ = app_for_progress.emit(
                        ipc_events::INDEX_PROGRESS,
                        serde_json::json!({
                            "status": "building",
                            "message": msg,
                            "phase": "files",
                            "files": cumulative,
                            "volumes": total_volumes,
                            "current_volume": drive,
                            "current_index": idx,
                        }),
                    );
                })
                .await;
            match res {
                Err(e) => {
                    log::error!("[boot] 文件索引构建失败: {}", e);
                    let _ = app_clone.emit(
                        ipc_events::INDEX_PROGRESS,
                        serde_json::json!({
                            "status": "error",
                            "message": format!("索引构建失败: {}", e),
                            "phase": "files",
                        }),
                    );
                }
                Ok(_) => {
                    log::info!("[boot] 文件索引构建完成，耗时 {:?}", start.elapsed());
                    let total = fs_clone.total();
                    let _ = app_clone.emit(
                        ipc_events::INDEX_PROGRESS,
                        serde_json::json!({
                            "status": "completed",
                            "message": "索引构建完成",
                            "phase": "files",
                            "files": total,
                        }),
                    );
                }
            }

            // 启动 USN 增量更新循环 (每 120s 检查一次).
            use crate::search_engine::start_update_loop;
            let fs_clone2 = fs_clone.clone();
            start_update_loop(
                move || fs_clone2.update_index(),
                std::time::Duration::from_secs(120),
            );
        });
    }
    Ok(())
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
        state
            .stats_repo
            .record_launch(path, &item.title, chrono::Utc::now().timestamp());
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
pub async fn remove_command(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
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
pub async fn set_window_height(app: tauri::AppHandle, height: u32) -> Result<(), String> {
    let Some(w) = app.get_webview_window("search") else {
        return Ok(());
    };
    let height = height.clamp(window::MIN_HEIGHT, window::MAX_HEIGHT);
    let _ = w.set_size(LogicalSize::new(window::DEFAULT_WIDTH, height as f64));
    Ok(())
}

/// 应用层提供的"开始拖拽窗口"命令。
#[tauri::command]
pub async fn start_dragging(
    state: tauri::State<'_, Arc<AppState>>,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    if let Ok(mut dragging) = state.is_dragging.lock() {
        *dragging = true;
    }

    window.start_dragging().map_err(|e| {
        println!("[ERROR] Failed to start dragging: {}", e);
        format!("Failed to start dragging: {e}")
    })?;

    Ok(())
}

/// 设置拖拽状态
#[tauri::command]
pub async fn set_dragging(
    state: tauri::State<'_, Arc<AppState>>,
    dragging: bool,
) -> Result<(), String> {
    if let Ok(mut is_dragging) = state.is_dragging.lock() {
        *is_dragging = dragging;
    }
    Ok(())
}

/// 退出整个应用。
#[tauri::command]
pub async fn quit_app(app: tauri::AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

/// 前端命令面板使用：列出全部已注册命令（不含别名）。
#[tauri::command]
pub async fn list_command_specs() -> Result<serde_json::Value, String> {
    use crate::core::command::build_default_registry;
    let reg = build_default_registry();
    let names = reg.main_names();
    let mut specs: Vec<serde_json::Value> = Vec::with_capacity(names.len());
    for name in names {
        let Some(cmd) = reg.lookup(&name) else {
            continue;
        };
        let spec = cmd.spec();
        specs.push(serde_json::json!({
            "name": spec.name,
            "description": spec.description,
            "aliases": spec.aliases,
            "usage": spec.usage,
        }));
    }
    Ok(serde_json::Value::Array(specs))
}

/// 前端命令面板使用：精确路由到后端命令。
#[tauri::command]
pub async fn dispatch_command(
    state: State<'_, Arc<AppState>>,
    command_id: String,
    args: Option<Vec<String>>,
) -> Result<crate::core::command::CommandOutput, String> {
    use crate::core::command::{registry_dispatch, CommandContext};
    let ctx = CommandContext::from_app_state(&state);
    let arg_list = args.unwrap_or_default();
    registry_dispatch(&command_id, &arg_list, &ctx)
        .await
        .map_err(|e| e.to_string())
}

/// 获取可执行文件图标 (base64 编码 PNG).
#[tauri::command]
pub async fn get_app_icon(path: String) -> Result<Option<String>, String> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

    let resolved_path =
        crate::platform::windows::shell::resolve_shortcut(&std::path::PathBuf::from(&path))
            .unwrap_or(std::path::PathBuf::from(&path));
    let resolved_path_str = resolved_path.to_string_lossy().to_string();

    {
        let cache = crate::platform::windows::icon::cache_snapshot();
        if let Some(cached) = cache.get(&resolved_path_str.to_lowercase()) {
            return Ok(cached.as_ref().map(|v| BASE64.encode(v)));
        }
    }

    let path_clone = resolved_path_str.clone();
    let bytes = tokio::task::spawn_blocking(move || {
        crate::platform::windows::icon::get_or_extract_cached(&path_clone).unwrap_or(None)
    })
    .await
    .map_err(|e| format!("icon join error: {}", e))?;

    if let Some(ref b) = bytes {
        Ok(Some(BASE64.encode(b)))
    } else {
        log::debug!(
            "[icon-ipc] extraction returned None for path={}",
            resolved_path_str
        );
        Ok(None)
    }
}

/// 批量获取图标 (base64 编码 PNG 数组).
#[tauri::command]
pub async fn get_app_icons_batch(paths: Vec<String>) -> Result<Vec<Option<String>>, String> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

    let total = paths.len();
    if total == 0 {
        return Ok(Vec::new());
    }

    let resolved: Vec<String> = paths
        .iter()
        .map(|p| {
            crate::platform::windows::shell::resolve_shortcut(&std::path::PathBuf::from(p))
                .unwrap_or_else(|_| std::path::PathBuf::from(p))
                .to_string_lossy()
                .to_string()
        })
        .collect();

    let mut out: Vec<Option<String>> = vec![None; total];
    let mut pending: Vec<(usize, String)> = Vec::new();
    {
        let cache = crate::platform::windows::icon::cache_snapshot();
        for (i, p) in resolved.iter().enumerate() {
            if let Some(cached) = cache.get(&p.to_lowercase()) {
                out[i] = cached.as_ref().map(|v| BASE64.encode(v));
            } else {
                pending.push((i, p.clone()));
            }
        }
    }

    let cache_hits = out.iter().filter(|x| x.is_some()).count();
    if pending.is_empty() {
        log::info!("[icon] batch fetched {} paths (all cache hit)", total);
        return Ok(out);
    }

    let concurrency = std::cmp::min(
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4),
        8,
    );
    let concurrency = concurrency.max(2);

    let chunk_size = pending.len().div_ceil(concurrency);
    let mut chunks: Vec<Vec<(usize, String)>> = Vec::with_capacity(concurrency);
    for chunk in pending.chunks(chunk_size) {
        chunks.push(chunk.to_vec());
    }

    let mut handles = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        let handle = tokio::task::spawn_blocking(move || {
            let mut results = Vec::with_capacity(chunk.len());
            for (idx, p) in chunk {
                let bytes =
                    crate::platform::windows::icon::get_or_extract_cached(&p).unwrap_or(None);
                results.push((idx, bytes));
            }
            results
        });
        handles.push(handle);
    }

    let all_results = futures::future::join_all(handles).await;

    for result in all_results {
        match result {
            Ok(chunk_results) => {
                for (idx, bytes) in chunk_results {
                    out[idx] = bytes.as_ref().map(|v| BASE64.encode(v));
                }
            }
            Err(e) => {
                log::error!("[icon] batch chunk join error: {}", e);
            }
        }
    }

    log::info!(
        "[icon] batch fetched {} paths, cache_hits={}, extracted={}, success={}, concurrency={}",
        total,
        cache_hits,
        pending.len(),
        out.iter().filter(|x| x.is_some()).count(),
        concurrency
    );

    Ok(out)
}

/// 列出全部已 pin 的 id (按用户添加顺序, 最新在前).
#[tauri::command]
pub async fn list_pinned(state: State<'_, Arc<AppState>>) -> Result<Vec<String>, String> {
    Ok(state.pin_repo.list())
}

/// 添加一个 id 到 pin 列表 (已存在则去重并挪到头部).
#[tauri::command]
pub async fn pin_item(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    state.pin_repo.add(id);
    Ok(())
}

/// 从 pin 列表移除一个 id.
#[tauri::command]
pub async fn unpin_item(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    state.pin_repo.remove(&id);
    Ok(())
}

/// 打开文件所在位置 (在资源管理器中选中文件).
#[tauri::command]
pub async fn open_file_location(path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    crate::platform::windows::shell::open_path(&p).map_err(|e| e.to_string())
}

/// 显示文件属性对话框.
#[tauri::command]
pub async fn show_file_properties(path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    crate::platform::windows::shell::show_file_properties(&p).map_err(|e| e.to_string())
}

/// 删除文件到回收站.
#[tauri::command]
pub async fn delete_file_to_recycle_bin(path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    crate::platform::windows::shell::delete_to_recycle_bin(&p).map_err(|e| e.to_string())
}

/// 获取 Windows 系统当前主题模式 ("light" 或 "dark").
/// 读取注册表 HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize\AppsUseLightTheme.
#[tauri::command]
pub fn get_system_theme() -> Result<String, String> {
    #[cfg(windows)]
    {
        use std::process::Command;

        let output = Command::new("reg")
            .args([
                "query",
                r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize",
                "/v",
                "AppsUseLightTheme",
            ])
            .output()
            .map_err(|e| format!("无法执行 reg query: {}", e))?;

        if !output.status.success() {
            // 如果注册表键不存在（如旧版 Windows），默认 dark
            return Ok("dark".to_string());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        // reg query 输出格式: "    AppsUseLightTheme    REG_DWORD    0x1"
        for line in stdout.lines() {
            if line.contains("AppsUseLightTheme") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let value = parts[2];
                    // 0x0 = dark, 0x1 = light
                    if value == "0x0" || value == "0" {
                        return Ok("dark".to_string());
                    } else {
                        return Ok("light".to_string());
                    }
                }
            }
        }

        // 未找到值，默认 dark
        Ok("dark".to_string())
    }

    #[cfg(not(windows))]
    {
        // 非 Windows 平台默认 dark
        Ok("dark".to_string())
    }
}

/// 获取窗口监控状态 (当前激活应用 + 最近应用历史).
#[tauri::command]
pub async fn get_window_monitor_state(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let monitor = state.window_monitor.lock().map_err(|e| e.to_string())?;
    let snapshot = monitor.snapshot();
    Ok(serde_json::json!({
        "activeAppPath": snapshot.active_app_path,
        "activeAppTitle": snapshot.active_app_title,
        "recentApps": snapshot.recent_apps.iter().map(|a| serde_json::json!({
            "path": &a.path,
            "title": &a.title,
        })).collect::<Vec<_>>(),
    }))
}

/// 设置是否跟随系统主题.
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
