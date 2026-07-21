//! 设置模块托盘菜单项
//!
//! 注意：依赖 AppState（由上层组装），
//! 但逻辑上属于设置模块，因此放在这里统一维护。

use crate::app::state::AppState;
use crate::services::tray::TrayMenuItem;
use std::sync::Arc;
use tauri::Manager;

/// 注册设置相关的托盘菜单项
pub fn register_tray_items<R: tauri::Runtime>(
    tray_service: &mut crate::services::tray::TrayService,
    app: &tauri::App<R>,
) {
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
