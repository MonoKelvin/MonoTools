//! 延迟启动器 - 在系统启动后延迟执行启动项
use crate::error::Result;
use crate::models::StartupItem;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tokio::task::JoinHandle;
use parking_lot::Mutex;

pub struct DelayLauncher {
    tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
}

impl DelayLauncher {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn schedule(&self, item: &StartupItem) -> Result<()> {
        if !item.enabled {
            return Ok(());
        }
        let cmd = item.clone();
        let handle = tokio::spawn(async move {
            if cmd.delay_seconds > 0 {
                sleep(Duration::from_secs(cmd.delay_seconds as u64)).await;
            }
            if let Err(e) = launch_item(&cmd).await {
                log::warn!("延迟启动 {} 失败: {}", cmd.name, e);
            }
        });
        self.tasks.lock().insert(item.id.clone(), handle);
        Ok(())
    }

    pub fn cancel_all(&self) {
        let mut tasks = self.tasks.lock();
        for (_, h) in tasks.drain() {
            h.abort();
        }
    }
}

async fn launch_item(item: &StartupItem) -> Result<()> {
    use crate::platform::windows;
    if item.run_as_admin {
        windows::shell::launch_as_admin(&item.command, &item.args)?;
    } else {
        let _pid = windows::shell::launch(&item.command, &item.args)?;
    }
    Ok(())
}

impl Default for DelayLauncher {
    fn default() -> Self {
        Self::new()
    }
}
