//! 设置相关 CLI 命令
//!
//! `config` 命令 —— 查看或修改全局设置。

use crate::core::command::{Command, CommandContext, CommandOutput, CommandSpec, SettingsRepo};
use std::sync::Arc;

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
    ) -> crate::core::error::Result<CommandOutput> {
        let settings_repo = ctx
            .get::<Arc<dyn SettingsRepo>>()
            .ok_or("SettingsRepo not found in context")?;

        let s = settings_repo.get();
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
            settings_repo.update(Box::new(move |s| {
                s.apply_field(&ks, &serde_json::Value::String(vs_str.clone()));
            }))?;
            Ok(CommandOutput::ok(format!("已设置 {} = {}", key, val_str)))
        }
    }
}

/// 注册设置相关命令
pub fn register_commands(reg: &mut crate::core::command::CommandRegistry) {
    reg.register(ConfigCommand);
}
