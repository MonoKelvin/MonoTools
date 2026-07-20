//! 全局快捷键服务
//!
//! 注册策略:
//! 1. 优先使用 tauri-plugin-global-shortcut (底层 RegisterHotKey)
//! 2. 若失败 (如 Alt+Space 被 Windows 保留), 回退到低级键盘钩子 (WH_KEYBOARD_LL)
use crate::core::error::Result;
use crate::platform::windows::hotkey::{hotkey_to_vk, LowLevelHotkeyHook};
use parking_lot::Mutex;
use std::sync::Arc;
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

/// 热键服务 IPC 命令
///
/// 独立模块设计：热键相关的 IPC 命令定义在此模块中，
/// 通过 core_ipc_commands! 宏注册到全局。
pub mod ipc {
    use crate::app::state::AppState;
    use std::sync::Arc;
    use tauri::{AppHandle, State};

    #[tauri::command]
    pub async fn register_hotkey_cmd(
        app: AppHandle,
        state: State<'_, Arc<AppState>>,
        hotkey: String,
    ) -> Result<String, String> {
        state
            .hotkey
            .register(&hotkey, &app)
            .await
            .map_err(|e| e.to_string())?;
        Ok(hotkey)
    }

    #[tauri::command]
    pub async fn unregister_hotkey(
        app: AppHandle,
        state: State<'_, Arc<AppState>>,
        hotkey: String,
    ) -> Result<(), String> {
        state
            .hotkey
            .unregister(&hotkey, &app)
            .await
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn get_current_hotkey(state: State<'_, Arc<AppState>>) -> Result<String, String> {
        Ok(state.hotkey.current().unwrap_or_default())
    }
}

/// 热键服务初始化
///
/// 从设置中读取快捷键并注册。
/// 原本在 builder.rs 的 setup 中，现在抽离到 hotkey 模块。
pub fn init_hotkey_service(
    app: &AppHandle,
    hotkey: Arc<HotkeyService>,
    initial_hotkey: String,
) {
    let app_handle = app.clone();
    let hotkey_clone = hotkey.clone();

    tauri::async_runtime::spawn(async move {
        #[cfg(debug_assertions)]
        log::info!("[hotkey] 后台注册 hotkey: {}", initial_hotkey);

        if let Err(e) = hotkey_clone
            .register(&initial_hotkey, &app_handle)
            .await
        {
            log::warn!("注册默认快捷键失败: {}，请检查是否被其他程序占用", e);
            log::info!("尝试重新注册...");
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = hotkey_clone
                .register(&initial_hotkey, &app_handle)
                .await;
        } else {
            #[cfg(debug_assertions)]
            log::info!("[hotkey] hotkey 注册成功: {}", initial_hotkey);
        }
    });
}
