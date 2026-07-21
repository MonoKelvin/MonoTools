//! 命令系统 IPC 命令
//!
//! 本模块只包含自定义命令 CRUD 相关的 IPC 命令。
//!
//! 命令列表查询和命令派发属于框架级组装逻辑，
//! 因为需要汇总所有业务模块的命令，已移到 `crate::app::ipc`。

use crate::app::state::AppState;
use crate::core::command::CustomCommand;
use std::sync::Arc;
use tauri::State;

// ==================== 自定义命令 CRUD ====================

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

/// 注册命令模块的 IPC 命令到 Tauri builder
pub fn register_ipc_commands(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder.invoke_handler(tauri::generate_handler![
        list_commands,
        add_command,
        remove_command,
        run_command,
    ])
}
