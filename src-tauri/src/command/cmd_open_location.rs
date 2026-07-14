//! open-location 命令 - 打开文件所在目录
use crate::command::command_trait::{Command, CommandSpec};
use crate::command::{CommandContext, CommandOutput};
use crate::platform::windows::shell;

pub struct OpenLocationCommand;

#[async_trait::async_trait]
impl Command for OpenLocationCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("open-location", "打开文件所在目录")
            .with_usage("open-location <path>")
    }

    async fn execute(
        &self,
        args: &[String],
        _ctx: &CommandContext,
    ) -> crate::error::Result<CommandOutput> {
        if args.is_empty() {
            return Ok(CommandOutput::err("用法：open-location <path>"));
        }
        let path = args.join(" ");
        let path_buf = std::path::PathBuf::from(&path);
        shell::open_containing_folder(&path_buf)?;
        Ok(CommandOutput::ok(format!("已打开 {path} 的所在目录")))
    }
}
