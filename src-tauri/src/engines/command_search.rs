//! 自定义命令搜索 - 与应用引擎并列
use crate::models::{ResultType, SearchAction, SearchCategory, SearchResult};
use crate::repositories::CommandRepo;
use std::sync::Arc;

pub struct CommandSearchEngine {
    pub command_repo: Arc<dyn CommandRepo>,
}

impl CommandSearchEngine {
    pub fn new(command_repo: Arc<dyn CommandRepo>) -> Self {
        Self { command_repo }
    }

    pub fn total(&self) -> usize {
        self.command_repo.list_enabled().len()
    }

    pub fn search(&self, query: &str, limit: u32) -> Vec<SearchResult> {
        let q = query.to_lowercase();
        let cmds = self.command_repo.list_enabled();
        if q.is_empty() {
            // 空查询: 列出所有已启用命令, 不截断. 按 name 排序, 方便浏览.
            let mut results: Vec<SearchResult> = cmds
                .into_iter()
                .map(|cmd| SearchResult {
                    id: cmd.id.clone(),
                    title: cmd.name.clone(),
                    subtitle: cmd.command.clone() + " " + &cmd.args.join(" "),
                    // 命令没有"大小"等次级元信息.
                    meta: None,
                    icon: cmd.icon.clone(),
                    category: SearchCategory::Commands,
                    result_type: ResultType::Command,
                    action: SearchAction::Run {
                        command: cmd.command.clone(),
                        args: cmd.args.clone(),
                    },
                    score: 0.0,
                })
                .collect();
            results.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
            return results;
        }
        let mut results: Vec<SearchResult> = Vec::new();
        for cmd in cmds {
            let name_l = cmd.name.to_lowercase();
            let kw_l = cmd.keyword.to_lowercase();
            let mut score = 0.0;
            if name_l == q {
                score += 100.0;
            } else if name_l.starts_with(&q) || kw_l.starts_with(&q) {
                score += 80.0;
            } else if name_l.contains(&q) || kw_l.contains(&q) {
                score += 50.0;
            }
            if score == 0.0 {
                continue;
            }
            // 频率加权
            if let Some(last) = cmd.last_used_at {
                let age = chrono::Utc::now().timestamp() - last;
                if age < 3600 * 24 * 7 {
                    score += 10.0;
                }
            }

            results.push(SearchResult {
                id: cmd.id.clone(),
                title: cmd.name.clone(),
                subtitle: cmd.command.clone() + " " + &cmd.args.join(" "),
                meta: None,
                icon: cmd.icon.clone(),
                category: SearchCategory::Commands,
                result_type: ResultType::Command,
                action: SearchAction::Run {
                    command: cmd.command.clone(),
                    args: cmd.args.clone(),
                },
                score,
            });
            if results.len() >= limit as usize {
                break;
            }
        }
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results
    }
}
