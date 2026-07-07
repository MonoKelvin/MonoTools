//! version 命令
use crate::command::command_trait::{Command, CommandSpec};
use crate::command::{CommandContext, CommandOutput};

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
    ) -> crate::error::Result<CommandOutput> {
        Ok(CommandOutput::ok_with_data(
            "MonoTools v0.1.0 · OS: ".to_string() + std::env::consts::OS,
            serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
                "name": env!("CARGO_PKG_NAME"),
                "os": std::env::consts::OS,
            }),
        ))
    }
}
