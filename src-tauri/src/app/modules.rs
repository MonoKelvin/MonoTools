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
use crate::core::command::{
    build_core_registry, CommandRegistry, CommandRepo, InMemoryCommandRepo,
};
use crate::core::settings::{InMemorySettingsRepo, Settings, SettingsRepo};
use crate::search_engine::app_search::AppSearchEngine;
use crate::search_engine::command_search::CommandSearchEngine;
use crate::search_engine::file_search::FileSearchEngine;
use crate::search_engine::{PinRepo, SearchEngine, StatsRepo};
use crate::services::hotkey::HotkeyService;
use crate::services::tray::{TrayMenuItem, TrayService};
use crate::services::window::WindowService;
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
    let command_repo: Arc<dyn CommandRepo> = Arc::new(InMemoryCommandRepo::new());
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
        is_dragging: Arc::new(Mutex::new(false)),
        frontend_initialized: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    })
}

/// 初始化所有业务模块
///
/// 在 setup 阶段调用，按顺序初始化各模块。
///
/// # 架构原则
/// - 本函数是 app 层唯一知道业务模块存在的地方
/// - 各模块自己负责初始化逻辑，这里只调用入口函数
/// - 模块所需依赖通过参数注入（依赖倒置）
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
    crate::services::hotkey::init_hotkey_service(app, state.hotkey.clone(), initial_hotkey);

    // 独立模块: Recommend (智能推荐)
    #[cfg(feature = "recommend")]
    {
        crate::recommend::init_full(app, crate::recommend::RecommendInitConfig::default());
    }

    // 仅 pybridge，不带 recommend（通用基础设施）
    #[cfg(feature = "pybridge")]
    {
        use crate::pybridge::{self, PyBridgeConfig};
        let config = PyBridgeConfig::default();
        pybridge::init(app, config);
    }
}

/// 应用窗口后置初始化（pin_to_top 等）
pub fn post_window_init(app: &AppHandle, state: &Arc<AppState>) {
    let pin_to_top = state.settings_repo.get().pin_to_top;
    crate::services::window::post_init(app, pin_to_top);
}

/// 设置系统托盘
///
/// 各业务模块自己负责注册菜单项，这里只负责组装。
/// 删除模块时，只需移除对应模块的注册调用。
pub fn setup_tray(app: &tauri::App<tauri::Wry>) -> Result<(), Box<dyn std::error::Error>> {
    let mut tray_service = TrayService::new();

    // 窗口模块菜单项
    crate::services::window::register_tray_items(&mut tray_service);

    // 设置模块菜单项
    crate::core::settings::tray::register_tray_items(&mut tray_service, app);

    // 通用菜单项
    tray_service.register_item(TrayMenuItem::separator());
    tray_service.register_item(TrayMenuItem::quit());

    tray_service.setup(app)?;
    app.manage(std::sync::Mutex::new(tray_service));

    Ok(())
}

/// 注册 IPC 命令到 builder
///
/// 统一注册所有模块的 IPC 命令，避免多次调用 invoke_handler 导致覆盖。
/// Tauri 的 invoke_handler 只能设置一次，多次调用会被最后一次覆盖。
///
/// # 架构原则
/// - 所有模块的命令统一在此列出，一次性注册
/// - 新增模块命令时，在此处添加对应的命令函数
/// - 各模块内部仍保留命令实现，内聚性不变
pub fn register_ipc_commands(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder.invoke_handler(tauri::generate_handler![
        // core::settings
        crate::core::settings::ipc::get_setting,
        crate::core::settings::ipc::set_setting,
        crate::core::settings::ipc::get_all_settings,
        crate::core::settings::ipc::set_all_settings,
        crate::core::settings::ipc::get_appearance,
        crate::core::settings::ipc::set_appearance,
        crate::core::settings::ipc::get_pin_top,
        crate::core::settings::ipc::set_pin_top,
        crate::core::settings::ipc::set_follow_system_theme,
        // core::command
        crate::core::command::ipc::list_commands,
        crate::core::command::ipc::add_command,
        crate::core::command::ipc::remove_command,
        crate::core::command::ipc::run_command,
        // search_engine
        crate::search_engine::ipc::search_cmd,
        crate::search_engine::ipc::search_more_cmd,
        crate::search_engine::ipc::execute_result,
        crate::search_engine::ipc::build_file_index,
        crate::search_engine::ipc::get_index_status,
        crate::search_engine::ipc::list_pinned,
        crate::search_engine::ipc::pin_item,
        crate::search_engine::ipc::unpin_item,
        // services::window
        crate::services::window::ipc::show_window,
        crate::services::window::ipc::hide_window,
        crate::services::window::ipc::toggle_window,
        crate::services::window::ipc::set_window_height,
        crate::services::window::ipc::start_dragging,
        crate::services::window::ipc::set_dragging,
        crate::services::window::ipc::quit_app,
        // services::hotkey
        crate::services::hotkey::ipc::register_hotkey_cmd,
        crate::services::hotkey::ipc::unregister_hotkey,
        crate::services::hotkey::ipc::get_current_hotkey,
        // platform::windows
        #[cfg(windows)]
        crate::platform::windows::ipc::get_app_icon,
        #[cfg(windows)]
        crate::platform::windows::ipc::get_app_icons_batch,
        #[cfg(windows)]
        crate::platform::windows::ipc::open_file_location,
        #[cfg(windows)]
        crate::platform::windows::ipc::show_file_properties,
        #[cfg(windows)]
        crate::platform::windows::ipc::delete_file_to_recycle_bin,
        #[cfg(windows)]
        crate::platform::windows::ipc::get_system_theme,
        // app (框架级)
        crate::app::ipc::frontend_ready,
        crate::app::ipc::list_command_specs,
        crate::app::ipc::dispatch_command,
        // recommend (可选模块)
        #[cfg(feature = "recommend")]
        crate::recommend::ipc::recommend_set_items,
        #[cfg(feature = "recommend")]
        crate::recommend::ipc::recommend_record_launch,
        #[cfg(feature = "recommend")]
        crate::recommend::ipc::recommend_get_scores,
        #[cfg(feature = "recommend")]
        crate::recommend::ipc::recommend_report_feedback,
        #[cfg(feature = "recommend")]
        crate::recommend::ipc::recommend_get_status,
        #[cfg(feature = "recommend")]
        crate::recommend::ipc::get_window_monitor_state,
    ])
}

/// 构建包含所有已注册命令的 registry。
///
/// 这是框架级的组装点，负责把 core 层命令 + 各业务模块命令组装在一起。
///
/// # 架构原则
/// - core 层只提供 `build_core_registry()`，不含任何业务命令
/// - 各业务模块提供 `register_commands(&mut registry)` 函数
/// - 本函数是唯一的组装点，新增模块只需加一行调用
pub fn build_command_registry() -> CommandRegistry {
    let mut reg = build_core_registry();

    #[cfg(windows)]
    crate::platform::windows::commands::register_commands(&mut reg);

    crate::search_engine::commands::register_commands(&mut reg);

    reg
}
