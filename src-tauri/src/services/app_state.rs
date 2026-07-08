//! 全局 App 状态 - 由 tauri::Builder.manage 注入

use crate::engines::app_search::AppSearchEngine;
use crate::engines::command_search::CommandSearchEngine;
use crate::engines::file_search::FileSearchService;
use crate::engines::startup_search::StartupSearchService;
use crate::repositories::*;
use crate::services::hotkey::HotkeyService;
use crate::services::window::WindowService;
use std::sync::Arc;
use tauri::AppHandle;

pub struct AppState {
    pub app: AppHandle,
    pub settings_repo: Arc<dyn SettingsRepo>,
    pub startup_repo: Arc<dyn StartupRepo>,
    pub command_repo: Arc<dyn CommandRepo>,
    pub stats_repo: Arc<StatsRepo>,

    pub app_search: Arc<AppSearchEngine>,
    pub startup_search: Arc<StartupSearchService>,
    pub command_search: Arc<CommandSearchEngine>,
    pub file_search: Arc<FileSearchService>,

    pub hotkey: Arc<HotkeyService>,
    pub window: Arc<WindowService>,
}
