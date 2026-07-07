//! 设置仓储（基于内存模式管理 Settings）
//! 我们使用仓储模式（trait 抽象）便于测试和未来的实现替换
use crate::error::Result;
use crate::models::Settings;
use parking_lot::RwLock;
use std::sync::Arc;

pub trait SettingsRepo: Send + Sync {
    fn get(&self) -> Settings;
    fn save(&self, settings: Settings) -> Result<()>;
    fn update(&self, f: Box<dyn FnOnce(&mut Settings) + Send + '_>) -> Result<Settings>;
}

/// 简单的内存版实现（完整持久化在 StorageService 中）
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
