pub struct UsnJournalTestConfig {
    pub test_drives: Vec<char>,
    pub monitor_duration_ms: u64,
    pub max_record_count: usize,
    pub sample_size: usize,
    pub validity_threshold: f64,
}

impl Default for UsnJournalTestConfig {
    fn default() -> Self {
        UsnJournalTestConfig {
            test_drives: vec!['E'],
            monitor_duration_ms: 5000,
            max_record_count: 100,
            sample_size: 100,
            validity_threshold: 0.90,
        }
    }
}

impl UsnJournalTestConfig {
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
    
    pub fn with_monitor_duration(mut self, duration_ms: u64) -> Self {
        self.monitor_duration_ms = duration_ms;
        self
    }
}