use std::collections::HashMap;
use anyhow::Result;
use serde_json::Value;
use crate::commands::bus::{Command, CommandHandler, CommandContext};
use crate::services::file_indexer::index_store::IndexStore;

pub struct SearchCommandHandler {
    index_store: std::sync::Arc<tokio::sync::RwLock<IndexStore>>,
}

impl SearchCommandHandler {
    pub fn new(index_store: std::sync::Arc<tokio::sync::RwLock<IndexStore>>) -> Self {
        Self { index_store }
    }
}

#[async_trait::async_trait]
impl CommandHandler for SearchCommandHandler {
    async fn execute(&self, cmd: &Command, ctx: &CommandContext) -> Result<Value> {
        match cmd.action.as_str() {
            "files" => self.search_files(cmd).await,
            "apps" => self.search_apps(cmd).await,
            "all" => self.search_all(cmd).await,
            "providers" => self.list_providers().await,
            "history" => self.get_history(cmd).await,
            _ => Err(anyhow::anyhow!("Unknown search action: {}", cmd.action)),
        }
    }

    fn validate(&self, cmd: &Command) -> Result<()> {
        match cmd.action.as_str() {
            "files" | "apps" | "all" => {
                if !cmd.args.contains_key("query") {
                    return Err(anyhow::anyhow!("Missing required argument: query"));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl SearchCommandHandler {
    /// 搜索文件
    async fn search_files(&self, cmd: &Command) -> Result<Value> {
        let query = cmd.args.get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing query argument"))?;

        let limit = cmd.args.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;

        let store = self.index_store.read().await;
        let files = store.search_files(query, limit).await?;

        let results: Vec<Value> = files.into_iter()
            .map(|f| {
                serde_json::json!({
                    "id": f.id,
                    "title": f.name,
                    "subtitle": f.path,
                    "source": "文件",
                    "type": "file",
                    "path": f.path,
                    "is_dir": f.is_dir,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "results": results,
            "total": results.len(),
            "query": query
        }))
    }

    /// 搜索应用
    async fn search_apps(&self, _cmd: &Command) -> Result<Value> {
        // TODO: 实现应用搜索
        Ok(serde_json::json!({
            "results": [],
            "total": 0
        }))
    }

    /// 聚合搜索（所有提供者）
    async fn search_all(&self, cmd: &Command) -> Result<Value> {
        let query = cmd.args.get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing query argument"))?;

        let limit = cmd.args.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;

        // 并行搜索文件和应用的简化实现
        let mut all_results = Vec::new();

        // 搜索文件
        let store = self.index_store.read().await;
        if let Ok(files) = store.search_files(query, limit).await {
            let file_results: Vec<Value> = files.into_iter()
                .map(|f| {
                    serde_json::json!({
                        "id": format!("file:{}", f.id),
                        "title": f.name,
                        "subtitle": f.path,
                        "source": "文件",
                        "type": "file",
                        "path": f.path,
                        "is_dir": f.is_dir,
                        "score": 1.0
                    })
                })
                .collect();
            all_results.extend(file_results);
        }

        // TODO: 搜索应用
        // TODO: 搜索工作区

        // 按相关性排序
        all_results.sort_by(|a, b| {
            let score_a = a.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let score_b = b.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            score_b.partial_cmp(&score_a).unwrap()
        });

        // 限制结果数量
        all_results.truncate(limit);

        Ok(serde_json::json!({
            "results": all_results,
            "total": all_results.len(),
            "query": query
        }))
    }

    /// 列出所有搜索提供者
    async fn list_providers(&self) -> Result<Value> {
        let providers = vec![
            serde_json::json!({
                "id": "builtin:file-search",
                "name": "文件搜索",
                "type": "file",
                "enabled": true,
                "priority": 100
            }),
            serde_json::json!({
                "id": "builtin:app-launcher",
                "name": "应用启动器",
                "type": "app",
                "enabled": true,
                "priority": 90
            }),
            serde_json::json!({
                "id": "builtin:workspace-manager",
                "name": "工作区管理",
                "type": "workspace",
                "enabled": true,
                "priority": 80
            }),
            serde_json::json!({
                "id": "builtin:command-palette",
                "name": "命令面板",
                "type": "command",
                "enabled": true,
                "priority": 70
            }),
        ];

        Ok(serde_json::json!({
            "providers": providers
        }))
    }

    /// 获取搜索历史
    async fn get_history(&self, cmd: &Command) -> Result<Value> {
        let limit = cmd.args.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        // TODO: 从数据库读取搜索历史
        Ok(serde_json::json!({
            "history": [],
            "limit": limit
        }))
    }
}
