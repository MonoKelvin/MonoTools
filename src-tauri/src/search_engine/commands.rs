//! 搜索相关命令
//!
//! 搜索、索引构建等命令实现。

use crate::core::command::{Command, CommandContext, CommandOutput, CommandSpec};

// ==================== search 命令 ====================

pub struct SearchCommand;

#[async_trait::async_trait]
impl Command for SearchCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("search", "搜索应用/文件/命令")
            .with_aliases(&["s", "find"])
            .with_usage("search <query> [--limit N]")
    }

    async fn execute(
        &self,
        args: &[String],
        ctx: &CommandContext,
    ) -> crate::core::error::Result<CommandOutput> {
        if args.is_empty() {
            return Ok(CommandOutput::err("用法：search <query>"));
        }

        let mut limit: u32 = 20;
        let mut query_parts: Vec<String> = Vec::new();
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--limit" | "-l" => {
                    if i + 1 < args.len() {
                        limit = args[i + 1].parse().unwrap_or(10);
                        i += 2;
                        continue;
                    }
                }
                _ => query_parts.push(args[i].clone()),
            }
            i += 1;
        }

        let query = query_parts.join(" ");
        if query.is_empty() {
            return Ok(CommandOutput::err("用法：search <query>"));
        }

        let mut results: Vec<serde_json::Value> = Vec::new();
        results.extend(
            ctx.app_search
                .search(&query, limit)
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "title": r.title,
                        "subtitle": r.subtitle,
                        "category": r.category,
                        "score": r.score,
                    })
                }),
        );
        results.extend(
            ctx.file_search
                .search(&query, limit)
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "title": r.title,
                        "subtitle": r.subtitle,
                        "category": r.category,
                        "score": r.score,
                    })
                }),
        );
        results.extend(
            ctx.command_search
                .search(&query, limit)
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "title": r.title,
                        "subtitle": r.subtitle,
                        "category": r.category,
                        "score": r.score,
                    })
                }),
        );

        let mut arr = results;
        arr.sort_by(|a, b| {
            let sa = a.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let sb = b.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });
        arr.truncate(limit as usize);

        Ok(CommandOutput::ok_with_data(
            format!("找到 {} 项结果", arr.len()),
            serde_json::Value::Array(arr),
        ))
    }
}

// ==================== index 命令 ====================

pub struct IndexCommand;

#[async_trait::async_trait]
impl Command for IndexCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("index", "文件索引管理")
            .with_aliases(&["idx"])
            .with_usage("index <build|update|stats|add-root|remove-root> [options]")
    }

    async fn execute(
        &self,
        args: &[String],
        ctx: &CommandContext,
    ) -> crate::core::error::Result<CommandOutput> {
        if args.is_empty() {
            return Ok(CommandOutput::err("用法：index <build|update|stats|add-root|remove-root>"));
        }

        match args[0].as_str() {
            "build" => self.build_index(ctx).await,
            "update" => self.update_index(ctx).await,
            "stats" => self.stats(ctx).await,
            "add-root" => self.add_root(ctx, &args[1..]).await,
            "remove-root" => self.remove_root(ctx, &args[1..]).await,
            _ => Ok(CommandOutput::err("未知子命令：build|update|stats|add-root|remove-root")),
        }
    }
}

impl IndexCommand {
    async fn build_index(&self, ctx: &CommandContext) -> crate::core::error::Result<CommandOutput> {
        let roots = ctx.settings_repo.get().file_search_roots.clone();
        if roots.is_empty() {
            return Ok(CommandOutput::err("未配置搜索根目录，请先使用 index add-root 添加"));
        }

        match ctx.file_search.build_index().await {
            Ok(_) => {
                let total = ctx.file_search.total();
                Ok(CommandOutput::ok(format!("索引构建完成，共 {} 个文件", total)))
            }
            Err(e) => Ok(CommandOutput::err(format!("索引构建失败: {}", e))),
        }
    }

    async fn update_index(&self, ctx: &CommandContext) -> crate::core::error::Result<CommandOutput> {
        match ctx.file_search.update_index() {
            Ok(_) => Ok(CommandOutput::ok("索引已更新")),
            Err(e) => Ok(CommandOutput::err(format!("索引更新失败: {}", e))),
        }
    }

    async fn stats(&self, ctx: &CommandContext) -> crate::core::error::Result<CommandOutput> {
        let stats = serde_json::json!({
            "files": ctx.file_search.total(),
            "apps": ctx.app_search.total(),
            "commands": ctx.command_search.search("", 0).len(),
            "roots": ctx.settings_repo.get().file_search_roots,
        });
        Ok(CommandOutput::ok_with_data("索引统计", stats))
    }

    async fn add_root(&self, ctx: &CommandContext, args: &[String]) -> crate::core::error::Result<CommandOutput> {
        if args.is_empty() {
            return Ok(CommandOutput::err("用法：index add-root <path>"));
        }

        let path = args.join(" ");
        let path_buf = std::path::PathBuf::from(&path);

        if !path_buf.exists() {
            return Ok(CommandOutput::err(format!("路径不存在: {}", path)));
        }

        ctx.settings_repo.update(Box::new(move |s| {
            if !s.file_search_roots.contains(&path_buf) {
                s.file_search_roots.push(path_buf);
            }
        }))?;

        Ok(CommandOutput::ok(format!("已添加搜索根目录: {}", path)))
    }

    async fn remove_root(&self, ctx: &CommandContext, args: &[String]) -> crate::core::error::Result<CommandOutput> {
        if args.is_empty() {
            return Ok(CommandOutput::err("用法：index remove-root <path>"));
        }

        let path = args.join(" ");
        let path_buf = std::path::PathBuf::from(&path);

        ctx.settings_repo.update(Box::new(move |s| {
            s.file_search_roots.retain(|p| p != &path_buf);
        }))?;

        Ok(CommandOutput::ok(format!("已移除搜索根目录: {}", path)))
    }
}

/// 注册所有搜索相关命令
pub fn register_commands(reg: &mut crate::core::command::CommandRegistry) {
    reg.register(SearchCommand);
    reg.register(IndexCommand);
}
