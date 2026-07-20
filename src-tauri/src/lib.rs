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

// === 独立模块: PyBridge (通用 Rust-Python 桥接) ===
// 删除此模块: 去掉下面这行 + 对应 feature + 依赖即可
#[cfg(feature = "pybridge")]
pub mod pybridge;

// === 独立模块: Recommend (智能推荐) ===
// 删除此模块: 去掉下面这行 + 对应 feature + 依赖即可
#[cfg(feature = "recommend")]
pub mod recommend;

pub use crate::core::error::{AppError, Result};
pub use crate::search_engine::models::{AppEntry, FileResult, SearchResult};

/// 启动 Tauri GUI 应用
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app::run();
}
