//! MonoTools 库入口 - 由 main.rs 和 cli_main.rs 共用
//! 主要暴露：App 构建、Command 命令系统

pub mod app;
pub mod command;
pub mod engines;
pub mod error;
pub mod models;
pub mod platform;
pub mod repositories;
pub mod services;
pub mod types;
pub mod utils;

pub mod commands;

pub use error::{AppError, Result};
pub use types::{AppEntry, FileResult, SearchResult, StartupItem};

/// 启动 Tauri GUI 应用
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app::run_lib();
}
