//! GUI 应用状态 - 由 tauri::Builder.manage 注入
//!
//! 仅在 GUI 模式下使用，CLI 模式使用 CommandContext::new_headless()。
//!
//! # 模块组织原则
//! - AppState 只存放核心的、被多个模块共享的状态
//! - 独立模块自己管理状态，通过 app.manage() 注册
//! - 独立模块的 IPC 命令通过 State<'_, Arc<ModuleService>> 访问自己的状态

use crate::core::command::{CommandContext, CommandRepo};
use crate::core::settings::SettingsRepo;
use crate::search_engine::app_search::AppSearchEngine;
use crate::search_engine::command_search::CommandSearchEngine;
use crate::search_engine::file_search::FileSearchEngine;
use crate::search_engine::{PinRepo, SearchEngine, StatsRepo};
use crate::services::hotkey::HotkeyService;
use crate::services::window::WindowService;
use std::sync::atomic::AtomicBool;
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

    /// 前端 UI 渲染完成标志. 为 true 时表示 frontend_ready 已被调用, 窗口可以安全显示.
    pub frontend_initialized: Arc<AtomicBool>,
}

impl AppState {
    /// 从 AppState 构建命令执行上下文
    ///
    /// 将所有业务依赖注入到 CommandContext 中，
    /// 命令实现通过 `ctx.get::<Arc<T>>()` 获取所需依赖。
    pub fn build_command_context(&self) -> CommandContext {
        let mut ctx = CommandContext::new();
        ctx.insert(self.settings_repo.clone());
        ctx.insert(self.command_repo.clone());
        ctx.insert(self.stats_repo.clone());
        ctx.insert(self.pin_repo.clone());
        ctx.insert(self.app_search.clone());
        ctx.insert(self.command_search.clone());
        ctx.insert(self.file_search.clone());
        ctx.insert(self.search_engine.clone());
        ctx.insert(self.hotkey.clone());
        ctx.insert(self.window.clone());

        let command_specs = crate::app::modules::build_command_registry().all_specs();
        ctx.insert(command_specs);
        ctx
    }
}
