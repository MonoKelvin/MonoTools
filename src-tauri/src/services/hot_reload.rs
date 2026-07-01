use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use anyhow::Result;
use notify::{Watcher, RecursiveMode, Event, EventKind};
use tokio::sync::RwLock;
use tracing::{info, warn};

pub struct HotReloadManager {
    plugin_dirs: HashMap<String, PathBuf>,
    watchers: RwLock<HashMap<String, notify::RecommendedWatcher>>,
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
        use notify::Config;

        let plugin_id = plugin_id.to_string();

        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        let watcher = notify::RecommendedWatcher::try_new(
            move |result: Result<Event, notify::Error>| {
                if let Ok(event) = result {
                    let _ = tx.try_send(event);
                }
            },
            Config::default().with_poll_interval(Duration::from_millis(500)),
        )?;

        watcher.watch(&path, RecursiveMode::Recursive)?;

        self.plugin_dirs.insert(plugin_id.clone(), path);
        self.watchers.write().await.insert(plugin_id.clone(), watcher);

        // 启动监听任务
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                info!("Plugin change detected: {:?}", event);
                // TODO: 触发插件重载
            }
        });

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
