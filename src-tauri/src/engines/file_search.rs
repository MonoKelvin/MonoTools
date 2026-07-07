//! 文件搜索服务 - 包装 platform/usn.rs 的引擎
use crate::error::Result;
use crate::models::{FileResult, SearchAction, SearchCategory, SearchResult};
use crate::platform::windows::usn::{FileEngine, FallbackFileEngine};
use crate::repositories::SettingsRepo;
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

pub struct FileSearchService {
    pub engine: RwLock<Option<Arc<dyn FileEngine>>>,
    pub settings: Arc<dyn SettingsRepo>,
}

impl FileSearchService {
    pub fn new() -> Self {
        let settings: Arc<dyn SettingsRepo> = Arc::new(crate::repositories::InMemorySettingsRepo::new(
            crate::models::Settings::default(),
        ));
        Self {
            engine: RwLock::new(None),
            settings,
        }
    }

    pub async fn build_index(&self) -> Result<()> {
        let roots: Vec<PathBuf> = self.settings.get().file_search_roots.clone();
        let engine = Arc::new(FallbackFileEngine::new(roots));
        engine.build_index()?;
        *self.engine.write() = Some(engine);
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
        Self::new()
    }
}
