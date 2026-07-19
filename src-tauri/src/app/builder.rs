//! Tauri 应用构建器
//!
//! 负责 Tauri GUI 应用的初始化、插件注册、窗口设置、系统托盘等。
//! 这部分代码仅在 GUI 模式下编译使用，CLI 模式不依赖。

use crate::app::state::AppState;
use crate::models::Settings;
use crate::repositories::*;
use crate::search_engine::app_search::AppSearchEngine;
use crate::search_engine::command_search::CommandSearchEngine;
use crate::search_engine::file_search::FileSearchEngine;
use crate::search_engine::SearchEngine;
use crate::services::hotkey::HotkeyService;
use crate::services::window::WindowService;
use crate::services::window_monitor::WindowMonitorService;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::menu::{CheckMenuItem, Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, WindowEvent};

pub fn run() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info,monotools_lib=debug");
    }
    let _ = env_logger::try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            log::info!("检测到新实例启动，激活已有窗口");
            if let Some(window) = app.get_webview_window("search") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .on_window_event(|window, event| {
            if let WindowEvent::Focused(false) = event {
                if window.label() == "search" {
                    let app_handle = window.app_handle();
                    if let Some(state) = app_handle.try_state::<Arc<AppState>>() {
                        if let Ok(is_dragging) = state.is_dragging.lock() {
                            if *is_dragging {
                                return;
                            }
                        }
                        let settings = state.settings_repo.get();
                        if settings.pin_to_top {
                            return;
                        }
                    }
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            #[cfg(debug_assertions)]
            log::info!("[boot] setup: enter");
            let app_handle = app.handle().clone();

            #[cfg(windows)]
            {
                if let Some(window) = app.get_webview_window("search") {
                    let (major, minor) = unsafe {
                        let os_version =
                            windows_sys::Win32::System::SystemInformation::GetVersion();
                        ((os_version >> 8) & 0xFF, (os_version >> 16) & 0xFF)
                    };

                    if major >= 10 && (major > 10 || minor >= 22000) {
                        match window_vibrancy::apply_mica(&window, None) {
                            Ok(_) => {
                                log::info!("[effects] window-vibrancy mica 效果已应用 (Win11+)");
                            }
                            Err(e) => {
                                log::warn!("[effects] window-vibrancy apply_mica 失败: {e}, 回退到 CSS backdrop-filter");
                            }
                        }
                    } else {
                        log::info!("[effects] Win10 环境，使用 CSS backdrop-filter 实现毛玻璃效果");
                    }
                }
            }

            let default_settings = Settings::default();
            let initial_pin = default_settings.pin_to_top;

            #[cfg(debug_assertions)]
            log::info!("[boot] setup: 构建系统托盘");
            let show_item = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
            let hide_item = MenuItem::with_id(app, "hide", "隐藏窗口", true, None::<&str>)?;
            let pin_item = CheckMenuItem::with_id(
                app,
                "toggle_pin_top",
                "窗口置顶",
                true,
                initial_pin,
                None::<&str>,
            )?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let tray_menu =
                Menu::with_items(app, &[&show_item, &hide_item, &pin_item, &quit_item])?;

            #[cfg(debug_assertions)]
            log::info!("[boot] setup: TrayIconBuilder::build");
            let _tray = TrayIconBuilder::with_id("monotools")
                .icon(app.default_window_icon().unwrap().clone())
                .icon_as_template(true)
                .tooltip("MonoTools")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event({
                    let pin_item = pin_item.clone();
                    move |app_listener, event| {
                        let id = event.id.as_ref();
                        if let Some(window) = app_listener.get_webview_window("search") {
                            if id == "show" {
                                let _ = window.show();
                                let _ = window.set_focus();
                            } else if id == "hide" {
                                let _ = window.hide();
                            } else if id == "toggle_pin_top" {
                                let state: tauri::State<Arc<AppState>> = app_listener.state();
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
                                let _ = pin_item.set_checked(next);
                            } else if id == "quit" {
                                app_listener.exit(0);
                            }
                        }
                    }
                })
                .on_tray_icon_event(move |tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("search") {
                            // 检查前端是否已初始化完成. 未完成时不显示窗口, 避免透明窗口闪烁.
                            let is_ready = app
                                .try_state::<Arc<AppState>>()
                                .map(|s| s.frontend_initialized.load(std::sync::atomic::Ordering::Acquire))
                                .unwrap_or(false);
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else if is_ready {
                                let _ = w.show();
                                let _ = w.set_focus();
                            } else {
                                // 前端未就绪, 记录日志但不显示窗口
                                log::debug!("[tray] 前端未初始化完成, 忽略托盘点击");
                            }
                        }
                    }
                })
                .build(app)?;

            #[cfg(debug_assertions)]
            log::info!("[boot] tauri Builder::setup 入口");

            #[cfg(debug_assertions)]
            log::info!("[boot] setup: 同步构造 AppState");
            let settings_repo = Arc::new(InMemorySettingsRepo::new(Settings::default()));
            let command_repo: Arc<dyn crate::repositories::CommandRepo> =
                Arc::new(InMemoryCommandRepo::new());
            let stats_repo = Arc::new(StatsRepo::new());
            let pin_repo = Arc::new(PinRepo::new());

            let app_search = Arc::new(AppSearchEngine::new_empty(settings_repo.clone()));
            let command_search = Arc::new(CommandSearchEngine::new(command_repo.clone()));

            let settings = settings_repo.get();
            let file_roots = settings.file_search_roots.clone();

            let explicit_drives = !settings.file_search_drives.is_empty();
            let selected_roots = if explicit_drives {
                let mut roots = file_roots.clone();
                for drive in &settings.file_search_drives {
                    roots.push(PathBuf::from(format!("{}:\\", drive)));
                }
                roots
            } else {
                Vec::new()
            };

            #[cfg(debug_assertions)]
            log::info!("[boot] setup: 构造 FileSearchEngine");
            let file_search = Arc::new(FileSearchEngine::new(selected_roots).unwrap());
            #[cfg(debug_assertions)]
            log::info!("[boot] setup: FileSearchEngine 构造完成");

            let search_engine = Arc::new(SearchEngine::new(
                app_search.clone(),
                file_search.clone(),
                command_search.clone(),
            ));

            let hotkey = Arc::new(HotkeyService::new());
            let window_inner = WindowService::new(app_handle.clone());
            let window = Arc::new(window_inner);

            // 启动窗口监控服务
            let mut window_monitor = WindowMonitorService::new();
            window_monitor.start(app_handle.clone());
            let window_monitor = Arc::new(Mutex::new(window_monitor));

            let state: Arc<AppState> = Arc::new(AppState {
                app: app_handle.clone(),
                settings_repo,
                command_repo,
                stats_repo,
                pin_repo,
                app_search,
                command_search,
                file_search,
                search_engine,
                hotkey,
                window,
                window_monitor,
                is_dragging: Arc::new(Mutex::new(false)),
                frontend_initialized: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            });
            #[cfg(debug_assertions)]
            log::info!("[boot] setup: AppState 就绪");

            {
                let app_search_for_refresh = state.app_search.clone();
                let app_handle_for_apps = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    #[cfg(debug_assertions)]
                    log::info!("[boot] 后台应用索引刷新任务已 spawn");
                    let _ = app_handle_for_apps.emit(
                        "index_progress",
                        serde_json::json!({
                            "status": "building",
                            "message": "正在加载应用列表...",
                            "phase": "apps",
                        }),
                    );
                    let app_handle_for_progress = app_handle_for_apps.clone();
                    let result = app_search_for_refresh
                        .refresh_index_incremental(move |count, phase| {
                            let phase_label = match phase {
                                "common_start_menu" => "公共开始菜单",
                                "user_start_menu" => "用户开始菜单",
                                "desktop" => "桌面",
                                _ => phase,
                            };
                            let _ = app_handle_for_progress.emit(
                                "index_progress",
                                serde_json::json!({
                                    "status": "building",
                                    "message": format!("已加载 {} 个应用（{}）", count, phase_label),
                                    "phase": "apps",
                                    "apps": count,
                                    "apps_phase": phase,
                                }),
                            );
                        })
                        .await;
                    match result {
                        Ok(()) => {
                            let total = app_search_for_refresh.total();
                            log::info!("应用索引刷新完成: {} 个应用", total);
                            let _ = app_handle_for_apps.emit(
                                "index_progress",
                                serde_json::json!({
                                    "status": "completed",
                                    "message": format!("已加载 {} 个应用", total),
                                    "phase": "apps",
                                    "apps": total,
                                }),
                            );
                        }
                        Err(e) => {
                            log::warn!("应用索引刷新失败: {}", e);
                            let _ = app_handle_for_apps.emit(
                                "index_progress",
                                serde_json::json!({
                                    "status": "error",
                                    "message": format!("应用列表加载失败: {}", e),
                                    "phase": "apps",
                                }),
                            );
                        }
                    }
                });
            }

            app.manage(state.clone());

            // 确保窗口在 frontend_ready 之前保持隐藏.
            // Tauri 在 Windows 上创建窗口时可能短暂显示, 即使 visible: false.
            // 这里显式隐藏, 防止启动时出现透明窗口闪烁.
            if let Some(w) = app_handle.get_webview_window("search") {
                let _ = w.hide();
            }

            #[cfg(debug_assertions)]
            log::info!("[boot] 同步 pin_to_top -> 窗口");
            if let Some(w) = app_handle.get_webview_window("search") {
                let _ = w.set_always_on_top(state.settings_repo.get().pin_to_top);
            }

            let app_handle_for_setup = app.handle().clone();
            let initial_hotkey = state.settings_repo.get().hotkey.clone();
            tauri::async_runtime::spawn(async move {
                #[cfg(debug_assertions)]
                log::info!("[hotkey] 后台注册 hotkey: {}", initial_hotkey);
                if let Err(e) = state
                    .hotkey
                    .register(&initial_hotkey, &app_handle_for_setup)
                    .await
                {
                    log::warn!("注册默认快捷键失败: {}，请检查是否被其他程序占用", e);
                    log::info!("尝试重新注册...");
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    let _ = state
                        .hotkey
                        .register(&initial_hotkey, &app_handle_for_setup)
                        .await;
                } else {
                    #[cfg(debug_assertions)]
                    log::info!("[hotkey] hotkey 注册成功: {}", initial_hotkey);
                }
            });

            #[cfg(debug_assertions)]
            log::info!("[boot] setup 即将 return Ok(())");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crate::app::ipc::search_cmd,
            crate::app::ipc::search_more_cmd,
            crate::app::ipc::execute_result,
            crate::app::ipc::show_window,
            crate::app::ipc::hide_window,
            crate::app::ipc::toggle_window,
            crate::app::ipc::register_hotkey_cmd,
            crate::app::ipc::unregister_hotkey,
            crate::app::ipc::get_current_hotkey,
            crate::app::ipc::list_commands,
            crate::app::ipc::add_command,
            crate::app::ipc::remove_command,
            crate::app::ipc::run_command,
            crate::app::ipc::get_setting,
            crate::app::ipc::set_setting,
            crate::app::ipc::get_all_settings,
            crate::app::ipc::set_all_settings,
            crate::app::ipc::get_appearance,
            crate::app::ipc::set_appearance,
            crate::app::ipc::get_pin_top,
            crate::app::ipc::set_pin_top,
            crate::app::ipc::set_window_height,
            crate::app::ipc::start_dragging,
            crate::app::ipc::set_dragging,
            crate::app::ipc::quit_app,
            crate::app::ipc::build_file_index,
            crate::app::ipc::get_index_status,
            crate::app::ipc::frontend_ready,
            crate::app::ipc::list_command_specs,
            crate::app::ipc::dispatch_command,
            crate::app::ipc::get_app_icon,
            crate::app::ipc::get_app_icons_batch,
            crate::app::ipc::list_pinned,
            crate::app::ipc::pin_item,
            crate::app::ipc::unpin_item,
            crate::app::ipc::open_file_location,
            crate::app::ipc::show_file_properties,
            crate::app::ipc::delete_file_to_recycle_bin,
            crate::app::ipc::get_system_theme,
            crate::app::ipc::set_follow_system_theme,
            crate::app::ipc::get_window_monitor_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
