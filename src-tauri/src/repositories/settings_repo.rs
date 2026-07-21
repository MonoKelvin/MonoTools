//! 设置仓储（基于内存模式管理 Settings）
//! 我们使用仓储模式（trait 抽象）便于测试和未来的实现替换
use crate::core::error::Result;
use crate::models::Settings;
use crate::services::tray::TrayMenuItem;
use parking_lot::RwLock;
use std::sync::Arc;
use tauri::Manager;

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

/// 注册设置相关的托盘菜单项
///
/// 注意：需要 AppState 已注册才能使用。
pub fn register_tray_items<R: tauri::Runtime>(
    tray_service: &mut crate::services::tray::TrayService,
    app: &tauri::App<R>,
) {
    use crate::app::state::AppState;

    let state: tauri::State<Arc<AppState>> = app.state();
    let initial_pin = state.settings_repo.get().pin_to_top;

    tray_service.register_item(TrayMenuItem::check(
        "toggle_pin_top",
        "窗口置顶",
        initial_pin,
        |app, _id| {
            let state: tauri::State<Arc<AppState>> = app.state();
            if let Some(window) = app.get_webview_window("search") {
                let cur = state.settings_repo.get().pin_to_top;
                let next = !cur;
                if let Err(e) = state.settings_repo.update(Box::new(move |s| {
                    s.pin_to_top = next;
                })) {
                    log::warn!("更新 pin_to_top 失败: {e}");
                }
                if let Err(e) = window.set_always_on_top(next) {
                    log::warn!("set_always_on_top 失败: {e}");
                }
                if next {
                    let _ = window.show();
                    let _ = window.set_focus();
                } else {
                    let _ = window.hide();
                }
            }
        },
    ));
}
