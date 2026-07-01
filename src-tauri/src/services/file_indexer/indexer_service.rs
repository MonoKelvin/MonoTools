pub struct IndexerService {
    indexer: crate::services::file_indexer::UsnIndexer,
    search_engine: crate::services::file_indexer::SearchEngine,
}

impl IndexerService {
    pub fn new() -> Result<Self> {
        let indexer = crate::services::file_indexer::UsnIndexer::new()?;
        let search_engine = crate::services::file_indexer::SearchEngine::new(indexer.pool.clone());

        Ok(Self { indexer, search_engine })
    }

    pub async fn build_index(&self) -> Result<crate::services::file_indexer::IndexStats> {
        self.indexer.build_index().await
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<crate::models::search::SearchResult>> {
        // TODO: 实现搜索
        Ok(vec![])
    }
}
