//! 启动项搜索 - 来自 StartupRepo + 注册表实时刷新
use crate::models::{SearchAction, SearchCategory, SearchResult};
use crate::repositories::StartupRepo;
use std::sync::Arc;

pub struct StartupSearchService {
    repo: Arc<dyn StartupRepo>,
    cached: parking_lot::RwLock<Vec<crate::models::StartupItem>>,
}

impl StartupSearchService {
    pub async fn new(repo: Arc<dyn StartupRepo>) -> crate::error::Result<Self> {
        let svc = Self {
            repo,
            cached: parking_lot::RwLock::new(Vec::new()),
        };
        svc.refresh().await?;
        Ok(svc)
    }

    pub async fn refresh(&self) -> crate::error::Result<()> {
        self.repo.replace(Vec::new());
        let mut items: Vec<crate::models::StartupItem> = Vec::new();

        if let Ok(hkcu) = crate::platform::windows::registry::read_run_key(
            false,
            crate::models::StartupSource::RegistryRun,
        ) {
            items.extend(hkcu);
        }
        if let Ok(hklm) = crate::platform::windows::registry::read_run_key(
            true,
            crate::models::StartupSource::RegistryRun,
        ) {
            items.extend(hklm);
        }
        if let Ok(folder) = crate::platform::windows::startup_folder::read_items() {
            items.extend(folder);
        }

        self.repo.replace(items.clone());
        *self.cached.write() = items;
        Ok(())
    }

    pub fn search(&self, query: &str, limit: u32) -> Vec<SearchResult> {
        let items = self.cached.read();
        let q = query.to_lowercase();
        if q.is_empty() {
            return vec![];
        }

        let mut results: Vec<SearchResult> = Vec::new();
        for item in items.iter() {
            let name_l = item.name.to_lowercase();
            let cmd_l = item.command.to_lowercase();
            let mut score = 0.0;
            if name_l.contains(&q) {
                score += 80.0;
            }
            if cmd_l.contains(&q) {
                score += 30.0;
            }
            if score == 0.0 {
                continue;
            }
            if !item.enabled {
                score -= 5.0;
            }

            results.push(SearchResult {
                id: item.id.clone(),
                title: item.name.clone(),
                subtitle: item.command.clone(),
                icon: None,
                category: SearchCategory::Startup,
                action: SearchAction::Launch(item.command.clone()),
                score,
            });
            if results.len() >= limit as usize {
                break;
            }
        }
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    pub fn list(&self) -> Vec<crate::models::StartupItem> {
        self.cached.read().clone()
    }
}
