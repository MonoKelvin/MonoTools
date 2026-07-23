//! 命令注册表：每条命令以 `id` + 默认 `aliases` 注册；通过同名 id 直接派发或通过字符串输入解析派发。
use crate::core::command::command_trait::{Command, CommandSpec};
use crate::core::command::{CommandContext, CommandOutput};
use std::collections::HashMap;

/// 命令注册表
///
/// 纯机制，不包含任何业务命令注册逻辑。
/// 业务模块通过自己的 `register_commands()` 函数注册命令，
/// 组装点在 app 层或 CLI 入口。
pub struct CommandRegistry {
    cmds: HashMap<String, Box<dyn Command>>,
    aliases: HashMap<String, String>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            cmds: HashMap::new(),
            aliases: HashMap::new(),
        }
    }

    /// 注册一条命令。`spec.aliases` 自动作为他的别名索引（同样能 dispatch 到该命令）。
    pub fn register_boxed(&mut self, cmd: Box<dyn Command>) {
        let spec = cmd.spec();
        let name = spec.name.to_string();

        let mut to_remove: Vec<String> = Vec::new();
        for (alias, target) in self.aliases.iter() {
            if target == &name {
                to_remove.push(alias.clone());
            }
        }
        for k in to_remove {
            self.aliases.remove(&k);
        }

        self.cmds.insert(name.clone(), cmd);
        for alias in spec.aliases.iter() {
            if !self.cmds.contains_key(*alias) {
                self.aliases.insert(alias.to_string(), name.clone());
            }
        }
    }

    /// 注册一条命令（impl `Command` 的具体类型）。等价于 `register_boxed(Box::new(cmd))`。
    pub fn register<C: Command + 'static>(&mut self, cmd: C) {
        self.register_boxed(Box::new(cmd));
    }

    pub fn lookup(&self, name: &str) -> Option<&dyn Command> {
        if let Some(cmd) = self.cmds.get(name).map(|c| c.as_ref()) {
            return Some(cmd);
        }
        if let Some(target) = self.aliases.get(name) {
            if let Some(cmd) = self.cmds.get(target).map(|c| c.as_ref()) {
                return Some(cmd);
            }
        }
        None
    }

    pub fn names(&self) -> Vec<String> {
        let mut all: Vec<String> = self.cmds.keys().cloned().collect();
        for a in self.aliases.keys() {
            if !all.iter().any(|n| n == a) {
                all.push(a.clone());
            }
        }
        all.sort();
        all
    }

    /// 返回所有主命令名（不含别名）。
    pub fn main_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.cmds.keys().cloned().collect();
        names.sort();
        names
    }

    /// 返回所有主命令的 spec（按名称排序）。
    pub fn all_specs(&self) -> Vec<CommandSpec> {
        let mut specs: Vec<CommandSpec> = self.cmds.values().map(|cmd| cmd.spec()).collect();
        specs.sort_by(|a, b| a.name.cmp(b.name));
        specs
    }

    /// 动态生成帮助文本。
    pub fn help_text(&self) -> String {
        let specs = self.all_specs();
        let mut lines = vec![
            "MonoTools CLI - 命令帮助".to_string(),
            String::new(),
            "用法：".to_string(),
            "  monotools-cli <command> [args]".to_string(),
            String::new(),
            "可用命令：".to_string(),
        ];

        for spec in specs {
            let alias_str = if spec.aliases.is_empty() {
                String::new()
            } else {
                format!(" ({})", spec.aliases.join(", "))
            };
            let usage = if spec.usage != spec.name {
                spec.usage.to_string()
            } else {
                spec.name.to_string()
            };
            lines.push(format!("  {:<26} {}{}", usage, spec.description, alias_str));
        }

        lines.push(String::new());
        lines.push("示例：".to_string());
        lines.push("  monotools-cli search chrome".to_string());
        lines.push("  monotools-cli config hotkey \"Ctrl+Space\"".to_string());
        lines.push("  monotools-cli index build".to_string());
        lines.push(String::new());

        lines.join("\n")
    }

    /// 解析 + 执行。`input` 是完整命令行字符串（含要执行的子命令）。
    pub async fn dispatch_str(
        &self,
        input: &str,
        ctx: &CommandContext,
    ) -> crate::core::error::Result<CommandOutput> {
        let parts: Vec<String> = shell_words::split(input)
            .map_err(|e| crate::core::error::AppError::InvalidInput(format!("参数解析失败: {e}")))?;
        if parts.is_empty() {
            return Ok(CommandOutput::err("空命令"));
        }
        self.dispatch_id(&parts[0], &parts[1..], ctx).await
    }

    /// 直接按 id 派发（绕过 subcommand 解析）。
    pub async fn dispatch_id(
        &self,
        id: &str,
        args: &[String],
        ctx: &CommandContext,
    ) -> crate::core::error::Result<CommandOutput> {
        if let Some(cmd) = self.cmds.get(id).map(|c| c.as_ref()) {
            return cmd.execute(args, ctx).await;
        }
        if let Some(target) = self.aliases.get(id) {
            if let Some(cmd) = self.cmds.get(target).map(|c| c.as_ref()) {
                return cmd.execute(args, ctx).await;
            }
        }
        Ok(CommandOutput::err(format!(
            "未知命令 '{id}'（输入 help 查看可用列表）"
        )))
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 构建只包含 core 层命令的 registry（纯机制，无业务）。
///
/// 包含：HelpCommand、VersionCommand、CustomCommandHandler、ConfigCommand
///
/// 业务模块的命令由上层（app 层或 CLI 入口）组装。
pub fn build_core_registry() -> CommandRegistry {
    let mut reg = CommandRegistry::new();

    reg.register(crate::core::command::cmd_help::HelpCommand);
    reg.register(crate::core::command::cmd_version::VersionCommand);
    reg.register(crate::core::command::cmd_custom::CustomCommandHandler);

    crate::core::settings::cmd_settings::register_commands(&mut reg);

    reg
}

/// 给 CLI 使用的便利入口：用指定 registry 解析并派发命令。
pub async fn dispatch(
    registry: &CommandRegistry,
    input: &str,
    ctx: &CommandContext,
) -> crate::core::error::Result<CommandOutput> {
    registry.dispatch_str(input, ctx).await
}

/// 供前端 Tauri IPC（`dispatch_command`）调用：按 id 精确派发。
pub async fn registry_dispatch(
    registry: &CommandRegistry,
    id: &str,
    args: &[String],
    ctx: &CommandContext,
) -> crate::core::error::Result<CommandOutput> {
    registry.dispatch_id(id, args, ctx).await
}
