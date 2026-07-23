//! Tauri 应用构建器 - 纯框架逻辑
//!
//! # 架构原则
//!
//! - 本文件只包含**框架级**的初始化逻辑
//! - 所有业务模块的组装都在 `app::modules` 中
//! - 本文件不直接引用任何业务模块（search_engine, services, repositories 等）
//! - 删除业务模块时，本文件不需要任何修改
//!
//! # 职责范围
//!
//! - 基础插件注册（通用框架插件）
//! - 调用模块注册表配置 builder（业务相关的事件、单例逻辑等）
//! - 调用模块注册表进行业务初始化和托盘设置
//! - 窗口显示的启动流程控制
//! - 窗口视觉效果（Mica 毛玻璃等框架级 UI 增强）

use tauri::Manager;

pub fn run() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info,monotools_lib=debug");
    }

    // 自定义日志格式：时间 级别 消息（去掉模块路径）
    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            use std::io::Write;
            let timestamp = buf.timestamp();
            writeln!(buf, "[{} {}] {}", timestamp, record.level(), record.args())
        })
        .try_init()
        .ok();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init());

    let builder = crate::app::modules::configure_builder(builder);

    let builder = builder.setup(|app| {
        #[cfg(debug_assertions)]
        log::info!("[boot] setup: enter");
        let app_handle = app.handle().clone();

        apply_window_effects(app);

        #[cfg(debug_assertions)]
        log::info!("[boot] setup: 构建 AppState");
        let state = crate::app::modules::build_app_state(&app_handle);
        #[cfg(debug_assertions)]
        log::info!("[boot] setup: AppState 就绪");

        app.manage(state.clone());

        crate::app::modules::post_window_init(&app_handle, &state);

        // 初始化图标磁盘缓存路径 (在 setup 中, app_handle 可用时)
        #[cfg(windows)]
        if let Ok(cache_dir) = app_handle.path().app_cache_dir() {
            crate::platform::windows::icon::init_disk_cache(&cache_dir);
            crate::platform::windows::icon::refresh_icon_index();
            log::info!("[icon] 磁盘缓存路径: {:?}", cache_dir.join("icons"));
        }

        #[cfg(debug_assertions)]
        log::info!("[boot] 初始化所有业务模块");
        crate::app::modules::init_all_modules(&app_handle, &state);

        #[cfg(debug_assertions)]
        log::info!("[boot] setup: 设置系统托盘");
        crate::app::modules::setup_tray(app)?;

        #[cfg(debug_assertions)]
        log::info!("[boot] setup 即将 return Ok(())");

        Ok(())
    });

    let builder = crate::app::modules::register_ipc_commands(builder);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 应用窗口视觉效果（Mica 毛玻璃等）
///
/// 平台相关的窗口效果设置，属于框架层的 UI 增强。
#[cfg(windows)]
fn apply_window_effects(app: &tauri::App) {
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

#[cfg(not(windows))]
fn apply_window_effects(_app: &tauri::App) {}
