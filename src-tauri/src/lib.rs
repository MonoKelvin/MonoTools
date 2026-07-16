//! MonoTools 库入口 - 由 main.rs 和 cli_main.rs 共用
//! 主要暴露：App 构建、Command 命令系统

pub mod app;
pub mod core;
pub mod models;
pub mod platform;
pub mod repositories;
pub mod search_engine;
pub mod services;
pub mod utils;

pub use crate::core::error::{AppError, Result};
pub use crate::search_engine::models::{AppEntry, FileResult, SearchResult};

/// 启动 Tauri GUI 应用
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app::run();
}
