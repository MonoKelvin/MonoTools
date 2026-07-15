use crate::core::command::command_custom::CustomCommand;
use crate::core::error::Result;
use parking_lot::RwLock;
use std::sync::Arc;

pub trait CommandRepo: Send + Sync {
    fn list(&self) -> Vec<CustomCommand>;
    fn list_enabled(&self) -> Vec<CustomCommand>;
    fn get(&self, id: &str) -> Option<CustomCommand>;
    fn add(&self, cmd: CustomCommand) -> Result<()>;
    fn remove(&self, id: &str) -> Result<()>;
    fn update(&self, cmd: CustomCommand) -> Result<()>;
    fn record_used(&self, id: &str, ts: i64) -> Result<()>;
}

pub struct InMemoryCommandRepo {
    items: Arc<RwLock<Vec<CustomCommand>>>,
}

impl InMemoryCommandRepo {
    pub fn new() -> Self {
        Self {
            items: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Default for InMemoryCommandRepo {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRepo for InMemoryCommandRepo {
    fn list(&self) -> Vec<CustomCommand> {
        self.items.read().clone()
    }

    fn list_enabled(&self) -> Vec<CustomCommand> {
        self.items
            .read()
            .iter()
            .filter(|c| c.enabled)
            .cloned()
            .collect()
    }

    fn get(&self, id: &str) -> Option<CustomCommand> {
        self.items.read().iter().find(|c| c.id == id).cloned()
    }

    fn add(&self, cmd: CustomCommand) -> Result<()> {
        self.items.write().push(cmd);
        Ok(())
    }

    fn remove(&self, id: &str) -> Result<()> {
        self.items.write().retain(|c| c.id != id);
        Ok(())
    }

    fn update(&self, cmd: CustomCommand) -> Result<()> {
        let mut g = self.items.write();
        if let Some(slot) = g.iter_mut().find(|c| c.id == cmd.id) {
            *slot = cmd;
        }
        Ok(())
    }

    fn record_used(&self, id: &str, ts: i64) -> Result<()> {
        let mut g = self.items.write();
        if let Some(slot) = g.iter_mut().find(|c| c.id == id) {
            slot.last_used_at = Some(ts);
        }
        Ok(())
    }
}
