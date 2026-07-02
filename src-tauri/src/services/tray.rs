use tauri::AppHandle;
use tauri::Manager;
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton};
use tauri::menu::{Menu, MenuItem};

pub fn setup_tray(app: &tauri::App) -> anyhow::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示搜索框", true, None::<&str>)?;
    let workspaces = MenuItem::with_id(app, "workspaces", "工作区管理", true, None::<&str>)?;
    let plugins = MenuItem::with_id(app, "plugins", "插件管理", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let separator1 = MenuItem::with_id(app, "separator1", "", true, None::<&str>)?;
    let about = MenuItem::with_id(app, "about", "关于 MonoTools", true, None::<&str>)?;
    let separator2 = MenuItem::with_id(app, "separator2", "", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[
        &show,
        &workspaces,
        &plugins,
        &settings,
        &separator1,
        &about,
        &separator2,
        &quit,
    ])?;

    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .on_menu_event(|app_handle, event| {
            match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "workspaces" => {
                    // TODO: 打开工作区管理
                }
                "plugins" => {
                    // TODO: 打开插件管理
                }
                "settings" => {
                    // TODO: 打开设置面板
                }
                "about" => {
                    // TODO: 显示关于对话框
                }
                "quit" => {
                    app_handle.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = event
            {
                let app_handle = tray.app_handle();
                if let Some(window) = app_handle.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .tooltip("MonoTools")
        .build(app)?;

    Ok(())
}
