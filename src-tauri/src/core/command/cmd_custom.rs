//! 自定义命令管理 CLI 命令
//!
//! `command` 命令 —— 自定义命令的增删改查。

use crate::core::command::command_custom::CustomCommand;
use crate::core::command::{Command, CommandContext, CommandOutput, CommandRepo, CommandSpec};
use std::sync::Arc;

pub struct CustomCommandHandler;

#[async_trait::async_trait]
impl Command for CustomCommandHandler {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("command", "自定义命令管理")
            .with_aliases(&["cmd", "c"])
            .with_usage("command <list|run|add|remove>")
    }

    async fn execute(
        &self,
        args: &[String],
        ctx: &CommandContext,
    ) -> crate::core::error::Result<CommandOutput> {
        let command_repo = ctx
            .get::<Arc<dyn CommandRepo>>()
            .ok_or("CommandRepo not found in context")?;

        if args.is_empty() {
            return Ok(CommandOutput::err("用法：command <list|run|add|remove>"));
        }

        let sub = args[0].as_str();
        match sub {
            "list" => {
                let cmds = command_repo.list();
                let data: Vec<_> = cmds
                    .into_iter()
                    .map(|c| {
                        serde_json::json!({
                            "id": c.id,
                            "name": c.name,
                            "keyword": c.keyword,
                            "command": c.command,
                            "args": c.args,
                            "enabled": c.enabled,
                        })
                    })
                    .collect();
                Ok(CommandOutput::ok_with_data(
                    "自定义命令列表",
                    serde_json::Value::Array(data),
                ))
            }
            "run" => {
                let id = args.get(1).cloned().unwrap_or_default();
                let cmd = command_repo.get(&id);
                let Some(cmd) = cmd else {
                    return Ok(CommandOutput::err(format!("Command not found: {id}")));
                };
                if cmd.run_as_admin {
                    crate::platform::windows::shell::launch_as_admin(&cmd.command, &cmd.args)?;
                } else {
                    crate::platform::windows::shell::launch(&cmd.command, &cmd.args)?;
                }
                command_repo
                    .record_used(&id, chrono::Utc::now().timestamp())?;
                Ok(CommandOutput::ok(format!("已运行 {}", cmd.name)))
            }
            "add" => {
                let name = args.get(1).cloned().unwrap_or_default();
                let kw = args.get(2).cloned().unwrap_or_default();
                let command = args.get(3).cloned().unwrap_or_default();
                if name.is_empty() || command.is_empty() {
                    return Ok(CommandOutput::err(
                        "用法：command add <name> <keyword> <command>",
                    ));
                }
                let name_for_field = name.clone();
                let cmd = CustomCommand {
                    id: uuid::Uuid::new_v4().to_string(),
                    name,
                    description: None,
                    keyword: if kw.is_empty() { name_for_field } else { kw },
                    command,
                    args: vec![],
                    working_dir: None,
                    icon: None,
                    category: "Custom".into(),
                    enabled: true,
                    run_as_admin: false,
                    created_at: chrono::Utc::now().timestamp(),
                    last_used_at: None,
                };
                let id = cmd.id.clone();
                command_repo.add(cmd)?;
                Ok(CommandOutput::ok(format!("已添加：{id}")))
            }
            "remove" | "rm" => {
                let id = args.get(1).cloned().unwrap_or_default();
                command_repo.remove(&id)?;
                Ok(CommandOutput::ok(format!("已删除 {id}")))
            }
            _ => Ok(CommandOutput::err(format!("未知子命令：{sub}"))),
        }
    }
}

/// 注册自定义命令管理命令
pub fn register_commands(reg: &mut crate::core::command::CommandRegistry) {
    reg.register(CustomCommandHandler);
}
