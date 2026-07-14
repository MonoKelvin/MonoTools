//! show-properties 命令 - 显示文件属性
use crate::command::command_trait::{Command, CommandSpec};
use crate::command::{CommandContext, CommandOutput};
use crate::platform::windows::shell;

pub struct ShowPropertiesCommand;

#[async_trait::async_trait]
impl Command for ShowPropertiesCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("show-properties", "显示文件属性")
            .with_usage("show-properties <path>")
    }

    async fn execute(
        &self,
        args: &[String],
        _ctx: &CommandContext,
    ) -> crate::error::Result<CommandOutput> {
        if args.is_empty() {
            return Ok(CommandOutput::err("用法：show-properties <path>"));
        }
        let path = args.join(" ");
        let path_buf = std::path::PathBuf::from(&path);
        shell::show_file_properties(&path_buf)?;
        Ok(CommandOutput::ok(format!("已显示 {path} 的属性")))
    }
}
