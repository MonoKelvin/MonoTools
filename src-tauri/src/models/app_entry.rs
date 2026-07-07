use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub icon_path: Option<PathBuf>,
    pub category: String,
    pub last_launched: Option<i64>,
    pub launch_count: u32,
    /// 唯一短键，用于排序
    pub alias: Option<String>,
}
