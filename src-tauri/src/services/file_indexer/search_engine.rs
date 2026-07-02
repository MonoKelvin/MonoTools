use crate::models::file_entry::FileEntry;
use crate::models::search::{SearchQuery, SearchResult};
use anyhow::Result;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 文件搜索引擎
pub struct SearchEngine {
    db: Arc<RwLock<Connection>>,
}

impl SearchEngine {
    pub fn new(db: Arc<RwLock<Connection>>) -> Self {
        Self { db }
    }

    /// 搜索文件
    pub async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        if query.tokens.is_empty() {
            return Ok(vec![]);
        }

        let db = self.db.read().await;
        let results = Vec::new();

        // TODO: 实现完整的搜索逻辑

        Ok(results)
    }

    /// 精确匹配
    fn exact_match(&self, query: &str) -> Result<Vec<FileEntry>> {
        Ok(vec![])
    }

    /// 前缀匹配
    fn prefix_match(&self, query: &str) -> Result<Vec<FileEntry>> {
        Ok(vec![])
    }

    /// 模糊匹配
    fn fuzzy_match(&self, query: &str) -> Result<Vec<FileEntry>> {
        Ok(vec![])
    }

    /// 拼音匹配
    fn pinyin_match(&self, query: &str) -> Result<Vec<FileEntry>> {
        Ok(vec![])
    }

    /// 合并并排序结果
    fn merge_and_rank(
        &self,
        exact: Vec<FileEntry>,
        prefix: Vec<FileEntry>,
        fuzzy: Vec<FileEntry>,
        pinyin: Vec<FileEntry>,
    ) -> Vec<FileEntry> {
        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for entry in exact.into_iter().chain(prefix).chain(fuzzy).chain(pinyin) {
            if seen.insert(entry.id) {
                results.push(entry);
            }
        }

        results
    }
}
