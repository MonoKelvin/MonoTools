//! launch 命令
use crate::command::command_trait::{Command, CommandSpec};
use crate::command::{CommandContext, CommandOutput};
use crate::platform::windows::shell;

pub struct LaunchCommand;

#[async_trait::async_trait]
impl Command for LaunchCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("launch", "按名称或路径启动应用")
            .with_aliases(&["run", "open-app"])
            .with_usage("launch <name-or-path>")
    }

    async fn execute(
        &self,
        args: &[String],
        ctx: &CommandContext,
    ) -> crate::error::Result<CommandOutput> {
        if args.is_empty() {
            return Ok(CommandOutput::err("用法：launch <name-or-path>"));
        }
        let name = args.join(" ");
        let results = ctx.app_search.search(&name, 1);
        if let Some(r) = results.first() {
            let path = r.subtitle.clone();
            shell::launch(&path, &[])?;
            return Ok(CommandOutput::ok(format!("已启动 {}", r.title)));
        }
        shell::launch(&name, &[])?;
        Ok(CommandOutput::ok(format!("已启动 {name}")))
    }
}
