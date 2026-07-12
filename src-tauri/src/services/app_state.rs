//! 全局 App 状态 - 由 tauri::Builder.manage 注入

use crate::engines::app_search::AppSearchEngine;
use crate::engines::command_search::CommandSearchEngine;
use crate::engines::file_search::FileSearchEngine;
use crate::repositories::*;
use crate::services::hotkey::HotkeyService;
use crate::services::search::SearchEngine;
use crate::services::window::WindowService;
use std::sync::Arc;
use std::sync::Mutex;
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

    pub is_dragging: Arc<Mutex<bool>>,
}
