//! 命令注册与分派
use crate::command::command_trait::Command;
use crate::command::{CommandContext, CommandOutput};
use std::collections::HashMap;

pub struct CommandRegistry {
    pub cmds: HashMap<String, Box<dyn Command>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            cmds: HashMap::new(),
        }
    }

    pub fn register_boxed(&mut self, cmd: Box<dyn Command>) {
        let spec = cmd.spec();
        self.cmds.insert(spec.name.to_string(), cmd);
    }

    pub fn lookup(&self, name: &str) -> Option<&dyn Command> {
        self.cmds.get(name).map(|c| c.as_ref())
    }

    /// 解析 + 执行。`input` 是一个完整命令行字符串。
    pub async fn dispatch_str(&self, input: &str, ctx: &CommandContext) -> crate::error::Result<CommandOutput> {
        let parts: Vec<String> = shell_words::split(input)
            .map_err(|e| crate::error::AppError::InvalidInput(format!("参数解析失败: {e}")))?;
        if parts.is_empty() {
            return Ok(CommandOutput::err("空命令"));
        }
        let name = &parts[0];
        let args = &parts[1..];

        if let Some(cmd) = self.lookup(name) {
            cmd.execute(args, ctx).await
        } else {
            Ok(CommandOutput::err(format!(
                "未知命令 '{name}'（输入 help 查看可用列表）"
            )))
        }
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 给 CLI 使用的便利入口
pub async fn dispatch(input: &str, ctx: &CommandContext) -> crate::error::Result<CommandOutput> {
    let mut reg = CommandRegistry::new();
    reg.register_boxed(Box::new(crate::command::cmd_search::SearchCommand));
    reg.register_boxed(Box::new(crate::command::cmd_launch::LaunchCommand));
    reg.register_boxed(Box::new(crate::command::cmd_open::OpenCommand));
    reg.register_boxed(Box::new(crate::command::cmd_command::CustomCommandHandler));
    reg.register_boxed(Box::new(crate::command::cmd_config::ConfigCommand));
    reg.register_boxed(Box::new(crate::command::cmd_help::HelpCommand));
    reg.register_boxed(Box::new(crate::command::cmd_version::VersionCommand));
    reg.register_boxed(Box::new(crate::command::cmd_index::IndexCommand));
    reg.register_boxed(Box::new(crate::command::cmd_stats::StatsCommand));

    reg.dispatch_str(input, ctx).await
}
