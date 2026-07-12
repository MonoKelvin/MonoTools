//! 搜索引擎协调 - 合并多个来源的结果
//! 
//! 核心功能：
//! - 并行搜索多个引擎
//! - 统一结果排序（基于分数 + 类别权重）
//! - 结果去重

use crate::engines::app_search::AppSearchEngine;
use crate::engines::command_search::CommandSearchEngine;
use crate::engines::file_search::FileSearchEngine;
use crate::models::{SearchAction, SearchResult, SearchCategory};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::collections::HashSet;
use std::sync::Arc;

const CATEGORY_WEIGHTS: [(SearchCategory, f32); 3] = [
    (SearchCategory::Apps, 0.8),
    (SearchCategory::Commands, 1.2),
    (SearchCategory::Files, 1.0),
];

pub struct SearchEngine {
    pub apps: Arc<AppSearchEngine>,
    pub files: Arc<FileSearchEngine>,
    pub commands: Arc<CommandSearchEngine>,
    fuzzy_matcher: SkimMatcherV2,
}

impl SearchEngine {
    pub fn new(
        apps: Arc<AppSearchEngine>,
        files: Arc<FileSearchEngine>,
        commands: Arc<CommandSearchEngine>,
    ) -> Self {
        Self {
            apps,
            files,
            commands,
            fuzzy_matcher: SkimMatcherV2::default(),
        }
    }

    pub fn search(&self, query: &str, limit: u32) -> Vec<SearchResult> {
        let mut combined: Vec<SearchResult> = Vec::new();

        combined.extend(self.apps.search(query, limit));
        combined.extend(self.files.search(query, limit));
        combined.extend(self.commands.search(query, limit));

        self.post_process(query, combined, limit)
    }

    pub fn search_by_category(&self, query: &str, category: SearchCategory, limit: u32) -> Vec<SearchResult> {
        let results = match category {
            SearchCategory::Apps => self.apps.search(query, limit),
            SearchCategory::Files => self.files.search(query, limit),
            SearchCategory::Commands => self.commands.search(query, limit),
            SearchCategory::All => return self.search(query, limit),
        };
        self.post_process(query, results, limit)
    }

    fn post_process(&self, query: &str, mut results: Vec<SearchResult>, limit: u32) -> Vec<SearchResult> {
        if query.is_empty() {
            // 空查询: 不截断 (上层传入的 limit 已经是 2000), 按 score 倒序即可.
            results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            return results;
        }

        results = self.apply_fuzzy_score(query, results);
        results = self.apply_category_weight(results);
        results = self.remove_duplicates(results);

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit as usize);

        results
            .into_iter()
            .map(|r| with_default_action(r))
            .collect()
    }

    fn apply_fuzzy_score(&self, query: &str, mut results: Vec<SearchResult>) -> Vec<SearchResult> {
        let q = query.to_lowercase();
        for r in results.iter_mut() {
            let title_lower = r.title.to_lowercase();
            let subtitle_lower = r.subtitle.to_lowercase();
            
            let title_score = self.fuzzy_matcher.fuzzy_match(&title_lower, &q)
                .map(|s| s as f32 / 100.0)
                .unwrap_or(0.0);
            
            let subtitle_score = self.fuzzy_matcher.fuzzy_match(&subtitle_lower, &q)
                .map(|s| s as f32 / 200.0)
                .unwrap_or(0.0);
            
            r.score = r.score * 0.6 + title_score * 0.3 + subtitle_score * 0.1;
        }
        results
    }

    fn apply_category_weight(&self, mut results: Vec<SearchResult>) -> Vec<SearchResult> {
        for r in results.iter_mut() {
            if let Some(&(_, weight)) = CATEGORY_WEIGHTS.iter().find(|(c, _)| c == &r.category) {
                r.score *= weight;
            }
        }
        results
    }

    fn remove_duplicates(&self, results: Vec<SearchResult>) -> Vec<SearchResult> {
        let mut seen = HashSet::new();
        let mut unique = Vec::new();
        
        for r in results {
            let key = (r.title.clone(), r.category.clone());
            if !seen.contains(&key) {
                seen.insert(key);
                unique.push(r);
            }
        }
        
        unique
    }

    pub fn total_indexed(&self) -> IndexStats {
        IndexStats {
            apps: self.apps.total(),
            files: self.files.total(),
            commands: self.commands.total(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IndexStats {
    pub apps: usize,
    pub files: usize,
    pub commands: usize,
}

fn with_default_action(mut r: SearchResult) -> SearchResult {
    if matches!(r.action, SearchAction::Launch(_)) && r.title.is_empty() {
        r.action = SearchAction::Launch("explorer.exe".into());
    }
    r
}
