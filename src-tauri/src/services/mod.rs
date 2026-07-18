//! 服务层
//!
//! 搜索相关服务已迁移至 `crate::search_engine`，
//! GUI 应用状态已迁移至 `crate::app::state`，
//! 这里保留非搜索的通用服务。

pub mod hotkey;
pub mod storage;
pub mod window;
pub mod window_monitor;

pub use hotkey::HotkeyService;
pub use storage::StorageService;
pub use window::WindowService;
pub use window_monitor::WindowMonitorService;

// 搜索服务 —— 从 search_engine 模块重新导出（向后兼容）
pub use crate::search_engine::SearchEngine;
