//! search 命令
use crate::command::command_trait::{Command, CommandSpec};
use crate::command::{CommandContext, CommandOutput};

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
    ) -> crate::error::Result<CommandOutput> {
        if args.is_empty() {
            return Ok(CommandOutput::err("用法：search <query>"));
        }

        let mut limit: u32 = 10;
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
