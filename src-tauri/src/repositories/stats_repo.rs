use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppStat {
    pub app_path: String,
    pub launch_count: u64,
    pub last_launched: i64,
    pub name: String,
}

/// 应用统计仓储
pub struct StatsRepo {
    inner: Arc<RwLock<HashMap<String, AppStat>>>,
}

impl StatsRepo {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn record_launch(&self, key: &str, name: &str, ts: i64) {
        let mut g = self.inner.write();
        let stat = g.entry(key.to_string()).or_insert_with(|| AppStat {
            app_path: key.to_string(),
            launch_count: 0,
            last_launched: ts,
            name: name.to_string(),
        });
        stat.launch_count += 1;
        stat.last_launched = ts;
        if !name.is_empty() {
            stat.name = name.to_string();
        }
    }

    pub fn get(&self, key: &str) -> Option<AppStat> {
        self.inner.read().get(key).cloned()
    }

    pub fn list(&self) -> Vec<AppStat> {
        let mut v: Vec<AppStat> = self.inner.read().values().cloned().collect();
        v.sort_by(|a, b| b.last_launched.cmp(&a.last_launched));
        v
    }

    pub fn top(&self, limit: usize) -> Vec<AppStat> {
        let mut v = self.list();
        v.sort_by(|a, b| b.launch_count.cmp(&a.launch_count));
        v.into_iter().take(limit).collect()
    }

    pub fn search_history(&self, last_n: usize) -> Vec<AppStat> {
        self.list().into_iter().take(last_n).collect()
    }
}

impl Default for StatsRepo {
    fn default() -> Self {
        Self::new()
    }
}
