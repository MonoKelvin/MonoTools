use std::path::Path;
use anyhow::Result;
use blake3::hash;

pub struct IconCache {
    dir: std::path::PathBuf,
    max_size_mb: usize,
}

impl IconCache {
    pub fn new(dir: PathBuf, max_size_mb: usize) -> Self {
        Self { dir, max_size_mb }
    }

    /// 获取缓存的图标路径
    pub fn get(&self, exe_path: &str) -> Option<std::path::PathBuf> {
        let hash = hash(exe_path.as_bytes());
        let path = self.dir.join(format!("{}.png", hex::encode(hash)));
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// 提取图标并缓存
    pub async fn extract(&self, exe_path: &str) -> Result<std::path::PathBuf> {
        let hash = hash(exe_path.as_bytes());
        let out_path = self.dir.join(format!("{}.png", hex::encode(hash)));

        // 如果已缓存，直接返回
        if out_path.exists() {
            return Ok(out_path);
        }

        // TODO: 实现图标提取逻辑
        // 使用 windows-rs 提取图标资源
        // 暂时返回占位路径
        Err(anyhow::anyhow!("Icon extraction not implemented"))
    }

    /// 清理旧缓存
    pub async fn cleanup_if_needed(&self) -> Result<()> {
        // TODO: 检查缓存大小，超过限制时清理最旧的图标
        Ok(())
    }
}
