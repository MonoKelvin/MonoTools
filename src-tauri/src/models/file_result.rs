use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 文件搜索结果（在搜索结果 platform/windows/usn.rs 中复用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileResult {
    pub path: PathBuf,
    pub name: String,
    pub extension: Option<String>,
    pub size: i64,
    pub modified_at: i64,
    pub is_directory: bool,
}
