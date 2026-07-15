//! 命令注册表：每条命令以 `id` + 默认 `aliases` 注册；通过同名 id 直接派发或通过字符串输入解析派发。
use crate::core::command::command_trait::{Command, CommandSpec};
use crate::core::command::{CommandContext, CommandOutput};
use std::collections::HashMap;

/// 内置的 Command registry —— 通过 `build_default_registry()` 初始化。
///
/// 注意：[`crate::core::command::command_registry::dispatch`] 仍保持为顶层便利入口，
/// CLI 工具继续用它；前端 IPC 走 [`crate::core::command::command_registry::registry_dispatch`]。
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
        self.cmds.get(name).map(|c| c.as_ref())
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

/// 注册 MonoTools 内置命令并返回初始化的 registry。
///
/// 各业务模块通过自己的 `register_commands()` 函数注册命令，
/// 这里是统一的组装点。
pub fn build_default_registry() -> CommandRegistry {
    let mut reg = CommandRegistry::new();

    // 系统命令（命令模块内置）
    reg.register(crate::core::command::cmd_help::HelpCommand);
    reg.register(crate::core::command::cmd_version::VersionCommand);

    // Windows 平台命令
    crate::platform::windows::commands::register_commands(&mut reg);

    // 搜索 & 索引命令
    crate::search_engine::commands::register_commands(&mut reg);

    // 仓库相关命令（设置、统计、自定义命令管理）
    crate::repositories::commands::register_commands(&mut reg);

    reg
}

/// 给 CLI 使用的便利入口：每次调用都构造一个新的默认 registry 后执行。
pub async fn dispatch(input: &str, ctx: &CommandContext) -> crate::core::error::Result<CommandOutput> {
    build_default_registry().dispatch_str(input, ctx).await
}

/// 供前端 Tauri IPC（`dispatch_command`）调用：按 id 精确派发。
pub async fn registry_dispatch(
    id: &str,
    args: &[String],
    ctx: &CommandContext,
) -> crate::core::error::Result<CommandOutput> {
    build_default_registry().dispatch_id(id, args, ctx).await
}
