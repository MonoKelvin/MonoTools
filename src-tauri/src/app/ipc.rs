//! 框架级 IPC 命令
//!
//! 本模块只包含**框架级**的 IPC 命令。
//! 所有业务模块的 IPC 命令都定义在各自模块中，
//! 通过 `app::modules::core_ipc_commands!` 宏统一注册。
//!
//! **业务模块的 IPC 命令归属**：
//! - 搜索相关 → `crate::search_engine::ipc`
//! - 窗口相关 → `crate::services::window::ipc`
//! - 热键相关 → `crate::services::hotkey::ipc`
//! - 窗口监控 → `crate::services::window_monitor::ipc`
//! - 仓库相关 → `crate::repositories::ipc`
//! - 平台相关 → `crate::platform::windows::ipc`
//! - 命令系统 → `crate::core::command::ipc`

use crate::app::state::AppState;
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

        state.frontend_initialized.store(true, std::sync::atomic::Ordering::Release);
        log::info!("[boot] frontend_initialized 标志已设置");

        if let Some(w) = app.get_webview_window("search") {
            let _ = w.show();
            let _ = w.set_focus();
        }

        let _ = app.emit(ipc_events::FRONTEND_READY, ());
    }
    Ok(())
}
