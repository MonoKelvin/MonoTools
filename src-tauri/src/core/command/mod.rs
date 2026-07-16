//! 命令系统 —— 通用机制 + 自定义命令模型 + 系统内置命令
//!
//! 核心框架（纯机制，无业务逻辑）：
//! - `Command` trait / `CommandSpec`: 命令接口
//! - `CommandRegistry`: 命令注册与派发
//! - `CommandOutput`: 统一输出格式
//! - `CommandContext`: 命令执行上下文（业务依赖容器）
//!
//! 自定义命令模型与仓库：
//! - `CustomCommand`: 自定义命令数据结构
//! - `CommandRepo` trait + InMemoryCommandRepo: 自定义命令存储接口
//!
//! 系统内置命令（本模块保留，因为是通用系统功能）：
//! - `HelpCommand`: 帮助命令
//! - `VersionCommand`: 版本命令
//!
//! 业务命令已迁移到各自模块：
//! - Windows 平台命令 → `crate::platform::windows::commands`
//! - 搜索 & 索引命令 → `crate::search_engine::commands`
//! - 仓库相关命令（设置、统计、自定义命令管理）→ `crate::repositories::commands`

pub mod command_custom;
pub mod command_registry;
pub mod command_repo;
pub mod command_trait;

pub mod cmd_help;
pub mod cmd_version;

pub use command_custom::CustomCommand;
pub use command_registry::{build_default_registry, dispatch, registry_dispatch, CommandRegistry};
pub use command_repo::{CommandRepo, InMemoryCommandRepo};
pub use command_trait::{Command, CommandSpec};

pub use cmd_help::HelpCommand;
pub use cmd_version::VersionCommand;

pub use crate::repositories::settings_repo::SettingsRepo;

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommandOutput {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl CommandOutput {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
        }
    }

    pub fn ok_with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
        }
    }
}

#[derive(Clone)]
pub struct CommandContext {
    pub app_search: std::sync::Arc<crate::search_engine::app_search::AppSearchEngine>,
    pub file_search: std::sync::Arc<crate::search_engine::file_search::FileSearchEngine>,
    pub command_search: std::sync::Arc<crate::search_engine::command_search::CommandSearchEngine>,
    pub command_repo: std::sync::Arc<dyn crate::repositories::CommandRepo>,
    pub settings_repo: std::sync::Arc<dyn crate::repositories::SettingsRepo>,
    pub stats_repo: std::sync::Arc<crate::repositories::StatsRepo>,
    pub command_specs: Vec<CommandSpec>,
}

impl CommandContext {
    pub async fn new_headless() -> crate::core::error::Result<Self> {
        let settings_repo = std::sync::Arc::new(crate::repositories::InMemorySettingsRepo::new(
            crate::models::Settings::default(),
        ));
        let command_repo: std::sync::Arc<dyn crate::repositories::CommandRepo> =
            std::sync::Arc::new(crate::repositories::InMemoryCommandRepo::new());
        let stats_repo = std::sync::Arc::new(crate::repositories::StatsRepo::new());

        let app_search = std::sync::Arc::new(
            crate::search_engine::app_search::AppSearchEngine::new(settings_repo.clone()).await?,
        );
        let _ = app_search.refresh_index().await;

        let command_search = std::sync::Arc::new(
            crate::search_engine::command_search::CommandSearchEngine::new(command_repo.clone()),
        );

        let file_roots = settings_repo.get().file_search_roots.clone();
        let file_search = std::sync::Arc::new(
            crate::search_engine::file_search::FileSearchEngine::new(file_roots)?,
        );

        let command_specs =
            crate::core::command::command_registry::build_default_registry().all_specs();

        Ok(Self {
            app_search,
            file_search,
            command_search,
            command_repo,
            settings_repo,
            stats_repo,
            command_specs,
        })
    }

    /// 从全局 AppState 抽取命令执行所需的依赖：保留原 `from_app_state` 接口以兼容旧模块。
    pub fn from_app_state(state: &std::sync::Arc<crate::app::state::AppState>) -> Self {
        let command_specs =
            crate::core::command::command_registry::build_default_registry().all_specs();
        Self {
            app_search: state.app_search.clone(),
            file_search: state.file_search.clone(),
            command_search: state.command_search.clone(),
            command_repo: state.command_repo.clone(),
            settings_repo: state.settings_repo.clone(),
            stats_repo: state.stats_repo.clone(),
            command_specs,
        }
    }
}
