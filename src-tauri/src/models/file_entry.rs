use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub parent_path: String,
    pub ext: Option<String>,
    pub size: i64,
    pub created_at: i64,
    pub modified_at: i64,
    pub accessed_at: i64,
    pub is_dir: bool,
    pub ascii_sum: i32,
    pub pinyin: Option<String>,
    pub priority: i32,
    pub volume: String,
}
