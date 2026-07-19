//! GUI 应用状态 - 由 tauri::Builder.manage 注入
//!
//! 仅在 GUI 模式下使用，CLI 模式使用 CommandContext::new_headless()。

use crate::repositories::*;
use crate::search_engine::app_search::AppSearchEngine;
use crate::search_engine::command_search::CommandSearchEngine;
use crate::search_engine::file_search::FileSearchEngine;
use crate::search_engine::SearchEngine;
use crate::services::hotkey::HotkeyService;
use crate::services::window::WindowService;
use crate::services::window_monitor::WindowMonitorService;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use tauri::AppHandle;

pub struct AppState {
    pub app: AppHandle,
    pub settings_repo: Arc<dyn SettingsRepo>,
    pub command_repo: Arc<dyn CommandRepo>,
    pub stats_repo: Arc<StatsRepo>,
    pub pin_repo: Arc<PinRepo>,

    pub app_search: Arc<AppSearchEngine>,
    pub command_search: Arc<CommandSearchEngine>,
    pub file_search: Arc<FileSearchEngine>,
    pub search_engine: Arc<SearchEngine>,

    pub hotkey: Arc<HotkeyService>,
    pub window: Arc<WindowService>,
    pub window_monitor: Arc<Mutex<WindowMonitorService>>,

    pub is_dragging: Arc<Mutex<bool>>,

    /// 前端 UI 渲染完成标志. 为 true 时表示 frontend_ready 已被调用, 窗口可以安全显示.
    pub frontend_initialized: Arc<AtomicBool>,
}
