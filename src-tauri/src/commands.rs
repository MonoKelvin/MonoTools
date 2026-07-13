//! Tauri IPC Commands - 前端 ↔ 后端

use crate::config::{ipc_events, window};
use crate::models::SearchAction;
use crate::models::{CustomCommand, SearchResult, Settings};
use crate::platform::windows::shell;
use crate::services::app_state::AppState;
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
    log::info!("[search] search_cmd called with query='{}'", query);
    let default_limit = if query.is_empty() {
        crate::config::search::EMPTY_QUERY_LIMIT
    } else {
        crate::config::search::DEFAULT_LIMIT
    };
    let limit = options
        .as_ref()
        .and_then(|o| o.get("limit").and_then(|v| v.as_u64()))
        .map(|n| n.min(crate::config::search::MAX_LIMIT as u64) as u32)
        .unwrap_or(default_limit);
    let results = state.search_engine.search(&query, limit);
    log::info!("[search] search_cmd returned {} results", results.len());
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
        .unwrap_or(200); // 默认 page size, 与旧 LOAD_MORE_LIMIT=50 接近
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
        // 显式声明驱动盘符探测: 让懒枚举 NtfsIndexer 在后台触发.
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
///
/// 我们**不**在 Tauri 启动完成后立即显示窗口: 原因是 webview 拿到的前端 bundle
/// 在 cold-start 时要解析 + 执行, 此时窗口已"visible=true"会出现短暂白屏.
/// 因此启动时窗口 visible=false; 当前端根 mount 完成 + 首屏数据回来后,
/// 调用本 IPC 让 Rust 显式 show 窗口.
///
/// 一次性触发, 反复调用幂等.
#[tauri::command]
pub async fn frontend_ready(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    log::info!("[boot] frontend_ready: 显示窗口并标记初始化完成");
    // 无论前端主动调用几次, 都只触发一次窗口显示.
    use parking_lot::Mutex;
    use std::sync::OnceLock;
    static SHOWN: OnceLock<Mutex<bool>> = OnceLock::new();
    let m = SHOWN.get_or_init(|| Mutex::new(false));
    let mut shown = m.lock();
    if !*shown {
        *shown = true;
        drop(shown);
        if let Some(w) = app.get_webview_window("search") {
            let _ = w.show();
            let _ = w.set_focus();
        }
        // 前端 ready 时, 若文件索引**未在构建中**, 推一次当前统计让 ActionBar 显示
        // "已索引 N 个文件"; 若正在构建则不 emit, 避免把 building 状态覆盖成 completed
        // (索引任务自身会在完成时 emit completed).
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
        // 置顶时立即显示并聚焦窗口，取消置顶时立即隐藏窗口
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
    // 高度上下界, 防止前端误算导致窗口过大/过小.
    let height = height.clamp(window::MIN_HEIGHT, window::MAX_HEIGHT);
    // 只改高度，宽度固定为 window::DEFAULT_WIDTH, 永远不重新读取当前 width
    let _ = w.set_size(LogicalSize::new(window::DEFAULT_WIDTH, height as f64));
    Ok(())
}

/// 应用层提供的"开始拖拽窗口"命令——前端 header 空白区域会触发。
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

/// 设置拖拽状态（用于拖拽结束后重置状态）
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

/// 退出整个应用（被 menu 关闭 / Quit 等调用）。
#[tauri::command]
pub async fn quit_app(app: tauri::AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

/// 前端命令面板使用：列出全部已注册命令（不含别名）。
///
/// 返回 `Vec<CommandSpec>` 序列化后的精简结构（与 Rust 端 [`crate::command::CommandSpec`] 字段对齐）。
/// 只返回主命令名（主键），别名在 dispatch 时自动解析。
#[tauri::command]
pub async fn list_command_specs() -> Result<serde_json::Value, String> {
    use crate::command::build_default_registry;
    let reg = build_default_registry();
    // 只遍历主命令（cmds key set），跳过别名
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
///
/// 前端 `src/commands/store.ts::execute()` 唯一调用入口。
#[tauri::command]
pub async fn dispatch_command(
    state: State<'_, Arc<AppState>>,
    command_id: String,
    args: Option<Vec<String>>,
) -> Result<crate::command::CommandOutput, String> {
    use crate::command::{registry_dispatch, CommandContext};
    let ctx = CommandContext::from_app_state(&state);
    let arg_list = args.unwrap_or_default();
    registry_dispatch(&command_id, &arg_list, &ctx)
        .await
        .map_err(|e| e.to_string())
}

/// 获取可执行文件图标 (base64 编码 PNG).
///
/// 用于应用搜索结果展示:
/// - 前端拿到 base64 字符串后可以直接 `<img src="data:image/png;base64,...">`.
/// - 返回 `Ok(None)` 表示提取失败 (文件不存在 / 非 PE / 访问被拒 / 空白图标), 前端降级到 Lucide 通用图标.
/// - 内部已经过 `parking_lot::Mutex<HashMap>` 缓存, 同路径重复调用 < 1ms.
/// - 自动解析 .lnk 快捷方式到目标 .exe 路径.
#[tauri::command]
pub async fn get_app_icon(path: String) -> Result<Option<String>, String> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

    log::debug!("[icon-ipc] get_app_icon called for path={}", path);

    let resolved_path =
        crate::platform::windows::shell::resolve_shortcut(&std::path::PathBuf::from(&path))
            .unwrap_or(std::path::PathBuf::from(&path));
    let resolved_path_str = resolved_path.to_string_lossy().to_string();

    if resolved_path_str != path {
        log::debug!(
            "[icon-ipc] resolved .lnk from {} to {}",
            path,
            resolved_path_str
        );
    }

    let bytes = crate::platform::windows::icon::get_or_extract_cached(&resolved_path_str)
        .map_err(|e| e.to_string())?;

    if let Some(ref b) = bytes {
        let base64_str = BASE64.encode(b);
        log::debug!(
            "[icon-ipc] extracted successfully, bytes={}, base64_length={}",
            b.len(),
            base64_str.len()
        );
        Ok(Some(base64_str))
    } else {
        log::debug!(
            "[icon-ipc] extraction returned None for path={}",
            resolved_path_str
        );
        Ok(None)
    }
}

/// 批量获取图标 (base64 编码 PNG 数组). 一次 IPC 拉 N 个图标, 减少 RTT 开销.
///
/// 关键优化: 旧版每个 AppResultItem 单独 invoke get_app_icon, 200 个结果 =
/// 200 次 IPC, 弱机上首屏 10+ 秒. 新版: 一次调用拉满 (通常 30-60 个),
/// 后续按需再追加.
///
/// 失败语义: 单个 path 失败 → 对应位置返回 None, 整体不报错.
/// 重复 path 自动通过内部 cache 复用, 不会重复抽取.
/// 自动解析 .lnk 快捷方式到目标 .exe 路径.
#[tauri::command]
pub async fn get_app_icons_batch(paths: Vec<String>) -> Result<Vec<Option<String>>, String> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    let mut out: Vec<Option<String>> = Vec::with_capacity(paths.len());
    for p in &paths {
        let resolved_path =
            crate::platform::windows::shell::resolve_shortcut(&std::path::PathBuf::from(p))
                .unwrap_or(std::path::PathBuf::from(p));
        let resolved_path_str = resolved_path.to_string_lossy().to_string();
        let bytes = crate::platform::windows::icon::get_or_extract_cached(&resolved_path_str)
            .map_err(|e| e.to_string())?;
        out.push(bytes.map(|v| BASE64.encode(&v)));
    }
    log::info!(
        "[icon] batch fetched {} paths, hits={}",
        paths.len(),
        out.iter().filter(|x| x.is_some()).count()
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
