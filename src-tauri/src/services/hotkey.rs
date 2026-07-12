//! 全局快捷键服务
//!
//! 注册策略:
//! 1. 优先使用 tauri-plugin-global-shortcut (底层 RegisterHotKey)
//! 2. 若失败 (如 Alt+Space 被 Windows 保留), 回退到低级键盘钩子 (WH_KEYBOARD_LL)
use crate::error::Result;
use crate::platform::windows::hotkey::{hotkey_to_vk, LowLevelHotkeyHook};
use parking_lot::Mutex;
use tauri::{AppHandle, Runtime, WebviewWindow};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

pub struct HotkeyService {
    pub current: Mutex<Option<String>>,
    pub app: Mutex<Option<AppHandle>>,
    /// 低级键盘钩子 (RegisterHotKey 失败时的回退, 如 Alt+Space)
    ll_hook: Mutex<Option<LowLevelHotkeyHook>>,
}

impl HotkeyService {
    pub fn new() -> Self {
        Self {
            current: Mutex::new(None),
            app: Mutex::new(None),
            ll_hook: Mutex::new(None),
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

        // 先清理旧注册 (tauri global-shortcut + LL hook)
        let was_same = self
            .current
            .lock()
            .as_ref()
            .map(|c| c == hotkey)
            .unwrap_or(false);
        let ll_active = self.ll_hook.lock().is_some();
        if was_same && !ll_active {
            log::debug!("快捷键 {} 已注册 (tauri)，跳过", hotkey);
            return Ok(());
        }
        let _ = manager.unregister_all();
        // 停止旧的 LL hook
        if let Some(mut old) = self.ll_hook.lock().take() {
            old.stop();
        }

        let shortcut = crate::platform::windows::parse_hotkey_str(hotkey)?;

        let app_cb = app.clone();
        let _ = manager.on_shortcut(shortcut.clone(), move |_a, _sc, ev| {
            if ev.state() == ShortcutState::Pressed {
                let app = app_cb.clone();
                tauri::async_runtime::spawn(async move {
                    toggle_search_window(&app);
                });
            }
        });

        match manager.register(shortcut) {
            Ok(_) => {
                log::info!("快捷键注册成功 (tauri global-shortcut): {}", hotkey);
                *self.current.lock() = Some(hotkey.to_string());
                Ok(())
            }
            Err(e) => {
                log::warn!(
                    "tauri global-shortcut 注册失败: {} — 尝试低级键盘钩子 (WH_KEYBOARD_LL)...",
                    e
                );
                // 回退: 低级键盘钩子 (可拦截 Alt+Space 等 Windows 保留组合键)
                let (vk, needs_alt) = hotkey_to_vk(hotkey)?;
                let app_cb2 = app.clone();
                let hook = LowLevelHotkeyHook::start(vk, needs_alt, move || {
                    let app = app_cb2.clone();
                    tauri::async_runtime::spawn(async move {
                        toggle_search_window(&app);
                    });
                })?;
                *self.ll_hook.lock() = Some(hook);
                log::info!("快捷键注册成功 (WH_KEYBOARD_LL 回退): {}", hotkey);
                *self.current.lock() = Some(hotkey.to_string());
                Ok(())
            }
        }
    }

    pub async fn unregister<R: Runtime>(&self, _hotkey: &str, app: &AppHandle<R>) -> Result<()> {
        let manager = app.global_shortcut();
        match manager.unregister_all() {
            Ok(_) => log::info!("快捷键已注销 (tauri)"),
            Err(e) => log::warn!("注销快捷键失败 (tauri): {}", e),
        }
        // 停止 LL hook
        if let Some(mut hook) = self.ll_hook.lock().take() {
            hook.stop();
            log::info!("LL hook 已停止");
        }
        *self.current.lock() = None;
        Ok(())
    }

    pub fn current(&self) -> Option<String> {
        self.current.lock().clone()
    }
}

/// 切换 search 窗口的显示/隐藏状态 (Spotlight 体验)
fn toggle_search_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = get_webview_window_clone(app, "search") {
        let visible = w.is_visible().unwrap_or(false);
        if visible {
            let _ = w.hide();
        } else {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
}

fn get_webview_window_clone<R: Runtime>(app: &AppHandle<R>, label: &str) -> Option<WebviewWindow<R>> {
    use tauri::Manager;
    Manager::get_webview_window(app, label)
}
