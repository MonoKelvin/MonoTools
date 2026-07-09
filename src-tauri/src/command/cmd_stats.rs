//! stats 命令 - 统计信息查询
use crate::command::command_trait::{Command, CommandSpec};
use crate::command::{CommandContext, CommandOutput};

pub struct StatsCommand;

#[async_trait::async_trait]
impl Command for StatsCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("stats", "查询应用统计信息")
            .with_usage("stats [apps|commands|files]")
    }

    async fn execute(
        &self,
        args: &[String],
        ctx: &CommandContext,
    ) -> crate::error::Result<CommandOutput> {
        if args.is_empty() {
            return self.all_stats(ctx).await;
        }

        match args[0].as_str() {
            "apps" => self.app_stats(ctx).await,
            "commands" => self.command_stats(ctx).await,
            "files" => self.file_stats(ctx).await,
            _ => Ok(CommandOutput::err("未知子命令：apps|commands|files")),
        }
    }
}

impl StatsCommand {
    async fn all_stats(&self, ctx: &CommandContext) -> crate::error::Result<CommandOutput> {
        let stats = serde_json::json!({
            "apps": ctx.app_search.total(),
            "files": ctx.file_search.total(),
            "commands": ctx.command_repo.list().len(),
        });
        Ok(CommandOutput::ok_with_data("系统统计", stats))
    }

    async fn app_stats(&self, ctx: &CommandContext) -> crate::error::Result<CommandOutput> {
        let total = ctx.app_search.total();
        Ok(CommandOutput::ok(format!("已索引 {} 个应用", total)))
    }

    async fn command_stats(&self, ctx: &CommandContext) -> crate::error::Result<CommandOutput> {
        let cmds = ctx.command_repo.list();
        let enabled = cmds.iter().filter(|c| c.enabled).count();
        
        let stats = serde_json::json!({
            "total": cmds.len(),
            "enabled": enabled,
            "disabled": cmds.len() - enabled,
        });
        Ok(CommandOutput::ok_with_data("命令统计", stats))
    }

    async fn file_stats(&self, ctx: &CommandContext) -> crate::error::Result<CommandOutput> {
        let total = ctx.file_search.total();
        Ok(CommandOutput::ok(format!("已索引 {} 个文件", total)))
    }
}
