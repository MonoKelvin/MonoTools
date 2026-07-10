//! Tauri 应用构建与命令入口

pub mod app {
    use crate::engines::app_search::AppSearchEngine;
    use crate::engines::command_search::CommandSearchEngine;
    use crate::engines::file_search::FileSearchEngine;
    use crate::models::Settings;
    use crate::repositories::*;
    use crate::services::app_state::AppState;
    use crate::services::hotkey::HotkeyService;
    use crate::services::search::SearchEngine;
    use crate::services::window::WindowService;
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
                // 失焦自动隐藏（Spotlight 体验）
                if let WindowEvent::Focused(false) = event {
                    if window.label() == "search" {
                        let app_handle = window.app_handle();
                        if let Some(state) = app_handle.try_state::<Arc<AppState>>() {
                            if let Ok(is_dragging) = state.is_dragging.lock() {
                                if *is_dragging {
                                    return;
                                }
                            }
                        }
                        let _ = window.hide();
                    }
                }
            })
            .setup(|app| {
                let app_handle = app.handle().clone();

                // 初始默认行为依赖于设置；先构造 settings repo 读取 pin_to_top
                let default_settings = Settings::default();
                let initial_pin = default_settings.pin_to_top;

                // 构建系统托盘图标 + 菜单（含"窗口置顶"开关）
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
                                    // 同步菜单复选框状态
                                    let _ = pin_item.set_checked(next);
                                } else if id == "quit" {
                                    app_listener.exit(0);
                                }
                            }
                        }
                    })
                    .on_tray_icon_event(|tray, event| {
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
                    })
                    .build(app)?;

                let state: Arc<AppState> = tauri::async_runtime::block_on(async {
                    let settings_repo = Arc::new(InMemorySettingsRepo::new(Settings::default()));
                    let command_repo: Arc<dyn crate::repositories::CommandRepo> =
                        Arc::new(InMemoryCommandRepo::new());
                    let stats_repo = Arc::new(StatsRepo::new());

                    let app_search = Arc::new(AppSearchEngine::new(settings_repo.clone()).await?);
                    app_search.refresh_index().await?;
                    let command_search = Arc::new(CommandSearchEngine::new(command_repo.clone()));
                    let settings = settings_repo.get();
                    let mut file_roots = settings.file_search_roots.clone();

                    if !settings.file_search_drives.is_empty() {
                        for drive in &settings.file_search_drives {
                            file_roots.push(PathBuf::from(format!("{}:\\", drive)));
                        }
                    }

                    let file_search = Arc::new(FileSearchEngine::new(file_roots.clone()).unwrap());

                    let search_engine = Arc::new(SearchEngine::new(
                        app_search.clone(),
                        file_search.clone(),
                        command_search.clone(),
                    ));

                    let hotkey = Arc::new(HotkeyService::new());
                    let app_for_window = app_handle.clone();
                    let window_inner = WindowService::new(app_for_window);
                    let window = Arc::new(window_inner);

                    Ok::<_, crate::error::AppError>(Arc::new(AppState {
                        app: app_handle.clone(),
                        settings_repo,
                        command_repo,
                        stats_repo,
                        app_search,
                        command_search,
                        file_search,
                        search_engine,
                        hotkey,
                        window,
                        is_dragging: Arc::new(Mutex::new(false)),
                    }))
                })?;

                let file_search_clone = state.file_search.clone();
                let app_handle_for_index = app.handle().clone();
                let file_roots_clone = state.settings_repo.get().file_search_roots.clone();
                if !file_roots_clone.is_empty() {
                    tauri::async_runtime::spawn(async move {
                        log::info!("后台索引构建任务启动...");
                        let _ = app_handle_for_index.emit(
                            "index_progress",
                            serde_json::json!({
                                "status": "building",
                                "message": "正在构建文件索引...",
                            }),
                        );
                        let start = std::time::Instant::now();
                        if let Err(e) = file_search_clone.build_index().await {
                            log::error!("后台索引构建失败: {}", e);
                            let _ = app_handle_for_index.emit(
                                "index_progress",
                                serde_json::json!({
                                    "status": "error",
                                    "message": format!("索引构建失败: {}", e),
                                }),
                            );
                        } else {
                            log::info!("后台索引构建完成，耗时 {:?}", start.elapsed());
                            let _ = app_handle_for_index.emit(
                                "index_progress",
                                serde_json::json!({
                                    "status": "completed",
                                    "message": "索引构建完成",
                                }),
                            );
                        }

                        use crate::engines::start_update_loop;
                        let fs_clone = file_search_clone.clone();
                        start_update_loop(
                            move || fs_clone.update_index(),
                            std::time::Duration::from_secs(120),
                        );
                    });
                }

                app.manage(state.clone());

                // 同步初始置顶状态到窗口
                if let Some(w) = app_handle.get_webview_window("search") {
                    let _ = w.set_always_on_top(state.settings_repo.get().pin_to_top);
                }

                // 注册快捷键（非阻塞）
                let app_handle_for_setup = app.handle().clone();
                let initial_hotkey = state.settings_repo.get().hotkey.clone();
                tauri::async_runtime::spawn(async move {
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
                    }
                });

                Ok(())
            })
            .invoke_handler(tauri::generate_handler![
                crate::commands::search_cmd,
                crate::commands::execute_result,
                crate::commands::show_window,
                crate::commands::hide_window,
                crate::commands::toggle_window,
                crate::commands::register_hotkey_cmd,
                crate::commands::unregister_hotkey,
                crate::commands::get_current_hotkey,
                crate::commands::list_commands,
                crate::commands::add_command,
                crate::commands::remove_command,
                crate::commands::run_command,
                crate::commands::get_setting,
                crate::commands::set_setting,
                crate::commands::get_all_settings,
                crate::commands::set_all_settings,
                crate::commands::get_appearance,
                crate::commands::set_appearance,
                crate::commands::get_pin_top,
                crate::commands::set_pin_top,
                crate::commands::set_window_height,
                crate::commands::start_dragging,
                crate::commands::set_dragging,
                crate::commands::quit_app,
                crate::commands::build_file_index,
                crate::commands::get_index_status,
                crate::commands::file_search,
            ])
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    }
}

// 让 `app::run` 被外部调用（保持 namespace 名称 compat）
pub fn run() {
    app::run();
}

/// 给 lib.rs 调用的入口
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run_lib() {
    run();
}
