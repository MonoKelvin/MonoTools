//! 启动项管理服务 - 封装跨多源读取与注册表写入
use crate::error::{AppError, Result};
use crate::models::{NewStartupItem, StartupItem, StartupSource};
use crate::platform::windows;
use crate::repositories::StartupRepo;
use std::sync::Arc;

pub struct StartupManager {
    pub repo: Arc<dyn StartupRepo>,
    pub storage: Option<Arc<crate::services::storage::StorageService>>,
}

impl StartupManager {
    pub fn new(repo: Arc<dyn StartupRepo>) -> Self {
        Self { repo, storage: None }
    }

    pub fn with_storage(
        repo: Arc<dyn StartupRepo>,
        storage: Arc<crate::services::storage::StorageService>,
    ) -> Self {
        Self {
            repo,
            storage: Some(storage),
        }
    }

    pub async fn refresh(&self) -> Result<()> {
        let mut items: Vec<StartupItem> = Vec::new();

        items.extend(windows::registry::read_run_key(false, StartupSource::RegistryRun)?);
        items.extend(windows::registry::read_run_key(true, StartupSource::RegistryRun)?);
        items.extend(windows::startup_folder::read_items()?);

        self.repo.replace(items);
        Ok(())
    }

    pub fn list(&self) -> Vec<StartupItem> {
        self.repo.list()
    }

    pub fn list_enabled(&self) -> Vec<StartupItem> {
        self.repo.list_enabled()
    }

    pub fn find(&self, id: &str) -> Option<StartupItem> {
        self.repo.list().into_iter().find(|i| i.id == id)
    }

    /// 切换启用状态（根据来源分发）
    pub async fn toggle(&self, id: &str, enabled: bool) -> Result<()> {
        let item = self.find(id).ok_or(AppError::StartupItemNotFound(id.into()))?;
        match item.source {
            StartupSource::RegistryRun => {
                // 简化：对 HKCU 启动项添加前缀/删除
                if enabled {
                    windows::registry::write_run_key(false, &item.name, &item.command)?;
                } else {
                    // 禁用 = 添加前缀并禁用
                    windows::registry::write_run_key(false, &format!(".{}_disabled", item.name), "")?;
                }
            }
            StartupSource::StartupFolder => {
                #[cfg(windows)]
                {
                    if enabled {
                        windows::startup_folder::enable_item(&item.command)?;
                    } else {
                        windows::startup_folder::disable_item(&item.command)?;
                    }
                }
            }
            _ => {}
        }

        // 内存中更新
        let mut items = self.repo.list();
        if let Some(i) = items.iter_mut().find(|i| i.id == id) {
            i.enabled = enabled;
        }
        self.repo.replace(items);
        Ok(())
    }

    pub async fn add(&self, item: NewStartupItem) -> Result<String> {
        let startup: StartupItem = item.into();
        windows::registry::write_run_key(false, &startup.name, &startup.command)?;
        self.repo.add(startup.clone())?;
        Ok(startup.id)
    }

    pub async fn remove(&self, id: &str) -> Result<()> {
        let item = self.find(id).ok_or(AppError::StartupItemNotFound(id.into()))?;
        match item.source {
            StartupSource::RegistryRun => {
                windows::registry::delete_run_value(false, &item.name)?;
            }
            _ => {}
        }
        self.repo.remove(id)?;
        Ok(())
    }
}
