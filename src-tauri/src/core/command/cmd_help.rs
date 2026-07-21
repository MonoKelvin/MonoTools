//! help 命令
use crate::core::command::command_trait::{Command, CommandSpec};
use crate::core::command::{CommandContext, CommandOutput};

pub struct HelpCommand;

#[async_trait::async_trait]
impl Command for HelpCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("help", "显示帮助").with_aliases(&["-h", "--help"])
    }

    async fn execute(
        &self,
        _args: &[String],
        ctx: &CommandContext,
    ) -> crate::core::error::Result<CommandOutput> {
        let mut lines = vec![
            "MonoTools CLI - 命令帮助".to_string(),
            String::new(),
            "用法：".to_string(),
            "  monotools-cli <command> [args]".to_string(),
            String::new(),
            "可用命令：".to_string(),
        ];

        let command_specs = ctx.get::<Vec<CommandSpec>>().cloned().unwrap_or_default();
        let mut specs = command_specs;
        specs.sort_by(|a, b| a.name.cmp(b.name));

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
            lines.push(format!(
                "  {:<26} {}{}",
                usage, spec.description, alias_str
            ));
        }

        lines.push(String::new());
        lines.push("示例：".to_string());
        lines.push("  monotools-cli search chrome".to_string());
        lines.push("  monotools-cli config hotkey \"Ctrl+Space\"".to_string());
        lines.push("  monotools-cli index build".to_string());
        lines.push(String::new());

        Ok(CommandOutput::ok(lines.join("\n")))
    }
}
