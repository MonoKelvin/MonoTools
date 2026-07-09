pub struct SearchEngineTestConfig {
    pub test_file_count: usize,
    pub search_limit: u32,
    pub sample_size: usize,
    pub validity_threshold: f64,
    pub max_build_time_ms: u64,
    pub max_search_time_ms: u64,
    pub search_keyword_count: usize,
}

impl Default for SearchEngineTestConfig {
    fn default() -> Self {
        SearchEngineTestConfig {
            test_file_count: 100,
            search_limit: 50,
            sample_size: 100,
            validity_threshold: 0.85,
            max_build_time_ms: 30000,
            max_search_time_ms: 1000,
            search_keyword_count: 15,
        }
    }
}

impl SearchEngineTestConfig {
    pub fn from_file(_path: &str) -> Self {
        Self::default()
    }
}