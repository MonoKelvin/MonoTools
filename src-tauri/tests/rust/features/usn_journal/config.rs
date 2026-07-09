pub struct UsnJournalTestConfig {
    pub sample_size: usize,
    pub validity_threshold: f64,
    pub monitor_wait_seconds: u64,
    pub max_sample_records: usize,
}

impl Default for UsnJournalTestConfig {
    fn default() -> Self {
        UsnJournalTestConfig {
            sample_size: 100,
            validity_threshold: 0.90,
            monitor_wait_seconds: 5,
            max_sample_records: 100,
        }
    }
}

impl UsnJournalTestConfig {
    pub fn from_file(_path: &str) -> Self {
        Self::default()
    }
}