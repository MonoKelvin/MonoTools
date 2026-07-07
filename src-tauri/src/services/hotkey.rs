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

    /// 注册全局快捷键
    pub async fn register<R: Runtime>(&self, hotkey: &str, app: &AppHandle<R>) -> Result<()> {
        Self::static_register(app, hotkey)?;
        *self.current.lock() = Some(hotkey.to_string());
        Ok(())
    }

    pub async fn unregister<R: Runtime>(&self, hotkey: &str, app: &AppHandle<R>) -> Result<()> {
        let manager = app.global_shortcut();
        manager
            .unregister(hotkey)
            .map_err(|e| crate::error::AppError::Other(format!("注销快捷键失败: {e}")))?;
        *self.current.lock() = None;
        Ok(())
    }

    pub fn current(&self) -> Option<String> {
        self.current.lock().clone()
    }

    pub fn static_register<R: Runtime>(app: &AppHandle<R>, hotkey: &str) -> Result<()> {
        let manager = app.global_shortcut();
        let _ = manager.unregister_all();

        let app_cb = app.clone();
        manager
            .on_shortcut(hotkey, move |_a, _sc, ev| {
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
            })
            .map_err(|e| crate::error::AppError::Other(format!("注册回调失败: {e}")))?;

        let shortcut = crate::platform::windows::parse_hotkey_str(hotkey)?;
        manager
            .register(shortcut)
            .map_err(|e| crate::error::AppError::Other(format!("注册快捷键失败: {e}")))?;
        Ok(())
    }
}

fn get_webview_window_clone<R: Runtime>(app: &AppHandle<R>, label: &str) -> Option<WebviewWindow<R>> {
    use tauri::Manager;
    Manager::get_webview_window(app, label)
}
