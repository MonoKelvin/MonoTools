//! 全局快捷键服务
use crate::error::Result;
use parking_lot::Mutex;
use tauri::{AppHandle, Runtime, WebviewWindow};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

pub struct HotkeyService {
    pub current: Mutex<Option<String>>,
    pub app: Mutex<Option<AppHandle>>,
}

impl HotkeyService {
    pub fn new() -> Self {
        Self {
            current: Mutex::new(None),
            app: Mutex::new(None),
        }
    }

    pub fn set_app(&self, app: AppHandle) {
        *self.app.lock() = Some(app);
    }

    pub fn mark_registered(&self, hotkey: &str) {
        *self.current.lock() = Some(hotkey.to_string());
    }

    pub async fn register<R: Runtime>(&self, hotkey: &str, app: &AppHandle<R>) -> Result<()> {
        let manager = app.global_shortcut();

        if let Some(current) = self.current.lock().clone() {
            if current == hotkey {
                log::debug!("快捷键 {} 已注册，跳过", hotkey);
                return Ok(());
            }
            let _ = manager.unregister_all();
        }

        let shortcut = crate::platform::windows::parse_hotkey_str(hotkey)?;

        let app_cb = app.clone();
        let _ = manager.on_shortcut(shortcut.clone(), move |_a, _sc, ev| {
            if ev.state() == ShortcutState::Pressed {
                let app = app_cb.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(w) = get_webview_window_clone(&app, "search") {
                        let visible = w.is_visible().unwrap_or(false);
                        if visible {
                            let _ = w.hide();
                        } else {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                });
            }
        });

        match manager.register(shortcut) {
            Ok(_) => {
                log::info!("快捷键注册成功: {}", hotkey);
                *self.current.lock() = Some(hotkey.to_string());
                Ok(())
            }
            Err(e) => {
                log::warn!("快捷键注册失败: {}，尝试其他组合...", e);
                Err(crate::error::AppError::Other(format!("注册快捷键失败: {e}")))
            }
        }
    }

    pub async fn unregister<R: Runtime>(&self, _hotkey: &str, app: &AppHandle<R>) -> Result<()> {
        let manager = app.global_shortcut();
        match manager.unregister_all() {
            Ok(_) => {
                log::info!("快捷键已注销");
                *self.current.lock() = None;
                Ok(())
            }
            Err(e) => {
                log::warn!("注销快捷键失败: {}", e);
                *self.current.lock() = None;
                Ok(())
            }
        }
    }

    pub fn current(&self) -> Option<String> {
        self.current.lock().clone()
    }
}

fn get_webview_window_clone<R: Runtime>(app: &AppHandle<R>, label: &str) -> Option<WebviewWindow<R>> {
    use tauri::Manager;
    Manager::get_webview_window(app, label)
}
