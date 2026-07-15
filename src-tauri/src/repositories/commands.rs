//! 仓库相关命令
//!
//! 设置、统计、自定义命令管理等与仓库层相关的命令。

use crate::core::command::{Command, CommandContext, CommandOutput, CommandSpec};
use crate::models::CustomCommand;
use crate::platform::windows;
use serde_json::json;

// ==================== config 命令 ====================

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

// ==================== stats 命令 ====================

pub struct StatsCommand;

#[async_trait::async_trait]
impl Command for StatsCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("stats", "查询应用统计信息")
            .with_usage("stats [apps|commands|files]")
    }

    async fn execute(
        &self,
        args: &[String],
        ctx: &CommandContext,
    ) -> crate::core::error::Result<CommandOutput> {
        if args.is_empty() {
            return self.all_stats(ctx).await;
        }

        match args[0].as_str() {
            "apps" => self.app_stats(ctx).await,
            "commands" => self.command_stats(ctx).await,
            "files" => self.file_stats(ctx).await,
            "detail" => self.detail_stats(ctx).await,
            _ => Ok(CommandOutput::err("未知子命令：apps|commands|files|detail")),
        }
    }
}

impl StatsCommand {
    async fn detail_stats(&self, ctx: &CommandContext) -> crate::core::error::Result<CommandOutput> {
        let apps_total = ctx.app_search.total();
        let files_total = ctx.file_search.total();
        let cmds = ctx.command_repo.list();
        let enabled_cmds = cmds.iter().filter(|c| c.enabled).count();
        let settings = ctx.settings_repo.get();

        let stats = serde_json::json!({
            "apps": apps_total,
            "files": files_total,
            "commands": {
                "total": cmds.len(),
                "enabled": enabled_cmds,
                "disabled": cmds.len() - enabled_cmds,
            },
            "file_search_roots": settings.file_search_roots,
            "file_search_drives": settings.file_search_drives,
        });
        Ok(CommandOutput::ok_with_data("详细统计", stats))
    }

    async fn all_stats(&self, ctx: &CommandContext) -> crate::core::error::Result<CommandOutput> {
        let stats = serde_json::json!({
            "apps": ctx.app_search.total(),
            "files": ctx.file_search.total(),
            "commands": ctx.command_repo.list().len(),
        });
        Ok(CommandOutput::ok_with_data("系统统计", stats))
    }

    async fn app_stats(&self, ctx: &CommandContext) -> crate::core::error::Result<CommandOutput> {
        let total = ctx.app_search.total();
        Ok(CommandOutput::ok(format!("已索引 {} 个应用", total)))
    }

    async fn command_stats(&self, ctx: &CommandContext) -> crate::core::error::Result<CommandOutput> {
        let cmds = ctx.command_repo.list();
        let enabled = cmds.iter().filter(|c| c.enabled).count();

        let stats = serde_json::json!({
            "total": cmds.len(),
            "enabled": enabled,
            "disabled": cmds.len() - enabled,
        });
        Ok(CommandOutput::ok_with_data("命令统计", stats))
    }

    async fn file_stats(&self, ctx: &CommandContext) -> crate::core::error::Result<CommandOutput> {
        let total = ctx.file_search.total();
        Ok(CommandOutput::ok(format!("已索引 {} 个文件", total)))
    }
}

// ==================== command 命令（自定义命令管理） ====================

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
        if args.is_empty() {
            return Ok(CommandOutput::err(
                "用法：command <list|run|add|remove>",
            ));
        }

        let sub = args[0].as_str();
        match sub {
            "list" => {
                let cmds = ctx.command_repo.list();
                let data: Vec<_> = cmds
                    .into_iter()
                    .map(|c| {
                        json!({
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
                let cmd = ctx.command_repo.get(&id);
                let Some(cmd) = cmd else {
                    return Ok(CommandOutput::err(format!("Command not found: {id}")));
                };
                if cmd.run_as_admin {
                    windows::shell::launch_as_admin(&cmd.command, &cmd.args)?;
                } else {
                    windows::shell::launch(&cmd.command, &cmd.args)?;
                }
                ctx.command_repo
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
                ctx.command_repo.add(cmd)?;
                Ok(CommandOutput::ok(format!("已添加：{id}")))
            }
            "remove" | "rm" => {
                let id = args.get(1).cloned().unwrap_or_default();
                ctx.command_repo.remove(&id)?;
                Ok(CommandOutput::ok(format!("已删除 {id}")))
            }
            _ => Ok(CommandOutput::err(format!("未知子命令：{sub}"))),
        }
    }
}

/// 注册所有仓库相关命令
pub fn register_commands(reg: &mut crate::core::command::CommandRegistry) {
    reg.register(ConfigCommand);
    reg.register(StatsCommand);
    reg.register(CustomCommandHandler);
}
