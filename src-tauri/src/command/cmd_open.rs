//! open 命令 - 在文件管理器中打开
use crate::command::command_trait::{Command, CommandSpec};
use crate::command::{CommandContext, CommandOutput};
use crate::platform::windows::shell;

pub struct OpenCommand;

#[async_trait::async_trait]
impl Command for OpenCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("open", "在文件管理器中打开路径")
            .with_usage("open <path>")
    }

    async fn execute(
        &self,
        args: &[String],
        ctx: &CommandContext,
    ) -> crate::error::Result<CommandOutput> {
        if args.is_empty() {
            return Ok(CommandOutput::err("用法：open <path>"));
        }
        let path = args.join(" ");
        let path_buf = std::path::PathBuf::from(&path);
        shell::open_path(&path_buf)?;
        Ok(CommandOutput::ok(format!("已打开 {path}")))
    }
}
