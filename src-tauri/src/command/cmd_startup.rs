//! startup 命令 - 列表/启用/禁用/添加/删除
use crate::command::command_trait::{Command, CommandSpec};
use crate::command::{CommandContext, CommandOutput};
use crate::models::NewStartupItem;
use crate::services::startup::StartupManager;
use serde_json::json;

pub struct StartupCommand;

#[async_trait::async_trait]
impl Command for StartupCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("startup", "启动项管理")
            .with_aliases(&["su"])
            .with_usage("startup <list|enable|disable|add|remove>")
    }

    async fn execute(
        &self,
        args: &[String],
        ctx: &CommandContext,
    ) -> crate::error::Result<CommandOutput> {
        if args.is_empty() {
            return Ok(CommandOutput::err(
                "用法：startup <list|enable|disable|add|remove>",
            ));
        }

        let mgr = StartupManager::new(ctx.startup_repo.clone());
        mgr.refresh().await?;

        let sub = args[0].as_str();
        match sub {
            "list" => {
                let items = mgr.list();
                let data: Vec<serde_json::Value> = items
                    .iter()
                    .map(|i| {
                        json!({
                            "id": i.id,
                            "name": i.name,
                            "command": i.command,
                            "enabled": i.enabled,
                            "source": i.source,
                            "delay": i.delay_seconds,
                        })
                    })
                    .collect();
                Ok(CommandOutput::ok_with_data(
                    format!("共 {} 项", items.len()),
                    serde_json::Value::Array(data),
                ))
            }
            "enable" | "disable" => {
                let id = args.get(1).cloned().unwrap_or_default();
                let enabled = sub == "enable";
                mgr.toggle(&id, enabled).await?;
                Ok(CommandOutput::ok(format!("已{}启动项 {}", sub, id)))
            }
            "remove" | "rm" => {
                let id = args.get(1).cloned().unwrap_or_default();
                mgr.remove(&id).await?;
                Ok(CommandOutput::ok(format!("已删除 {id}")))
            }
            "add" => {
                let name = args.get(1).cloned().unwrap_or_default();
                let cmd = args.get(2).cloned().unwrap_or_default();
                if name.is_empty() || cmd.is_empty() {
                    return Ok(CommandOutput::err("用法：startup add <name> <command>"));
                }
                let item = NewStartupItem {
                    name,
                    command: cmd,
                    args: vec![],
                    working_dir: None,
                    delay_seconds: 0,
                    run_as_admin: false,
                };
                let id = mgr.add(item).await?;
                Ok(CommandOutput::ok(format!("已添加：{id}")))
            }
            _ => Ok(CommandOutput::err(format!("未知子命令：{sub}"))),
        }
    }
}
