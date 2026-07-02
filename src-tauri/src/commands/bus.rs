use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{Result, Context};
use serde_json::Value;
use crate::models::command::{Command, CommandHandler, CommandContext, CommandResponse};

/// 命令总线 - 所有功能的统一入口
#[derive(Clone)]
pub struct CommandBus {
    handlers: Arc<tokio::sync::RwLock<HashMap<String, Box<dyn CommandHandler + Send + Sync>>>>,
}

impl CommandBus {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// 注册命令处理器
    pub async fn register(
        &self,
        namespace: &str,
        action: &str,
        handler: Box<dyn CommandHandler + Send + Sync>,
    ) {
        let key = format!("{}:{}", namespace, action);
        let mut handlers = self.handlers.write().await;
        handlers.insert(key, handler);
    }

    /// 执行命令
    pub async fn execute(&self, cmd: Command, ctx: CommandContext) -> Result<CommandResponse> {
        let key = format!("{}:{}", cmd.namespace, cmd.action);
        let handlers = self.handlers.read().await;

        let handler = handlers.get(&key)
            .ok_or_else(|| anyhow::anyhow!("Unknown command: {}", key))?;

        let start = std::time::Instant::now();

        // 验证参数
        handler.validate(&cmd).context("Command validation failed")?;

        // 执行
        let result = handler.execute(&cmd, &ctx).await?;
        let elapsed = start.elapsed().as_millis() as u64;

        Ok(CommandResponse {
            success: true,
            data: serde_json::to_value(result)?,
            error: None,
            elapsed_ms: elapsed,
        })
    }

    /// 列出所有已注册的命令
    pub async fn list_commands(&self) -> Vec<String> {
        let handlers = self.handlers.read().await;
        handlers.keys().cloned().collect()
    }
}

impl Default for CommandBus {
    fn default() -> Self {
        Self::new()
    }
}
