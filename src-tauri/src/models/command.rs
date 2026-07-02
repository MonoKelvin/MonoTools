use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tauri::{AppHandle, WebviewWindow};
use anyhow::Result;

/// 命令定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub namespace: String,
    pub action: String,
    pub args: HashMap<String, Value>,
    pub flags: Vec<String>,
}

/// 命令响应
#[derive(Debug, Serialize)]
pub struct CommandResponse {
    pub success: bool,
    pub data: Value,
    pub error: Option<String>,
    pub elapsed_ms: u64,
}

/// 命令上下文
#[derive(Debug, Clone)]
pub struct CommandContext {
    pub app_handle: AppHandle,
    pub window: Option<WebviewWindow>,
    pub plugin_manager: std::sync::Arc<tokio::sync::RwLock<crate::plugins::manager::PluginManager>>,
    pub config: std::sync::Arc<tokio::sync::RwLock<crate::config::store::ConfigStore>>,
    pub caller: CallerType,
}

/// 调用者类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallerType {
    Cli,
    Ipc,
    Plugin,
}

/// 命令处理器 trait
#[async_trait::async_trait]
pub trait CommandHandler: Send + Sync {
    async fn execute(&self, cmd: &Command, ctx: &CommandContext) -> Result<Value>;
    fn validate(&self, cmd: &Command) -> Result<()>;
}
