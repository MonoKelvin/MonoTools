//! config 命令 - 显示 / 设置偏好
use crate::command::command_trait::{Command, CommandSpec};
use crate::command::{CommandContext, CommandOutput};

pub struct ConfigCommand;

#[async_trait::async_trait]
impl Command for ConfigCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("config", "查看或修改设置")
            .with_aliases(&["cfg", "setting"])
            .with_usage("config [key] [value]")
    }

    async fn execute(
        &self,
        args: &[String],
        ctx: &CommandContext,
    ) -> crate::error::Result<CommandOutput> {
        let s = ctx.settings_repo.get();
        if args.is_empty() {
            let json = serde_json::to_string_pretty(&s).unwrap_or_else(|_| "{}".into());
            return Ok(CommandOutput::ok_with_data(
                "当前设置",
                serde_json::from_str(&json).unwrap_or(serde_json::json!({})),
            ));
        }
        let key = args[0].clone();
        if args.len() == 1 {
            let json = serde_json::to_string(&s).unwrap_or_else(|_| "{}".into());
            let value = serde_json::from_str::<serde_json::Value>(&json)
                .ok()
                .and_then(|v| v.get(&key).cloned())
                .unwrap_or(serde_json::Value::Null);
            Ok(CommandOutput::ok_with_data("ok", value))
        } else {
            let val_str = args[1..].join(" ");
            let ks = key.clone();
            let vs_str = val_str.clone();
            ctx.settings_repo.update(Box::new(move |s| {
                s.apply_field(&ks, &serde_json::Value::String(vs_str.clone()));
            }))?;
            Ok(CommandOutput::ok(format!("已设置 {} = {}", key, val_str)))
        }
    }
}
