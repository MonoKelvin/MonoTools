//! 搜索引擎模块 —— 应用搜索、文件搜索、命令搜索
//!
//! 本模块包含完整的搜索相关功能：
//! - `models/`: 搜索相关数据模型
//! - `app_search/`: 应用搜索引擎
//! - `command_search.rs`: 自定义命令搜索引擎
//! - `file_search.rs`: 文件搜索引擎
//! - `stats.rs`: 应用使用统计（启动次数、评分排序）
//! - `pin.rs`: 用户固定到首页的项目
//! - `search_source.rs`: 搜索源 trait（可扩展）
//! - `service.rs`: SearchEngine 协调服务（合并多源结果）
//! - `commands.rs`: 搜索相关命令（search, index）
//! - `ipc.rs`: 搜索模块的 Tauri IPC 命令

pub mod app_search;
pub mod command_search;
pub mod file_search;
pub mod models;
pub mod search_source;
pub mod stats;
pub mod pin;

pub mod service;
pub mod commands;
pub mod ipc;
pub mod init;

pub use file_search::FileSearchEngine;
pub use file_search::start_update_loop;
pub use search_source::SearchSource;
pub use service::SearchEngine;
pub use stats::{AppStat, StatsRepo};
pub use pin::PinRepo;
pub use init::{SearchInitParams, start_indexing, emit_index_status};
