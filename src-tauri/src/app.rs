//! Tauri 应用构建与命令入口

pub mod app {
    use crate::services::app_state::AppState;
    use crate::services::hotkey::HotkeyService;
    use crate::services::window::WindowService;
    use crate::engines::app_search::AppSearchEngine;
    use crate::engines::command_search::CommandSearchEngine;
    use crate::engines::file_search::FileSearchService;
    use crate::engines::startup_search::StartupSearchService;
    use crate::repositories::*;
    use crate::models::Settings;
    use std::sync::Arc;
    use tauri::Manager;

    pub fn run() {
        if std::env::var("RUST_LOG").is_err() {
            std::env::set_var("RUST_LOG", "info,monotools_lib=debug");
        }
        let _ = env_logger::try_init();

        tauri::Builder::default()
            .plugin(tauri_plugin_global_shortcut::Builder::new().build())
            .plugin(tauri_plugin_shell::init())
            .plugin(tauri_plugin_fs::init())
            .setup(|app| {
                let app_handle = app.handle().clone();
                let state: Arc<AppState> = tauri::async_runtime::block_on(async {
                    let settings_repo = Arc::new(InMemorySettingsRepo::new(Settings::default()));
                    let startup_repo = Arc::new(InMemoryStartupRepo::new());
                    let command_repo: Arc<dyn crate::repositories::CommandRepo> =
                        Arc::new(InMemoryCommandRepo::new());
                    let stats_repo = Arc::new(StatsRepo::new());

                    let app_search = Arc::new(AppSearchEngine::new(settings_repo.clone()).await?);
                    app_search.refresh_index().await?;
                    let startup_search =
                        Arc::new(StartupSearchService::new(startup_repo.clone()).await?);
                    let command_search = Arc::new(CommandSearchEngine::new(
                        command_repo.clone(),
                        startup_repo.clone(),
                    ));
                    let file_roots = settings_repo.get().file_search_roots.clone();
                    let file_search = Arc::new(FileSearchService::new(file_roots));
                    if !file_roots.is_empty() {
                        let _ = file_search.build_index().await;
                    }
                    startup_search.refresh().await?;

                    let hotkey = Arc::new(HotkeyService::new());
                    let app_for_window = app_handle.clone();
                    let window_inner = WindowService::new(app_for_window);
                    let window = Arc::new(window_inner);

                    Ok::<_, crate::error::AppError>(Arc::new(AppState {
                        app: app_handle.clone(),
                        settings_repo,
                        startup_repo,
                        command_repo,
                        stats_repo,
                        app_search,
                        startup_search,
                        command_search,
                        file_search,
                        hotkey,
                        window,
                    }))
                })?;

                app.manage(state.clone());

                // 注册快捷键
                let app_handle_for_setup = app.handle().clone();
                let initial_hotkey = state.settings_repo.get().hotkey.clone();
                if let Err(e) = tauri::async_runtime::block_on(async {
                    state.hotkey.register(&initial_hotkey, &app_handle_for_setup).await
                }) {
                    log::warn!("注册默认快捷键失败: {e}");
                }

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
                crate::commands::list_startup,
                crate::commands::toggle_startup,
                crate::commands::add_startup,
                crate::commands::remove_startup,
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
