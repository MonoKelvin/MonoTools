//! 设置仓储 —— 抽象存储层
//!
//! 提供 SettingsRepo trait，便于不同存储实现（内存、文件、SQLite等）。
//! 业务模块应依赖 trait 而非具体实现，便于测试和替换。

use crate::core::error::Result;
use crate::core::settings::Settings;
use parking_lot::RwLock;
use std::sync::Arc;

pub trait SettingsRepo: Send + Sync {
    fn get(&self) -> Settings;
    fn save(&self, settings: Settings) -> Result<()>;
    fn update(&self, f: Box<dyn FnOnce(&mut Settings) + Send + '_>) -> Result<Settings>;
}

/// 内存版实现（完整持久化可在 StorageService 中实现）
pub struct InMemorySettingsRepo {
    inner: Arc<RwLock<Settings>>,
}

impl InMemorySettingsRepo {
    pub fn new(initial: Settings) -> Self {
        Self {
            inner: Arc::new(RwLock::new(initial)),
        }
    }
}

impl SettingsRepo for InMemorySettingsRepo {
    fn get(&self) -> Settings {
        self.inner.read().clone()
    }

    fn save(&self, settings: Settings) -> Result<()> {
        *self.inner.write() = settings;
        Ok(())
    }

    fn update(&self, f: Box<dyn FnOnce(&mut Settings) + Send + '_>) -> Result<Settings> {
        let mut g = self.inner.write();
        f(&mut g);
        Ok(g.clone())
    }
}
