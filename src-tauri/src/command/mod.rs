pub mod command_engine;
pub mod command_registry;
pub mod command_trait;

pub mod cmd_search;
pub mod cmd_launch;
pub mod cmd_open;
pub mod cmd_command;
pub mod cmd_config;
pub mod cmd_help;
pub mod cmd_version;
pub mod cmd_index;
pub mod cmd_stats;

pub use command_engine::CommandEngine;
pub use command_registry::CommandRegistry;
pub use command_trait::{Command, CommandSpec};

pub use cmd_search::SearchCommand;
pub use cmd_launch::LaunchCommand;
pub use cmd_open::OpenCommand;
pub use cmd_command::CustomCommandHandler;
pub use cmd_config::ConfigCommand;
pub use cmd_help::HelpCommand;
pub use cmd_version::VersionCommand;
pub use cmd_index::IndexCommand;
pub use cmd_stats::StatsCommand;

pub use crate::command::command_registry::dispatch;
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
    pub app_search: std::sync::Arc<crate::engines::app_search::AppSearchEngine>,
    pub file_search: std::sync::Arc<crate::engines::file_search::FileSearchEngine>,
    pub command_search: std::sync::Arc<crate::engines::command_search::CommandSearchEngine>,
    pub command_repo: std::sync::Arc<dyn crate::repositories::CommandRepo>,
    pub settings_repo: std::sync::Arc<dyn crate::repositories::SettingsRepo>,
    pub stats_repo: std::sync::Arc<crate::repositories::StatsRepo>,
}

impl CommandContext {
    pub async fn new_headless() -> crate::error::Result<Self> {
        let settings_repo = std::sync::Arc::new(crate::repositories::InMemorySettingsRepo::new(crate::models::Settings::default()));
        let command_repo: std::sync::Arc<dyn crate::repositories::CommandRepo> =
            std::sync::Arc::new(crate::repositories::InMemoryCommandRepo::new());
        let stats_repo = std::sync::Arc::new(crate::repositories::StatsRepo::new());

        let app_search = std::sync::Arc::new(crate::engines::app_search::AppSearchEngine::new(settings_repo.clone()).await?);
        let _ = app_search.refresh_index().await;

        let command_search = std::sync::Arc::new(crate::engines::command_search::CommandSearchEngine::new(command_repo.clone()));

        let file_roots = settings_repo.get().file_search_roots.clone();
        let file_search = std::sync::Arc::new(crate::engines::file_search::FileSearchEngine::new(file_roots)?);

        Ok(Self {
            app_search,
            file_search,
            command_search,
            command_repo,
            settings_repo,
            stats_repo,
        })
    }

    pub fn from_app_state(state: &std::sync::Arc<crate::services::app_state::AppState>) -> Self {
        Self {
            app_search: state.app_search.clone(),
            file_search: state.file_search.clone(),
            command_search: state.command_search.clone(),
            command_repo: state.command_repo.clone(),
            settings_repo: state.settings_repo.clone(),
            stats_repo: state.stats_repo.clone(),
        }
    }
}
