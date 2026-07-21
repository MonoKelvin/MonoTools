//! 框架级 IPC 命令
//!
//! 本模块只包含**框架级**的 IPC 命令。
//! 所有业务模块的 IPC 命令都定义在各自模块中，
//! 通过 `app::modules::register_ipc_commands` 统一注册。
//!
//! **业务模块的 IPC 命令归属**：
//! - 搜索相关 → `crate::search_engine::ipc`
//! - 窗口相关 → `crate::services::window::ipc`
//! - 热键相关 → `crate::services::hotkey::ipc`
//! - 窗口监控 → `crate::services::window_monitor::ipc`
//! - 设置相关 → `crate::core::settings::ipc`
//! - 自定义命令 CRUD → `crate::core::command::ipc`
//! - 平台相关 → `crate::platform::windows::ipc`

use crate::app::state::AppState;
use crate::core::command;
use crate::core::config::ipc_events;
use std::sync::Arc;
use tauri::{Emitter, Manager, State};

/// 标记前端 UI 渲染完成,可以显示窗口.
///
/// 这是框架级命令，负责应用启动流程的关键节点。
/// 只做框架层面的事情：
/// 1. 显示窗口
/// 2. 设置 frontend_initialized 标志
/// 3. 发出 `frontend_ready` 事件，各业务模块监听此事件做自己的初始化
///
/// 业务逻辑（如索引构建）通过监听事件完成，本函数不直接引用任何业务模块。
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

        state
            .frontend_initialized
            .store(true, std::sync::atomic::Ordering::Release);
        log::info!("[boot] frontend_initialized 标志已设置");

        if let Some(w) = app.get_webview_window("search") {
            let _ = w.show();
            let _ = w.set_focus();
        }

        let _ = app.emit(ipc_events::FRONTEND_READY, ());

        #[cfg(feature = "recommend")]
        {
            let config = crate::recommend::RecommendInitConfig::default();
            crate::recommend::start_deferred(&app, config);
        }
    }
    Ok(())
}

/// 前端命令面板使用：列出全部已注册命令（不含别名）。
#[tauri::command]
pub async fn list_command_specs() -> Result<serde_json::Value, String> {
    let reg = crate::app::modules::build_command_registry();
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
) -> Result<command::CommandOutput, String> {
    use command::registry_dispatch;
    let ctx = state.build_command_context();
    let arg_list = args.unwrap_or_default();
    let reg = crate::app::modules::build_command_registry();
    registry_dispatch(&reg, &command_id, &arg_list, &ctx)
        .await
        .map_err(|e| e.to_string())
}

/// 注册框架级 IPC 命令到 Tauri builder
pub fn register_ipc_commands(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder.invoke_handler(tauri::generate_handler![
        frontend_ready,
        list_command_specs,
        dispatch_command,
    ])
}
