//! 搜索模块 IPC 命令
//!
//! 独立模块设计：搜索相关的 IPC 命令定义在此模块中，
//! 通过 core_ipc_commands! 宏注册到全局。
//!
//! 注意：搜索是核心功能，命令需要访问 AppState 中的共享状态，
//! 这是合理的——app 层提供状态容器，业务模块提供命令实现。

use crate::app::state::AppState;
use crate::core::config::ipc_events;
use crate::platform::windows::shell;
use crate::search_engine::models::{SearchAction, SearchResult};
use std::sync::Arc;
use tauri::{Emitter, State};

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

/// 注册搜索模块的 IPC 命令到 Tauri builder
pub fn register_ipc_commands(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder.invoke_handler(tauri::generate_handler![
        search_cmd,
        search_more_cmd,
        execute_result,
        build_file_index,
        get_index_status,
        list_pinned,
        pin_item,
        unpin_item,
    ])
}
