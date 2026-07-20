//! 模块注册表 - 唯一的业务模块组装点
//!
//! # 架构原则
//!
//! - `app/builder.rs` 只通过本模块访问业务逻辑
//! - 新增/删除模块时，只需修改本文件
//! - 本文件是 app 层唯一知道业务模块存在的地方
//!
//! # 删除模块步骤
//!
//! 1. 删除模块目录（如 `src/search_engine/`）
//! 2. 删除 `lib.rs` 中的 `pub mod xxx;`
//! 3. 删除本文件中对应模块的注册代码
//! 4. 删除 `Cargo.toml` 中对应依赖（如有）

use crate::app::state::AppState;
use crate::models::Settings;
use crate::repositories::*;
use crate::search_engine::app_search::AppSearchEngine;
use crate::search_engine::command_search::CommandSearchEngine;
use crate::search_engine::file_search::FileSearchEngine;
use crate::search_engine::SearchEngine;
use crate::services::hotkey::HotkeyService;
use crate::services::tray::{TrayMenuItem, TrayService};
use crate::services::window::WindowService;
use crate::services::window_monitor::WindowMonitorService;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, WindowEvent};

/// 配置 Tauri builder（业务相关配置）
///
/// 所有业务相关的 builder 配置都在这里：
/// - single-instance 插件行为
/// - 窗口事件处理
/// - 等等
///
/// 删除模块时，只需修改对应配置。
pub fn configure_builder(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            log::info!("检测到新实例启动，激活已有窗口");
            if let Some(window) = app.get_webview_window("search") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
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
}

/// 构建完整的 AppState
///
/// 集中组装所有业务模块的状态。
/// 这是 app 层唯一直接引用业务模块的地方之一。
pub fn build_app_state(app_handle: &AppHandle) -> Arc<AppState> {
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

    let file_search = Arc::new(FileSearchEngine::new(selected_roots).unwrap());

    let search_engine = Arc::new(SearchEngine::new(
        app_search.clone(),
        file_search.clone(),
        command_search.clone(),
    ));

    let hotkey = Arc::new(HotkeyService::new());
    let window_inner = WindowService::new(app_handle.clone());
    let window = Arc::new(window_inner);

    let mut window_monitor = WindowMonitorService::new();
    window_monitor.start(app_handle.clone());
    let window_monitor = Arc::new(Mutex::new(window_monitor));

    Arc::new(AppState {
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
    })
}

/// 初始化所有业务模块
///
/// 在 setup 阶段调用，按顺序初始化各模块。
pub fn init_all_modules(app: &AppHandle, state: &Arc<AppState>) {
    // 搜索模块
    crate::search_engine::init::init(
        app,
        crate::search_engine::SearchInitParams {
            app_search: state.app_search.clone(),
            file_search: state.file_search.clone(),
            search_engine: state.search_engine.clone(),
        },
    );

    // 热键模块
    let initial_hotkey = state.settings_repo.get().hotkey.clone();
    crate::services::hotkey::init_hotkey_service(
        app,
        state.hotkey.clone(),
        initial_hotkey,
    );

    // 独立模块: Recommend (智能推荐)
    #[cfg(feature = "recommend")]
    {
        crate::recommend::init(app);
    }

    // 独立模块: PyBridge (Rust-Python 桥接)
    #[cfg(feature = "pybridge")]
    {
        use crate::pybridge::{self, PyBridgeConfig};
        let config = PyBridgeConfig::default();
        pybridge::init(app, config);
    }
}

/// 应用窗口后置初始化（pin_to_top 等）
pub fn post_window_init(app: &AppHandle, state: &Arc<AppState>) {
    // 确保窗口在 frontend_ready 之前保持隐藏
    if let Some(w) = app.get_webview_window("search") {
        let _ = w.hide();
    }

    // 同步 pin_to_top 设置到窗口
    if let Some(w) = app.get_webview_window("search") {
        let _ = w.set_always_on_top(state.settings_repo.get().pin_to_top);
    }
}

/// 核心 IPC 命令列表宏
///
/// 所有业务模块的 IPC 命令都在这里统一汇总。
/// 这是 app 层唯一直接引用业务模块 IPC 命令的地方。
#[macro_export]
macro_rules! core_ipc_commands {
    ($($extra:path),* $(,)?) => {
        tauri::generate_handler![
            // 搜索模块
            $crate::search_engine::ipc::search_cmd,
            $crate::search_engine::ipc::search_more_cmd,
            $crate::search_engine::ipc::execute_result,
            $crate::search_engine::ipc::build_file_index,
            $crate::search_engine::ipc::get_index_status,
            // 窗口服务
            $crate::services::window::ipc::show_window,
            $crate::services::window::ipc::hide_window,
            $crate::services::window::ipc::toggle_window,
            $crate::services::window::ipc::set_window_height,
            $crate::services::window::ipc::start_dragging,
            $crate::services::window::ipc::set_dragging,
            $crate::services::window::ipc::quit_app,
            // 热键服务
            $crate::services::hotkey::ipc::register_hotkey_cmd,
            $crate::services::hotkey::ipc::unregister_hotkey,
            $crate::services::hotkey::ipc::get_current_hotkey,
            // 窗口监控
            $crate::services::window_monitor::ipc::get_window_monitor_state,
            // 仓库层 (设置/命令/Pin)
            $crate::repositories::ipc::get_setting,
            $crate::repositories::ipc::set_setting,
            $crate::repositories::ipc::get_all_settings,
            $crate::repositories::ipc::set_all_settings,
            $crate::repositories::ipc::get_appearance,
            $crate::repositories::ipc::set_appearance,
            $crate::repositories::ipc::get_pin_top,
            $crate::repositories::ipc::set_pin_top,
            $crate::repositories::ipc::set_follow_system_theme,
            $crate::repositories::ipc::list_commands,
            $crate::repositories::ipc::add_command,
            $crate::repositories::ipc::remove_command,
            $crate::repositories::ipc::run_command,
            $crate::repositories::ipc::list_pinned,
            $crate::repositories::ipc::pin_item,
            $crate::repositories::ipc::unpin_item,
            // 平台层 (图标/Shell/主题)
            $crate::platform::windows::ipc::get_app_icon,
            $crate::platform::windows::ipc::get_app_icons_batch,
            $crate::platform::windows::ipc::open_file_location,
            $crate::platform::windows::ipc::show_file_properties,
            $crate::platform::windows::ipc::delete_file_to_recycle_bin,
            $crate::platform::windows::ipc::get_system_theme,
            // 命令系统
            $crate::core::command::ipc::list_command_specs,
            $crate::core::command::ipc::dispatch_command,
            // 框架级 (app 层)
            $crate::app::ipc::frontend_ready,
            $($extra),*
        ]
    };
}

/// 设置系统托盘
///
/// 集中注册所有托盘菜单项和处理函数。
/// 各模块的托盘菜单项在这里统一组装。
/// 删除模块时，只需移除对应菜单项的注册代码。
pub fn setup_tray(app: &tauri::App<tauri::Wry>) -> Result<(), Box<dyn std::error::Error>> {
    let mut tray_service = TrayService::new();

    // === 窗口相关菜单项 ===
    tray_service.register_item(TrayMenuItem::normal(
        "show",
        "显示主窗口",
        |app, _id| {
            if let Some(w) = app.get_webview_window("search") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        },
    ));
    tray_service.register_item(TrayMenuItem::normal(
        "hide",
        "隐藏窗口",
        |app, _id| {
            if let Some(w) = app.get_webview_window("search") {
                let _ = w.hide();
            }
        },
    ));

    // === 设置相关菜单项 ===
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

    tray_service.register_item(TrayMenuItem::separator());
    tray_service.register_item(TrayMenuItem::quit());

    tray_service.setup(app)?;
    app.manage(std::sync::Mutex::new(tray_service));

    Ok(())
}

/// 注册 IPC 命令到 builder
///
/// 根据 feature flag 决定是否追加独立模块的命令。
pub fn register_ipc_commands(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    #[cfg(not(feature = "recommend"))]
    let builder = builder.invoke_handler(crate::core_ipc_commands!());

    #[cfg(feature = "recommend")]
    let builder = builder.invoke_handler(crate::core_ipc_commands![
        crate::recommend::ipc::recommend_get_scores,
        crate::recommend::ipc::recommend_report_feedback,
        crate::recommend::ipc::recommend_get_status,
    ]);

    builder
}
