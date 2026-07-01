// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;
mod commands;
mod config;
mod models;
mod plugins;
mod services;
mod utils;

use anyhow::Result;
use commands::bus::CommandBus;
use config::store::ConfigStore;
use services::file_indexer::UsnIndexer;
use std::sync::Arc;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_single_instance::SingleInstanceBuilder;
use tokio::sync::RwLock;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

struct AppState {
    command_bus: Arc<CommandBus>,
    config: Arc<RwLock<ConfigStore>>,
}

#[tauri::command]
async fn execute_command(
    input: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<commands::bus::CommandResponse, String> {
    use commands::bus::*;
    use commands::parser::CommandParser;

    // 解析命令
    let cmd = CommandParser::parse(&input).map_err(|e| e.to_string())?;

    // 创建命令上下文
    let ctx = CommandContext {
        app_handle,
        window: None,
        plugin_manager: Arc::new(plugins::manager::PluginManager::new()),
        config: state.config.clone(),
        caller: commands::bus::CallerType::Ipc,
    };

    // 执行命令
    state.command_bus.execute(cmd, ctx).await.map_err(|e| e.to_string())
}

fn setup_logging() -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(true)
        .with_target(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    setup_logging().expect("Failed to setup logging");

    info!("MonoTools starting...");

    // 初始化配置
    let config = Arc::new(RwLock::new(
        ConfigStore::new().expect("Failed to initialize config store")
    ));

    // 初始化命令总线
    let command_bus = Arc::new(CommandBus::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            config.read().unwrap().get("general.autostart").unwrap_or(true)
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // 单实例：显示已有实例窗口
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.setFocus();
            }
        }))
        .setup(|app| {
            // 获取主窗口
            let window = app.get_webview_window("main").unwrap();

            // 配置单实例
            // 获取配置
            let cfg = config.read().unwrap();

            // 注册全局快捷键 (Alt + Space)
            // TODO: 实现全局热键注册

            // 设置系统托盘
            services::tray::setup_tray(app)?;

            // 启动文件索引器（后台任务）
            if cfg.get("fileSearch.indexOnStartup").unwrap_or(true) {
                let indexer = UsnIndexer::new();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = indexer.build_index().await {
                        error!("Failed to build file index: {}", e);
                    }
                });
            }

            // 注册 Tauri 命令
            app.manage(tauri::async_runtime::block_on(async {
                AppState {
                    command_bus: command_bus.clone(),
                    config: config.clone(),
                }
            }));

            // 加载内置插件
            info!("Loading builtin plugins...");
            // TODO: 加载内置插件

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::Focused(focused) if !focused => {
                // 窗口失焦时隐藏
                let _ = window.hide();
            }
            tauri::WindowEvent::CloseRequested { api, .. } => {
                // 阻止窗口关闭，改为隐藏
                window.hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![execute_command])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    info!("MonoTools started successfully");
}
