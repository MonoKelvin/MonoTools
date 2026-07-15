//! Tauri 应用构建与命令入口

pub mod app {
    use crate::models::Settings;
    use crate::repositories::*;
    use crate::search_engine::app_search::AppSearchEngine;
    use crate::search_engine::command_search::CommandSearchEngine;
    use crate::search_engine::file_search::FileSearchEngine;
    use crate::search_engine::SearchEngine;
    use crate::services::app_state::AppState;
    use crate::services::hotkey::HotkeyService;
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

                // === 应用毛玻璃效果 ===
                // Win11 使用 window-vibrancy 的 Mica 效果 (性能最好)
                // Win10 使用纯 CSS backdrop-filter (避免 window-vibrancy 的性能问题)
                #[cfg(windows)]
                {
                    if let Some(window) = app.get_webview_window("search") {
                        let (major, minor) = unsafe {
                            let os_version = windows_sys::Win32::System::SystemInformation::GetVersion();
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

                // 初始默认行为依赖于设置；先构造 settings repo 读取 pin_to_top
                let default_settings = Settings::default();
                let initial_pin = default_settings.pin_to_top;

                // 构建系统托盘图标 + 菜单（含"窗口置顶"开关）
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
                                    // 置顶时立即显示并聚焦窗口，取消置顶时立即隐藏窗口
                                    if next {
                                        let _ = window.show();
                                        let _ = window.set_focus();
                                    } else {
                                        let _ = window.hide();
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

                #[cfg(debug_assertions)]
                log::info!("[boot] tauri Builder::setup 入口");

                // 同步构造 AppState（全部为廉价操作）：不再用 block_on 阻塞事件循环。
                // app_search.refresh_index() 与文件索引一律放到后台 spawn，避免卡死 webview。
                #[cfg(debug_assertions)]
                log::info!("[boot] setup: 同步构造 AppState");
                let settings_repo = Arc::new(InMemorySettingsRepo::new(Settings::default()));
                let command_repo: Arc<dyn crate::repositories::CommandRepo> =
                    Arc::new(InMemoryCommandRepo::new());
                let stats_repo = Arc::new(StatsRepo::new());
                let pin_repo = Arc::new(PinRepo::new());

                // app_search 用空缓存构造（不扫盘）；refresh_index 在下方后台 spawn。
                let app_search = Arc::new(AppSearchEngine::new_empty(settings_repo.clone()));
                let command_search = Arc::new(CommandSearchEngine::new(command_repo.clone()));

                let settings = settings_repo.get();
                let file_roots = settings.file_search_roots.clone();

                // 让 FileSearchEngine 选择盘符：
                // 1) 用户在 settings.file_search_drives 显式指定的盘符优先；
                // 2) 否则 **保持空 roots**，由 FileSearchEngine 内部动态枚举所有 NTFS 卷。
                //    （即使 file_search_roots 里有用户目录，我们也想要自动发现全部盘符；
                //     那是 HUD 数据维度而非驱动边界，不能让单一 C: 限制全集枚举。）
                let explicit_drives = !settings.file_search_drives.is_empty();
                let selected_roots = if explicit_drives {
                    let mut roots = file_roots.clone();
                    for drive in &settings.file_search_drives {
                        roots.push(PathBuf::from(format!("{}:\\", drive)));
                    }
                    roots
                } else {
                    // 不传 roots，交由 FileSearchEngine::new 走 `NtfsIndexer::new()` 路径。
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
                    is_dragging: Arc::new(Mutex::new(false)),
                });
                #[cfg(debug_assertions)]
                log::info!("[boot] setup: AppState 就绪");

                // 后台刷新应用索引（扫描开始菜单/桌面）— 不阻塞 setup，进度通过 index_progress 上报。
                // 使用增量索引: 每扫描完一个目录就通知前端, 让用户能立即看到应用逐步出现。
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

                let file_search_clone = state.file_search.clone();
                let app_handle_for_index = app.handle().clone();
                // 启动后台索引：无需等待 file_search_roots，启动时也应建立盘符索引，
                // 因为 NtfsIndexer 会自动枚举所有 NTFS 卷（动态获取）。
                tauri::async_runtime::spawn(async move {
                    #[cfg(debug_assertions)]
                    log::info!("[boot] 后台索引构建任务已 spawn");
                    log::info!("后台索引构建任务启动...");
                    let _ = app_handle_for_index.emit(
                        "index_progress",
                        serde_json::json!({
                            "status": "building",
                            "message": "正在检测盘符...",
                            "phase": "files",
                        }),
                    );
                    let start = std::time::Instant::now();
                    let app_for_progress = app_handle_for_index.clone();
                    let res = file_search_clone
                        .build_index_with_volume_progress(
                            move |volume, idx, cumulative, total_volumes| {
                                let drive = crate::platform::windows::usn::drive_label(volume);
                                let msg = if total_volumes == 0 {
                                    format!("正在索引 {}", drive)
                                } else {
                                    format!(
                                        "正在索引 {}（{}/{}） — 已累计 {} 个文件",
                                        drive, idx, total_volumes, cumulative
                                    )
                                };
                                let _ = app_for_progress.emit(
                                    "index_progress",
                                    serde_json::json!({
                                        "status": "building",
                                        "message": msg,
                                        "phase": "files",
                                        "files": cumulative,
                                        "volumes": total_volumes,
                                        "current_volume": drive,
                                        "current_index": idx,
                                    }),
                                );
                            },
                        )
                        .await;
                    match res {
                        Err(e) => {
                            log::error!("后台索引构建失败: {}", e);
                            let _ = app_handle_for_index.emit(
                                "index_progress",
                                serde_json::json!({
                                    "status": "error",
                                    "message": format!("索引构建失败: {}", e),
                                    "phase": "files",
                                }),
                            );
                        }
                        Ok(_) => {
                            log::info!("后台索引构建完成，耗时 {:?}", start.elapsed());
                            let total = file_search_clone.total();
                            let _ = app_handle_for_index.emit(
                                "index_progress",
                                serde_json::json!({
                                    "status": "completed",
                                    "message": "索引构建完成",
                                    "phase": "files",
                                    "files": total,
                                }),
                            );
                        }
                    }

                    use crate::search_engine::start_update_loop;
                    let fs_clone = file_search_clone.clone();
                    start_update_loop(
                        move || fs_clone.update_index(),
                        std::time::Duration::from_secs(120),
                    );
                });

                app.manage(state.clone());

                // 同步初始置顶状态到窗口
                #[cfg(debug_assertions)]
                log::info!("[boot] 同步 pin_to_top -> 窗口");
                if let Some(w) = app_handle.get_webview_window("search") {
                    let _ = w.set_always_on_top(state.settings_repo.get().pin_to_top);
                }

                // 注册快捷键（非阻塞）
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
                crate::commands::search_cmd,
                crate::commands::search_more_cmd,
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
                crate::commands::frontend_ready,
                crate::commands::list_command_specs,
                crate::commands::dispatch_command,
                crate::commands::get_app_icon,
                crate::commands::get_app_icons_batch,
                crate::commands::list_pinned,
                crate::commands::pin_item,
                crate::commands::unpin_item,
                crate::commands::open_file_location,
                crate::commands::show_file_properties,
                crate::commands::delete_file_to_recycle_bin,
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
