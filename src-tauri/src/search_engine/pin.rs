//! Pin 项目仓储 —— 用户手动固定到首页的搜索结果
//!
//! 设计取舍:
//! - 暂不落 SQLite, 进程内持久已够. 列表 ≤ 8 项, 重启重置代价低.
//! - 保留 `hydrate()` 扩展位, 后续如要持久化, 只需在 storage 增加
//!   `pinned_items` 表, 启动时调一次 `hydrate()` 即可, API 表面不变.

use parking_lot::RwLock;
use std::sync::Arc;

/// 固定项目仓储 —— 用户手动 pin 到首页的项.
pub struct PinRepo {
    ids: Arc<RwLock<Vec<String>>>,
}

impl PinRepo {
    pub fn new() -> Self {
        Self {
            ids: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 从持久化层灌入已有 id 列表 (当前是空操作, 留扩展位).
    pub fn hydrate(&self, ids: Vec<String>) {
        *self.ids.write() = ids;
    }

    /// 按用户添加顺序返回所有 id (最新在前).
    pub fn list(&self) -> Vec<String> {
        self.ids.read().clone()
    }

    /// 添加 id 到列表头部 (最新 pin 排第一); 已存在则去重后再插头部.
    pub fn add(&self, id: String) {
        let mut g = self.ids.write();
        g.retain(|x| x != &id);
        g.insert(0, id);
    }

    /// 从列表移除 id.
    pub fn remove(&self, id: &str) {
        self.ids.write().retain(|x| x != id);
    }
}

impl Default for PinRepo {
    fn default() -> Self {
        Self::new()
    }
}
