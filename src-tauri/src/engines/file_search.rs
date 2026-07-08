//! 文件搜索服务 - 基于 SQLite FTS5 的高性能全文搜索
use crate::error::Result;
use crate::engines::file_fts5::FileFts5Engine;
use crate::models::{FileResult, SearchAction, SearchCategory, SearchResult};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

pub struct FileSearchService {
    engine: RwLock<Option<Arc<FileFts5Engine>>>,
    roots: Vec<PathBuf>,
}

impl FileSearchService {
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self {
            engine: RwLock::new(None),
            roots,
        }
    }

    /// 构建文件索引（SQLite FTS5）
    pub async fn build_index(&self) -> Result<()> {
        let roots = self.roots.clone();
        let engine = Arc::new(FileFts5Engine::new(get_db_path(), roots)?);
        engine.build_index()?;
        *self.engine.write() = Some(engine);
        Ok(())
    }

    /// 增量更新索引
    pub fn update_index(&self) -> Result<()> {
        let guard = self.engine.read();
        if let Some(engine) = guard.as_ref() {
            engine.update_index()?;
        }
        Ok(())
    }

    pub fn total(&self) -> usize {
        self.engine
            .read()
            .as_ref()
            .map(|e| e.total())
            .unwrap_or(0)
    }

    pub fn search(&self, query: &str, limit: u32) -> Vec<SearchResult> {
        let guard = self.engine.read();
        let Some(engine) = guard.as_ref() else {
            return vec![];
        };
        engine
            .search(query, limit)
            .into_iter()
            .map(|f: FileResult| SearchResult {
                id: f.path.to_string_lossy().to_string(),
                title: f.name.clone(),
                subtitle: f.path.to_string_lossy().to_string(),
                icon: None,
                category: SearchCategory::Files,
                action: SearchAction::Open(f.path.to_string_lossy().to_string()),
                score: 1.0,
            })
            .collect()
    }
}

impl Default for FileSearchService {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

/// 获取 SQLite 数据库文件路径
fn get_db_path() -> PathBuf {
    if let Ok(app_data) = std::env::var("APPDATA") {
        let p = PathBuf::from(app_data).join("MonoTools").join("file_index.db");
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        return p;
    }
    PathBuf::from("file_index.db")
}
