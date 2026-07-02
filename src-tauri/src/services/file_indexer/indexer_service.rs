use anyhow::Result;

pub struct IndexerService {
    indexer: crate::services::file_indexer::UsnJournal,
}

impl IndexerService {
    pub fn new() -> Result<Self> {
        let indexer = crate::services::file_indexer::UsnJournal::new("C:".to_string())?;

        Ok(Self { indexer })
    }

    pub async fn build_index(&self) -> Result<()> {
        // TODO: 实现构建索引
        Ok(())
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<crate::models::search::SearchResult>> {
        // TODO: 实现搜索
        Ok(vec![])
    }
}
