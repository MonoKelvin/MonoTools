//! 搜索模块初始化
//!
//! 负责搜索模块的启动、索引构建、USN 更新循环等业务逻辑。
//! app 层只调用本模块的 init() 函数，不关心具体实现细节。

use crate::core::config::ipc_events;
use crate::search_engine::app_search::AppSearchEngine;
use crate::search_engine::file_search::FileSearchEngine;
use crate::search_engine::service::SearchEngine;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Listener};

/// 搜索模块初始化参数
pub struct SearchInitParams {
    pub app_search: Arc<AppSearchEngine>,
    pub file_search: Arc<FileSearchEngine>,
    pub search_engine: Arc<SearchEngine>,
}

/// 启动文件索引构建和 USN 更新循环
///
/// 这是搜索模块的核心初始化逻辑，原本在 frontend_ready 中。
/// 现在抽离到 search_engine 模块，app 层只需调用此函数。
pub fn start_indexing(app: &AppHandle, params: SearchInitParams) {
    let fs_clone = params.file_search.clone();
    let app_clone = app.clone();
    let search_engine_clone = params.search_engine.clone();

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

        use crate::search_engine::start_update_loop;
        let fs_clone2 = fs_clone.clone();
        start_update_loop(
            move || fs_clone2.update_index(),
            std::time::Duration::from_secs(120),
        );

        let _ = search_engine_clone;
    });
}

/// 发送当前索引状态（用于 frontend_ready 时如果索引已构建完成）
pub fn emit_index_status(app: &AppHandle, search_engine: &SearchEngine) {
    let stats = search_engine.total_indexed();
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

/// 启动应用索引刷新（后台异步任务）
///
/// 原本在 builder.rs 的 setup 中，现在抽离到 search_engine 模块。
pub fn start_app_index_refresh(app: &AppHandle, app_search: Arc<AppSearchEngine>) {
    let app_search_for_refresh = app_search.clone();
    let app_handle_for_apps = app.clone();

    tauri::async_runtime::spawn(async move {
        #[cfg(debug_assertions)]
        log::info!("[boot] 后台应用索引刷新任务已 spawn");

        let _ = app_handle_for_apps.emit(
            ipc_events::INDEX_PROGRESS,
            serde_json::json!({
                "status": "building",
                "message": "正在加载应用列表...",
                "phase": "apps",
            }),
        );

        let app_handle_for_progress = app_handle_for_apps.clone();
        let result = app_search_for_refresh
            .refresh_index_incremental(move |count, phase| {
                let phase_label = match phase {
                    "common_start_menu" => "公共开始菜单",
                    "user_start_menu" => "用户开始菜单",
                    "desktop" => "桌面",
                    _ => phase,
                };
                let _ = app_handle_for_progress.emit(
                    ipc_events::INDEX_PROGRESS,
                    serde_json::json!({
                        "status": "building",
                        "message": format!("已加载 {} 个应用（{}）", count, phase_label),
                        "phase": "apps",
                        "apps": count,
                        "apps_phase": phase,
                    }),
                );
            })
            .await;

        match result {
            Ok(()) => {
                let total = app_search_for_refresh.total();
                log::info!("应用索引刷新完成: {} 个应用", total);
                let _ = app_handle_for_apps.emit(
                    ipc_events::INDEX_PROGRESS,
                    serde_json::json!({
                        "status": "completed",
                        "message": format!("已加载 {} 个应用", total),
                        "phase": "apps",
                        "apps": total,
                    }),
                );
            }
            Err(e) => {
                log::warn!("应用索引刷新失败: {}", e);
                let _ = app_handle_for_apps.emit(
                    ipc_events::INDEX_PROGRESS,
                    serde_json::json!({
                        "status": "error",
                        "message": format!("应用列表加载失败: {}", e),
                        "phase": "apps",
                    }),
                );
            }
        }
    });
}

/// 搜索模块完整初始化（在 builder setup 中调用）
///
/// 包含：
/// - 应用索引刷新（后台异步）
/// - 注册 FRONTEND_READY 事件监听器，前端就绪后启动文件索引构建
pub fn init(app: &AppHandle, params: SearchInitParams) {
    start_app_index_refresh(app, params.app_search.clone());

    // 监听前端就绪事件，启动文件索引构建
    let app_clone = app.clone();
    let file_search = params.file_search.clone();
    let search_engine = params.search_engine.clone();

    app.listen(ipc_events::FRONTEND_READY, move |_event| {
        let params_inner = SearchInitParams {
            app_search: params.app_search.clone(),
            file_search: file_search.clone(),
            search_engine: search_engine.clone(),
        };
        if !file_search.is_indexing() {
            emit_index_status(&app_clone, &search_engine);
        }
        start_indexing(&app_clone, params_inner);
    });
}
