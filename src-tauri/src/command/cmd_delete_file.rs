//! delete-file 命令 - 删除文件到回收站
use crate::command::command_trait::{Command, CommandSpec};
use crate::command::{CommandContext, CommandOutput};
use crate::platform::windows::shell;

pub struct DeleteFileCommand;

#[async_trait::async_trait]
impl Command for DeleteFileCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("delete-file", "删除文件到回收站")
            .with_usage("delete-file <path>")
    }

    async fn execute(
        &self,
        args: &[String],
        _ctx: &CommandContext,
    ) -> crate::error::Result<CommandOutput> {
        if args.is_empty() {
            return Ok(CommandOutput::err("用法：delete-file <path>"));
        }
        let path = args.join(" ");
        let path_buf = std::path::PathBuf::from(&path);
        shell::delete_to_recycle_bin(&path_buf)?;
        Ok(CommandOutput::ok(format!("已删除 {path}")))
    }
}
