//! 命令系统 - 同时服务于 CLI 和 Tauri IPC

// Re-exports of trait, registry, engine, and all command implementations
#[path = "command/command_trait.rs"]
pub mod command_trait;
#[path = "command/command_registry.rs"]
pub mod command_registry;
#[path = "command/command_engine.rs"]
pub mod command_engine;
#[path = "command/cmd_search.rs"]
pub mod cmd_search;
#[path = "command/cmd_launch.rs"]
pub mod cmd_launch;
#[path = "command/cmd_open.rs"]
pub mod cmd_open;
#[path = "command/cmd_startup.rs"]
pub mod cmd_startup;
#[path = "command/cmd_command.rs"]
pub mod cmd_command;
#[path = "command/cmd_config.rs"]
pub mod cmd_config;
#[path = "command/cmd_help.rs"]
pub mod cmd_help;
#[path = "command/cmd_version.rs"]
pub mod cmd_version;

pub use command_trait::*;
pub use command_registry::*;
pub use command_engine::*;

use crate::engines::app_search::AppSearchEngine;
use crate::engines::command_search::CommandSearchEngine;
use crate::engines::file_search::FileSearchService;
use crate::engines::startup_search::StartupSearchService;
use crate::models::{Settings};
use crate::repositories::*;
use std::sync::Arc;

/// Command 系统共享上下文
pub struct CommandContext {
    pub settings_repo: Arc<dyn SettingsRepo>,
    pub startup_repo: Arc<dyn StartupRepo>,
    pub command_repo: Arc<dyn CommandRepo>,
    pub stats_repo: Arc<StatsRepo>,

    pub app_search: Arc<AppSearchEngine>,
    pub file_search: Arc<FileSearchService>,
    pub command_search: Arc<CommandSearchEngine>,
    pub startup_search: Arc<StartupSearchService>,
}

impl CommandContext {
    /// CLI 模式（无 GUI）下创建一个最小上下文
    pub async fn new_headless() -> anyhow::Result<Self> {
        let settings_repo: Arc<dyn SettingsRepo> =
            Arc::new(InMemorySettingsRepo::new(Settings::default()));
        let startup_repo: Arc<dyn StartupRepo> = Arc::new(InMemoryStartupRepo::new());
        let command_repo: Arc<dyn CommandRepo> = Arc::new(InMemoryCommandRepo::new());
        let stats_repo = Arc::new(StatsRepo::new());

        let app_search = Arc::new(AppSearchEngine::new(settings_repo.clone()).await?);
        let _ = app_search.refresh_index().await;

        let startup_search = Arc::new(StartupSearchService::new(startup_repo.clone()).await?);
        let _ = startup_search.refresh().await;

        let command_search = Arc::new(CommandSearchEngine::new(
            command_repo.clone(),
            startup_repo.clone(),
        ));

        let file_search = Arc::new(FileSearchService::new(vec![]));
        let _ = file_search.build_index().await;

        Ok(Self {
            settings_repo,
            startup_repo,
            command_repo,
            stats_repo,
            app_search,
            file_search,
            command_search,
            startup_search,
        })
    }
}

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
