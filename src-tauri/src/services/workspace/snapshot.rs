use crate::models::workspace::{AppSnapshot, Workspace};
use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;

pub struct SnapshotService;

impl SnapshotService {
    /// 捕获当前工作区快照
    pub fn capture(name: Option<&str>, description: Option<&str>) -> Result<Workspace> {
        let apps = crate::services::workspace::process_enum::WindowEnumerator::capture_workspace()?;

        let workspace = Workspace {
            id: Uuid::new_v4().to_string(),
            name: name.unwrap_or("未命名工作区").to_string(),
            description: description.map(|s| s.to_string()),
            icon: None,
            color: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_restored_at: None,
            apps,
            auto_start: false,
        };

        Ok(workspace)
    }

    /// 保存工作区到文件
    pub fn save(workspace: &Workspace, path: Option<&str>) -> Result<String> {
        let save_path = if let Some(p) = path {
            std::path::PathBuf::from(p)
        } else {
            Self::default_save_path(&workspace.id)?
        };

        // 确保目录存在
        if let Some(parent) = save_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(workspace)?;
        std::fs::write(&save_path, json)?;

        Ok(save_path.to_string_lossy().to_string())
    }

    /// 加载工作区从文件
    pub fn load(path: &str) -> Result<Workspace> {
        let json = std::fs::read_to_string(path)?;
        let workspace: Workspace = serde_json::from_str(&json)?;
        Ok(workspace)
    }

    /// 获取默认保存路径
    fn default_save_path(id: &str) -> Result<std::path::PathBuf> {
        let data_dir = Self::get_data_dir()?;
        Ok(data_dir.join("workspaces").join(format!("{}.json", id)))
    }

    /// 获取数据目录
    fn get_data_dir() -> Result<std::path::PathBuf> {
        if let Ok(dir) = std::env::var("MONOTOOLS_DATA_DIR") {
            return Ok(std::path::PathBuf::from(dir));
        }

        let appdata = dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?;

        Ok(appdata.join("MonoTools"))
    }
}
