//! GUI 应用模块
//!
//! 包含 Tauri 应用构建、IPC 命令、应用状态等 GUI 专属代码。
//! CLI 模式不依赖此模块。

pub mod builder;
pub mod ipc;
pub mod state;

pub use state::AppState;

/// 启动 Tauri GUI 应用
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    builder::run();
}
