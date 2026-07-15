//! version 命令
use crate::core::command::command_trait::{Command, CommandSpec};
use crate::core::command::{CommandContext, CommandOutput};

pub struct VersionCommand;

#[async_trait::async_trait]
impl Command for VersionCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("version", "版本信息").with_aliases(&["-v", "--version"])
    }

    async fn execute(
        &self,
        _args: &[String],
        _ctx: &CommandContext,
    ) -> crate::core::error::Result<CommandOutput> {
        let version = env!("CARGO_PKG_VERSION");
        Ok(CommandOutput::ok_with_data(
            format!("MonoTools v{version} · OS: {}", std::env::consts::OS),
            serde_json::json!({
                "version": version,
                "name": env!("CARGO_PKG_NAME"),
                "os": std::env::consts::OS,
            }),
        ))
    }
}
