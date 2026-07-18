use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// 时间衰减半衰期 (毫秒): 24 小时.
/// 超过 24 小时未使用的应用, 其时间衰减因子降至 0.5.
const DECAY_HALF_LIFE_MS: f64 = 86_400_000.0;

/// 时间衰减最小值: 即使很久没用, 也保留 10% 的基础分.
const DECAY_MIN_FACTOR: f64 = 0.1;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppStat {
    pub app_path: String,
    pub launch_count: u64,
    pub last_launched: i64,
    pub name: String,
}

impl AppStat {
    /// 计算时间衰减因子: 基于当前时间与最后启动时间的差距.
    /// 使用指数衰减: factor = max(DECAY_MIN, exp(-ln(2) * elapsed / half_life))
    pub fn decay_factor(&self) -> f64 {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as f64)
            .unwrap_or(0.0);
        let elapsed = now_ms - self.last_launched as f64;
        if elapsed <= 0.0 {
            return 1.0;
        }
        let factor = (-std::f64::consts::LN_2 * elapsed / DECAY_HALF_LIFE_MS).exp();
        factor.max(DECAY_MIN_FACTOR)
    }

    /// 综合评分: 启动次数 × 时间衰减因子.
    /// 启动次数使用对数缩放, 避免高频应用垄断.
    pub fn composite_score(&self) -> f64 {
        let count_factor = ((self.launch_count as f64) + 1.0).ln();
        count_factor * self.decay_factor()
    }
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
        v.sort_by(|a, b| b.composite_score().partial_cmp(&a.composite_score()).unwrap_or(std::cmp::Ordering::Equal));
        v.into_iter().take(limit).collect()
    }

    /// 按启动次数排序 (不考虑时间衰减).
    pub fn top_by_count(&self, limit: usize) -> Vec<AppStat> {
        let mut v: Vec<AppStat> = self.inner.read().values().cloned().collect();
        v.sort_by(|a, b| b.launch_count.cmp(&a.launch_count));
        v.into_iter().take(limit).collect()
    }

    /// 按最近使用时间排序.
    pub fn top_by_recent(&self, limit: usize) -> Vec<AppStat> {
        let v = self.list();
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
