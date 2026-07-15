//! SearchSource trait —— 所有可搜索数据源的统一抽象.
//!
//! 设计目标:
//! - 编排器 (`services::search::SearchEngine`) 不感知具体数据源
//! - 新增 source (e.g., "bookmarks", "clipboard") 只需 impl `SearchSource` 并 push 到 `sources` Vec
//! - 所有 engine 互不直接 import, 编排由 `SearchEngine` 负责
//!
//! 实现要求:
//! - 必须 `Send + Sync` (跨 await 边界 + 多线程共享)
//! - `search` 返回的 `SearchResult.category` 应与 `category()` 方法一致
//! - 默认 `search_after` 实现回退到 `search`, 子类可覆盖以支持分页

use crate::search_engine::models::{SearchCategory, SearchResult};

/// 搜索数据源统一接口.
pub trait SearchSource: Send + Sync {
    /// 数据源名称 (用于日志/debug).
    fn name(&self) -> &'static str;

    /// 数据源提供的 category.
    fn category(&self) -> SearchCategory;

    /// 在 `query` 下搜索, 最多返回 `limit` 条.
    fn search(&self, query: &str, limit: u32) -> Vec<SearchResult>;

    /// 分页搜索: 从 `after_id` 之后继续取 `limit` 条. 默认回退到 `search`.
    fn search_after(&self, _query: &str, _after_id: i64, limit: u32) -> Vec<SearchResult> {
        // 默认实现: 大多数 source 不需要专门的分页, 直接 search + 前端过滤
        self.search(_query, limit)
    }

    /// 当前 source 的索引总数 (用于状态展示).
    fn total(&self) -> usize {
        0
    }

    /// 类别权重 (用于排序). 默认 1.0.
    fn category_weight(&self) -> f32 {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{SearchAction, SearchCategory, SearchResult};

    /// 最小可用的 mock source, 用于验证 trait 可扩展性.
    struct MockSource {
        name: &'static str,
        items: Vec<SearchResult>,
    }

    impl SearchSource for MockSource {
        fn name(&self) -> &'static str {
            self.name
        }

        fn category(&self) -> SearchCategory {
            SearchCategory::Apps
        }

        fn search(&self, query: &str, _limit: u32) -> Vec<SearchResult> {
            self.items
                .iter()
                .filter(|r| r.title.to_lowercase().contains(&query.to_lowercase()))
                .cloned()
                .collect()
        }

        fn total(&self) -> usize {
            self.items.len()
        }

        fn category_weight(&self) -> f32 {
            1.5
        }
    }

    fn mock_item(id: &str, title: &str) -> SearchResult {
        SearchResult {
            id: id.to_string(),
            title: title.to_string(),
            subtitle: String::new(),
            meta: None,
            icon: None,
            category: SearchCategory::Apps,
            result_type: crate::models::ResultType::UserApp,
            action: SearchAction::Launch("C:\\test.exe".into()),
            score: 1.0,
        }
    }

    #[test]
    fn trait_basic_methods() {
        let s = MockSource {
            name: "mock",
            items: vec![mock_item("1", "Chrome"), mock_item("2", "Firefox")],
        };
        assert_eq!(s.name(), "mock");
        assert_eq!(s.total(), 2);
        assert_eq!(s.category_weight(), 1.5);
        let r = s.search("chrome", 10);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].title, "Chrome");
    }

    #[test]
    fn trait_search_after_defaults_to_search() {
        let s = MockSource {
            name: "mock",
            items: vec![mock_item("1", "Chrome")],
        };
        let r = s.search_after("chrome", 0, 10);
        assert_eq!(r.len(), 1);
    }

    /// 验证 trait object 可放入 Vec (动态分发).
    #[test]
    fn trait_object_works_in_vec() {
        let sources: Vec<Box<dyn SearchSource>> = vec![
            Box::new(MockSource {
                name: "a",
                items: vec![mock_item("1", "A")],
            }),
            Box::new(MockSource {
                name: "b",
                items: vec![mock_item("2", "B")],
            }),
        ];
        assert_eq!(sources.len(), 2);
        let mut all: Vec<SearchResult> = vec![];
        for s in &sources {
            all.extend(s.search("", 10));
        }
        assert_eq!(all.len(), 2);
    }
}
