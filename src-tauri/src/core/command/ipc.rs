//! 命令系统 IPC 命令
//!
//! 独立模块设计：命令系统相关的 IPC 命令定义在此模块中，
//! 通过 core_ipc_commands! 宏注册到全局。

use crate::app::state::AppState;
use crate::core::command::{self, CommandContext};
use std::sync::Arc;
use tauri::State;

/// 前端命令面板使用：列出全部已注册命令（不含别名）。
#[tauri::command]
pub async fn list_command_specs() -> Result<serde_json::Value, String> {
    use command::build_default_registry;
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
) -> Result<command::CommandOutput, String> {
    use command::registry_dispatch;
    let ctx = CommandContext::from_app_state(&state);
    let arg_list = args.unwrap_or_default();
    registry_dispatch(&command_id, &arg_list, &ctx)
        .await
        .map_err(|e| e.to_string())
}
