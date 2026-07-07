//! 搜索引擎协调 - 合并多个来源的结果
use crate::engines::app_search::AppSearchEngine;
use crate::engines::command_search::CommandSearchEngine;
use crate::engines::file_search::FileSearchService;
use crate::engines::startup_search::StartupSearchService;
use crate::models::{SearchAction, SearchResult};
use std::sync::Arc;

pub struct SearchEngine {
    pub apps: Arc<AppSearchEngine>,
    pub files: Arc<FileSearchService>,
    pub commands: Arc<CommandSearchEngine>,
    pub startups: Arc<StartupSearchService>,
}

impl SearchEngine {
    pub fn new(
        apps: Arc<AppSearchEngine>,
        files: Arc<FileSearchService>,
        commands: Arc<CommandSearchEngine>,
        startups: Arc<StartupSearchService>,
    ) -> Self {
        Self {
            apps,
            files,
            commands,
            startups,
        }
    }

    pub fn search(&self, query: &str, limit: u32) -> Vec<SearchResult> {
        let mut combined: Vec<SearchResult> = Vec::new();

        combined.extend(self.apps.search(query, limit));
        combined.extend(self.files.search(query, limit));
        combined.extend(self.commands.search(query, limit));
        combined.extend(self.startups.search(query, limit));

        combined.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        combined.truncate(limit as usize);
        combined
            .into_iter()
            .map(|r| with_default_action(r))
            .collect()
    }
}

fn with_default_action(mut r: SearchResult) -> SearchResult {
    if matches!(r.action, SearchAction::Launch(_)) && r.title.is_empty() {
        r.action = SearchAction::Launch("explorer.exe".into());
    }
    r
}
