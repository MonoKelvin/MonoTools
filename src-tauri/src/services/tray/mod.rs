//! 系统托盘服务
//!
//! 独立的托盘模块，提供托盘图标的创建、菜单管理和事件处理。
//! 菜单项通过 `TrayService` 注册，各业务模块可以自行添加菜单项和处理函数。
//!
//! # 架构原则
//!
//! - 本模块不依赖任何业务模块（不引用 AppState、search_engine 等）
//! - 菜单项和处理函数通过注册机制注入
//! - 删除本模块后，只需移除 modules.rs 中的注册代码

use std::sync::Arc;
use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent, TrayIcon};
use tauri::{AppHandle, Manager, Wry};

/// 菜单项类型
pub enum TrayMenuItemKind {
    Normal {
        id: &'static str,
        label: &'static str,
        enabled: bool,
    },
    Check {
        id: &'static str,
        label: &'static str,
        enabled: bool,
        checked: bool,
    },
    Separator,
    Quit,
}

/// 菜单项描述符
pub struct TrayMenuItem {
    pub kind: TrayMenuItemKind,
    pub on_click: Option<Arc<dyn Fn(&AppHandle<Wry>, &str) + Send + Sync>>,
}

impl TrayMenuItem {
    pub fn normal(
        id: &'static str,
        label: &'static str,
        handler: impl Fn(&AppHandle<Wry>, &str) + Send + Sync + 'static,
    ) -> Self {
        Self {
            kind: TrayMenuItemKind::Normal { id, label, enabled: true },
            on_click: Some(Arc::new(handler)),
        }
    }

    pub fn check(
        id: &'static str,
        label: &'static str,
        checked: bool,
        handler: impl Fn(&AppHandle<Wry>, &str) + Send + Sync + 'static,
    ) -> Self {
        Self {
            kind: TrayMenuItemKind::Check { id, label, enabled: true, checked },
            on_click: Some(Arc::new(handler)),
        }
    }

    pub fn separator() -> Self {
        Self {
            kind: TrayMenuItemKind::Separator,
            on_click: None,
        }
    }

    pub fn quit() -> Self {
        Self {
            kind: TrayMenuItemKind::Quit,
            on_click: None,
        }
    }
}

pub struct TrayService {
    items: Vec<TrayMenuItem>,
    tray_icon: Option<TrayIcon<Wry>>,
}

impl TrayService {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            tray_icon: None,
        }
    }

    /// 注册菜单项
    pub fn register_item(&mut self, item: TrayMenuItem) {
        self.items.push(item);
    }

    /// 创建设置系统托盘
    ///
    /// 根据已注册的菜单项描述符构建实际的托盘菜单。
    pub fn setup(&mut self, app: &tauri::App<Wry>) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(debug_assertions)]
        log::info!("[tray] setup: 构建系统托盘 ({} 个菜单项)", self.items.len());

        let mut menu_items: Vec<Box<dyn IsMenuItem<Wry>>> = Vec::new();
        let mut handlers: Vec<Arc<dyn Fn(&AppHandle<Wry>, &str) + Send + Sync>> = Vec::new();

        for item_desc in &self.items {
            match &item_desc.kind {
                TrayMenuItemKind::Normal { id, label, enabled } => {
                    let item = MenuItem::with_id(app, *id, *label, *enabled, None::<&str>)?;
                    if let Some(h) = &item_desc.on_click {
                        let h = h.clone();
                        let id = id.to_string();
                        handlers.push(Arc::new(move |app, _event_id| {
                            h(app, &id);
                        }));
                    }
                    menu_items.push(Box::new(item));
                }
                TrayMenuItemKind::Check { id, label, enabled, checked } => {
                    let item = CheckMenuItem::with_id(
                        app,
                        *id,
                        *label,
                        *enabled,
                        *checked,
                        None::<&str>,
                    )?;
                    if let Some(h) = &item_desc.on_click {
                        let h = h.clone();
                        let id = id.to_string();
                        handlers.push(Arc::new(move |app, _event_id| {
                            h(app, &id);
                        }));
                    }
                    menu_items.push(Box::new(item));
                }
                TrayMenuItemKind::Separator => {
                    let sep = PredefinedMenuItem::separator(app)?;
                    menu_items.push(Box::new(sep));
                }
                TrayMenuItemKind::Quit => {
                    let quit = PredefinedMenuItem::quit(app, Some("退出"))?;
                    menu_items.push(Box::new(quit));
                }
            }
        }

        let menu_refs: Vec<&dyn IsMenuItem<Wry>> = menu_items
            .iter()
            .map(|b| b.as_ref())
            .collect();

        let tray_menu = Menu::with_items(app, &menu_refs)?;

        let handlers_arc = handlers.clone();
        let tray_icon = TrayIconBuilder::with_id("monotools")
            .icon(app.default_window_icon().unwrap().clone())
            .icon_as_template(true)
            .tooltip("MonoTools")
            .menu(&tray_menu)
            .show_menu_on_left_click(false)
            .on_menu_event(move |app_listener, event| {
                let id = event.id.0.clone();
                for handler in &handlers_arc {
                    handler(app_listener, &id);
                }
            })
            .on_tray_icon_event(|tray, event| {
                handle_tray_click(tray, event);
            })
            .build(app)?;

        self.tray_icon = Some(tray_icon);
        Ok(())
    }
}

impl Default for TrayService {
    fn default() -> Self {
        Self::new()
    }
}

fn handle_tray_click(tray: &TrayIcon<Wry>, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        let app = tray.app_handle();
        if let Some(w) = app.get_webview_window("search") {
            if w.is_visible().unwrap_or(false) {
                let _ = w.hide();
            } else {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
    }
}
