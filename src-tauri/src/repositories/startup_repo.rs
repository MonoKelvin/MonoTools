use crate::error::Result;
use crate::models::{NewStartupItem, StartupItem};
use parking_lot::RwLock;
use std::sync::Arc;

/// 启动项仓储（trait 抽象）
pub trait StartupRepo: Send + Sync {
    fn list(&self) -> Vec<StartupItem>;
    fn list_enabled(&self) -> Vec<StartupItem>;
    fn add(&self, item: StartupItem) -> Result<()>;
    fn update(&self, id: &str, item: &StartupItem) -> Result<()>;
    fn remove(&self, id: &str) -> Result<()>;
    fn replace(&self, items: Vec<StartupItem>);
}

pub struct InMemoryStartupRepo {
    items: Arc<RwLock<Vec<StartupItem>>>,
}

impl InMemoryStartupRepo {
    pub fn new() -> Self {
        Self {
            items: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Default for InMemoryStartupRepo {
    fn default() -> Self {
        Self::new()
    }
}

impl StartupRepo for InMemoryStartupRepo {
    fn list(&self) -> Vec<StartupItem> {
        self.items.read().clone()
    }

    fn list_enabled(&self) -> Vec<StartupItem> {
        self.items
            .read()
            .iter()
            .filter(|i| i.enabled)
            .cloned()
            .collect()
    }

    fn add(&self, item: StartupItem) -> Result<()> {
        self.items.write().push(item);
        Ok(())
    }

    fn update(&self, id: &str, item: &StartupItem) -> Result<()> {
        let mut g = self.items.write();
        if let Some(slot) = g.iter_mut().find(|i| i.id == id) {
            *slot = item.clone();
        }
        Ok(())
    }

    fn remove(&self, id: &str) -> Result<()> {
        self.items.write().retain(|i| i.id != id);
        Ok(())
    }

    fn replace(&self, items: Vec<StartupItem>) {
        *self.items.write() = items;
    }
}

// ============ NewStartupItem helper ============

impl From<NewStartupItem> for StartupItem {
    fn from(n: NewStartupItem) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: n.name,
            command: n.command,
            args: n.args,
            working_dir: n.working_dir,
            delay_seconds: n.delay_seconds,
            run_as_admin: n.run_as_admin,
            enabled: true,
            source: crate::models::StartupSource::Custom,
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}
