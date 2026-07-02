use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use anyhow::Result;
use notify::{Watcher, RecursiveMode, Event};
use tokio::sync::RwLock;
use tracing::info;

pub struct HotReloadManager {
    plugin_dirs: HashMap<String, PathBuf>,
    watchers: RwLock<HashMap<String, Box<dyn Watcher>>>,
}

impl HotReloadManager {
    pub fn new() -> Self {
        Self {
            plugin_dirs: HashMap::new(),
            watchers: RwLock::new(HashMap::new()),
        }
    }

    /// 注册插件目录监控
    pub async fn watch_plugin(&mut self, plugin_id: &str, path: PathBuf) -> Result<()> {
        let plugin_id = plugin_id.to_string();
        // TODO: 实现文件监控
        // 暂时只是记录目录
        self.plugin_dirs.insert(plugin_id, path);
        Ok(())
    }

    /// 停止监控插件
    pub async fn unwatch_plugin(&mut self, plugin_id: &str) {
        self.plugin_dirs.remove(plugin_id);
        self.watchers.write().await.remove(plugin_id);
    }
}

impl Default for HotReloadManager {
    fn default() -> Self {
        Self::new()
    }
}
