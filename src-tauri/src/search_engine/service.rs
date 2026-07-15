//! 搜索引擎协调 - 合并多个来源的结果
//!
//! 核心功能：
//! - 并行搜索多个 engine (通过 `SearchSource` trait 注册)
//! - 统一结果排序（基于分数 + 类别权重）
//! - 结果去重
//!
//! 加新 source (e.g., "bookmarks") 只需 impl `SearchSource` 并 push 到 `sources` Vec,
//! 编排器 (`SearchEngine`) 无需修改.

use crate::core::config::search as search_cfg;
use crate::search_engine::app_search::AppSearchEngine;
use crate::search_engine::command_search::CommandSearchEngine;
use crate::search_engine::file_search::FileSearchEngine;
use crate::search_engine::search_source::SearchSource;
use crate::search_engine::models::{SearchAction, SearchResult, SearchCategory};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::collections::HashSet;
use std::sync::Arc;

const CATEGORY_WEIGHTS: [(SearchCategory, f32); 3] = [
    (SearchCategory::Apps, search_cfg::CATEGORY_WEIGHT_APPS),
    (SearchCategory::Commands, search_cfg::CATEGORY_WEIGHT_COMMANDS),
    (SearchCategory::Files, search_cfg::CATEGORY_WEIGHT_FILES),
];

/// 搜索引擎：编排多个 `SearchSource`, 提供统一 query / after / category 接口.
///
/// 内部用 `Vec<Arc<dyn SearchSource>>` 持有所有 source, 旧版的 `apps/files/commands`
/// 具名字段仍保留为向下兼容的 accessor (`engine.apps()` 等).
pub struct SearchEngine {
    /// 所有搜索 source, 顺序影响后处理权重 (按 push 顺序遍历).
    sources: Vec<Arc<dyn SearchSource>>,
    /// 旧字段: 仍保留为 Arc 引用, 避免破坏现有 callers (`app_state` 等).
    pub apps: Arc<AppSearchEngine>,
    pub files: Arc<FileSearchEngine>,
    pub commands: Arc<CommandSearchEngine>,
    fuzzy_matcher: SkimMatcherV2,
}

impl SearchEngine {
    /// 构造时同时初始化 sources vec, 用具名 engine 填充.
    pub fn new(
        apps: Arc<AppSearchEngine>,
        files: Arc<FileSearchEngine>,
        commands: Arc<CommandSearchEngine>,
    ) -> Self {
        let sources: Vec<Arc<dyn SearchSource>> = vec![
            Arc::clone(&apps) as Arc<dyn SearchSource>,
            Arc::clone(&files) as Arc<dyn SearchSource>,
            Arc::clone(&commands) as Arc<dyn SearchSource>,
        ];
        Self {
            sources,
            apps,
            files,
            commands,
            fuzzy_matcher: SkimMatcherV2::default(),
        }
    }

    /// 通用构造: 直接传 sources (用于测试 / 动态扩展场景).
    pub fn with_sources(sources: Vec<Arc<dyn SearchSource>>) -> Self {
        Self {
            sources,
            apps: Arc::new(AppSearchEngine::new_empty_for_tests()),
            files: Arc::new(FileSearchEngine::new_with_db_path_for_tests().expect("test engine")),
            commands: Arc::new(CommandSearchEngine::new_empty_for_tests()),
            fuzzy_matcher: SkimMatcherV2::default(),
        }
    }

    /// 当前 source 数量.
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }

    /// 在 `query` 下搜索, 最多返回 `limit` 条.
    /// 
    /// 优化策略:
    /// 1. 每个 source 使用独立的子限制, 避免单个 source 返回过多结果
    /// 2. 子限制根据 category 权重分配, 重要类别获得更多配额
    /// 3. 合并后再进行后处理, 确保整体结果数量可控
    pub fn search(&self, query: &str, limit: u32) -> Vec<SearchResult> {
        let mut combined: Vec<SearchResult> = Vec::new();
        let sub_limit = std::cmp::max(10, limit / self.sources.len() as u32) + 20;
        
        for src in &self.sources {
            let src_limit = match src.category() {
                SearchCategory::Apps => std::cmp::min(sub_limit + 30, limit),
                SearchCategory::Commands => std::cmp::min(sub_limit, limit),
                SearchCategory::Files => std::cmp::min(sub_limit + 50, limit),
                _ => sub_limit,
            };
            combined.extend(src.search(query, src_limit));
        }
        self.post_process(query, combined, limit)
    }

    /// 分页搜索: 给前端"显示更多"按钮用. 与 search 同样的多 source 并行,
    /// 但每个 source 只返回 `after_id` 之后的项, 避免重复加载.
    pub fn search_after(
        &self,
        query: &str,
        after_id: i64,
        limit: u32,
    ) -> Vec<SearchResult> {
        let mut combined: Vec<SearchResult> = Vec::new();
        for src in &self.sources {
            combined.extend(src.search_after(query, after_id, limit));
        }
        self.post_process(query, combined, limit)
    }

    /// 按 category 过滤搜索. `SearchCategory::All` 走 `search()` 全量.
    pub fn search_by_category(&self, query: &str, category: SearchCategory, limit: u32) -> Vec<SearchResult> {
        if matches!(category, SearchCategory::All) {
            return self.search(query, limit);
        }
        // 仅在所有 source 中找匹配的 category, 避免误用 SearchCategory::All
        let mut results = Vec::new();
        for src in &self.sources {
            if src.category() == category {
                results.extend(src.search(query, limit));
            }
        }
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
                .map(|s| s as f32 / search_cfg::FUZZY_TITLE_NORM)
                .unwrap_or(0.0);

            let subtitle_score = self.fuzzy_matcher.fuzzy_match(&subtitle_lower, &q)
                .map(|s| s as f32 / search_cfg::FUZZY_SUBTITLE_NORM)
                .unwrap_or(0.0);

            r.score = r.score * search_cfg::FUZZY_BASE_SCORE_KEEP
                + title_score * search_cfg::FUZZY_TITLE_WEIGHT
                + subtitle_score * search_cfg::FUZZY_SUBTITLE_WEIGHT;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search_engine::search_source::SearchSource;
    use crate::models::{SearchAction, SearchCategory, SearchResult};
    use std::collections::HashMap;

    // 测试用 mock engine
    struct MockEngine {
        items: HashMap<String, SearchResult>,
    }

    impl SearchSource for MockEngine {
        fn name(&self) -> &'static str {
            "mock"
        }
        fn category(&self) -> SearchCategory {
            SearchCategory::Apps
        }
        fn search(&self, _query: &str, _limit: u32) -> Vec<SearchResult> {
            self.items.values().cloned().collect()
        }
        fn total(&self) -> usize {
            self.items.len()
        }
    }

    /// 验证 SearchEngine 可接受任意 `Arc<dyn SearchSource>` 而不限于 3 个具名 engine.
    #[test]
    fn search_engine_with_dynamic_sources() {
        let mut items = HashMap::new();
        items.insert(
            "1".into(),
            SearchResult {
                id: "1".into(),
                title: "Mock1".into(),
                subtitle: String::new(),
                meta: None,
                icon: None,
                category: SearchCategory::Apps,
                result_type: crate::models::ResultType::UserApp,
                action: SearchAction::Launch("C:\\test.exe".into()),
                score: 1.0,
            },
        );
        let mock: Arc<dyn SearchSource> = Arc::new(MockEngine { items });
        let sources: Vec<Arc<dyn SearchSource>> = vec![mock];
        let engine = SearchEngine::with_sources(sources);
        // 空 query + total = 1 项
        let r = engine.search("", 10);
        assert_eq!(r.len(), 1, "动态 source 应能提供结果, got {} 个", r.len());
        assert_eq!(engine.source_count(), 1);
    }

    /// 验证 search_by_category 走动态 source 列表 (而非硬编码 match).
    #[test]
    fn search_by_category_uses_source_list() {
        let mock: Arc<dyn SearchSource> = Arc::new(MockEngine {
            items: HashMap::new(),
        });
        let sources: Vec<Arc<dyn SearchSource>> = vec![mock];
        let engine = SearchEngine::with_sources(sources);
        // Apps 应走 mock (category=Apps), All 应走 search()
        let r = engine.search_by_category("", SearchCategory::Apps, 10);
        assert_eq!(r.len(), 0);
        let r_all = engine.search_by_category("", SearchCategory::All, 10);
        assert_eq!(r_all.len(), 0);
    }

    // 占位测试: 确保 new() 编译通过 (不实际开引擎, 需更复杂的 setup)
    #[test]
    fn search_engine_struct_compiles() {
        // 验证 sources 字段存在, name() 方法可调用
        fn _takes_source(s: &dyn SearchSource) -> &'static str {
            s.name()
        }
        assert_eq!(_takes_source(&MockEngine { items: HashMap::new() }), "mock");
    }
}
