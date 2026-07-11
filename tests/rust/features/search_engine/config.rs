pub struct SearchEngineTestConfig {
    pub test_drives: Vec<char>,
    pub index_timeout_ms: u64,
    pub sample_size: usize,
    pub search_limit: u32,
    pub max_search_time_ms: u64,
    pub search_keywords: Vec<String>,
    pub validity_threshold: f64,
}

impl Default for SearchEngineTestConfig {
    fn default() -> Self {
        SearchEngineTestConfig {
            test_drives: vec!['E'],
            index_timeout_ms: 180000,
            sample_size: 100,
            search_limit: 100,
            max_search_time_ms: 1000,
            search_keywords: vec![
                "txt".to_string(),
                "pdf".to_string(),
                "doc".to_string(),
                "exe".to_string(),
                "rust".to_string(),
                "code".to_string(),
                "test".to_string(),
                "project".to_string(),
                "data".to_string(),
                "config".to_string(),
            ],
            validity_threshold: 0.85,
        }
    }
}

impl SearchEngineTestConfig {
    pub fn from_file(path: &str) -> Self {
        if std::path::Path::new(path).exists() {
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    if let Ok(config) = serde_json::from_str(&content) {
                        return config;
                    }
                }
                Err(_) => {}
            }
        }
        Self::default()
    }
    
    pub fn with_drives(mut self, drives: Vec<char>) -> Self {
        self.test_drives = drives;
        self
    }
    
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.index_timeout_ms = timeout_ms;
        self
    }
}