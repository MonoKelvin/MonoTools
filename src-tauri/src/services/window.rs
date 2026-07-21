//! 窗口管理 - 显示/隐藏/居中等
use crate::core::error::Result;
use crate::services::tray::TrayMenuItem;
use parking_lot::Mutex;
use std::sync::Arc;
use tauri::Manager;
use tauri::Runtime;

/// 通用 AppHandle 容器（不关心具体 Runtime）
pub trait AppLike: Send + Sync {
    fn show_window(&self, label: &str) -> Result<()>;
    fn hide_window(&self, label: &str) -> Result<()>;
    fn is_visible(&self, label: &str) -> bool;
    fn focus_window(&self, label: &str) -> Result<()>;
    fn toggle_window(&self, label: &str) -> Result<()>;
    fn close_window(&self, label: &str) -> Result<()>;
    fn web_window_handle(&self, label: &str) -> Option<WindowHandle>;
}

pub struct WindowService {
    pub app: Mutex<Option<Arc<dyn AppLike>>>,
}

impl WindowService {
    pub fn new<R: Runtime>(app: tauri::AppHandle<R>) -> Self {
        let wrapper = RuntimeAppHandle::<R>::new(app);
        Self {
            app: Mutex::new(Some(Arc::new(wrapper))),
        }
    }

    pub fn new_ref() -> Arc<Self> {
        Arc::new_cyclic(|_| Self {
            app: Mutex::new(None),
        })
    }

    pub fn handle_for(&self, label: &str) -> Option<WindowHandle> {
        let g = self.app.lock();
        let app = g.as_ref()?;
        app.web_window_handle(label)
    }

    pub async fn show(&self) -> Result<()> {
        let g = self.app.lock();
        if let Some(app) = g.as_ref() {
            app.show_window("search")?;
            app.focus_window("search")?;
        }
        Ok(())
    }

    pub async fn hide(&self) -> Result<()> {
        let g = self.app.lock();
        if let Some(app) = g.as_ref() {
            app.hide_window("search")?;
        }
        Ok(())
    }

    pub async fn toggle<R: Runtime>(&self, _app: &tauri::AppHandle<R>) -> Result<()> {
        let g = self.app.lock();
        if let Some(app) = g.as_ref() {
            app.toggle_window("search")?;
        }
        Ok(())
    }
}

pub struct WindowHandle {
    pub label: String,
    pub is_visible_fn: Box<dyn Fn() -> bool + Send + Sync>,
    pub hide_fn: Box<dyn Fn() + Send + Sync>,
    pub show_fn: Box<dyn Fn() + Send + Sync>,
    pub focus_fn: Box<dyn Fn() + Send + Sync>,
    pub close_fn: Box<dyn Fn() + Send + Sync>,
}

impl WindowHandle {
    pub fn is_visible(&self) -> bool {
        (self.is_visible_fn)()
    }
    pub fn hide(&self) {
        (self.hide_fn)()
    }
    pub fn show(&self) {
        (self.show_fn)()
    }
    pub fn focus(&self) {
        (self.focus_fn)()
    }
    pub fn close(&self) {
        (self.close_fn)()
    }
}

struct RuntimeAppHandle<R: Runtime> {
    inner: tauri::AppHandle<R>,
}

impl<R: Runtime> RuntimeAppHandle<R> {
    fn new(app: tauri::AppHandle<R>) -> Self {
        Self { inner: app }
    }
}

impl<R: Runtime> AppLike for RuntimeAppHandle<R> {
    fn show_window(&self, label: &str) -> Result<()> {
        if let Some(w) = self.inner.get_webview_window(label) {
            let _ = w.show();
            Ok(())
        } else {
            Err(crate::core::error::AppError::NotFound(label.into()))
        }
    }

    fn hide_window(&self, label: &str) -> Result<()> {
        if let Some(w) = self.inner.get_webview_window(label) {
            let _ = w.hide();
            Ok(())
        } else {
            Err(crate::core::error::AppError::NotFound(label.into()))
        }
    }

    fn is_visible(&self, label: &str) -> bool {
        self.inner
            .get_webview_window(label)
            .and_then(|w| w.is_visible().ok())
            .unwrap_or(false)
    }

    fn focus_window(&self, label: &str) -> Result<()> {
        if let Some(w) = self.inner.get_webview_window(label) {
            let _ = w.set_focus();
            Ok(())
        } else {
            Err(crate::core::error::AppError::NotFound(label.into()))
        }
    }

    fn toggle_window(&self, label: &str) -> Result<()> {
        if let Some(w) = self.inner.get_webview_window(label) {
            if w.is_visible().unwrap_or(false) {
                let _ = w.hide();
            } else {
                let _ = w.show();
                let _ = w.set_focus();
            }
            Ok(())
        } else {
            Err(crate::core::error::AppError::NotFound(label.into()))
        }
    }

    fn close_window(&self, label: &str) -> Result<()> {
        if let Some(w) = self.inner.get_webview_window(label) {
            let _ = w.close();
            Ok(())
        } else {
            Err(crate::core::error::AppError::NotFound(label.into()))
        }
    }

    fn web_window_handle(&self, label: &str) -> Option<WindowHandle> {
        let w = self.inner.get_webview_window(label)?;
        let label = label.to_string();
        Some(WindowHandle {
            label: label.clone(),
            is_visible_fn: {
                let w = w.clone();
                Box::new(move || w.is_visible().unwrap_or(false))
            },
            hide_fn: {
                let w = w.clone();
                Box::new(move || {
                    let _ = w.hide();
                })
            },
            show_fn: {
                let w = w.clone();
                Box::new(move || {
                    let _ = w.show();
                })
            },
            focus_fn: {
                let w = w.clone();
                Box::new(move || {
                    let _ = w.set_focus();
                })
            },
            close_fn: {
                let w = w.clone();
                Box::new(move || {
                    let _ = w.close();
                })
            },
        })
    }
}

impl WindowService {
    pub fn set_app<R: Runtime>(&self, app: tauri::AppHandle<R>) {
        let wrapper = RuntimeAppHandle::<R>::new(app);
        *self.app.lock() = Some(Arc::new(wrapper));
    }
}

/// 注册窗口相关的托盘菜单项
pub fn register_tray_items(tray_service: &mut crate::services::tray::TrayService) {
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
    tray_service.register_item(TrayMenuItem::normal("hide", "隐藏窗口", |app, _id| {
        if let Some(w) = app.get_webview_window("search") {
            let _ = w.hide();
        }
    }));
}

/// 窗口后置初始化（pin_to_top 等）
pub fn post_init<R: tauri::Runtime>(app: &tauri::AppHandle<R>, pin_to_top: bool) {
    // 确保窗口在 frontend_ready 之前保持隐藏
    if let Some(w) = app.get_webview_window("search") {
        let _ = w.hide();
    }

    // 同步 pin_to_top 设置到窗口
    if let Some(w) = app.get_webview_window("search") {
        let _ = w.set_always_on_top(pin_to_top);
    }
}

/// 窗口服务 IPC 命令
///
/// 独立模块设计：窗口相关的 IPC 命令定义在此模块中，
/// 通过 core_ipc_commands! 宏注册到全局。
pub mod ipc {
    use crate::app::state::AppState;
    use crate::core::config::window;
    use std::sync::Arc;
    use tauri::{AppHandle, LogicalSize, Manager, State, WebviewWindow};

    #[tauri::command]
    pub async fn show_window(state: State<'_, Arc<AppState>>) -> Result<(), String> {
        state.window.show().await.map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn hide_window(state: State<'_, Arc<AppState>>) -> Result<(), String> {
        state.window.hide().await.map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn toggle_window(
        app: AppHandle,
        state: State<'_, Arc<AppState>>,
    ) -> Result<(), String> {
        state.window.toggle(&app).await.map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn set_window_height(app: AppHandle, height: u32) -> Result<(), String> {
        let Some(w) = app.get_webview_window("search") else {
            return Ok(());
        };
        let height = height.clamp(window::MIN_HEIGHT, window::MAX_HEIGHT);
        let _ = w.set_size(LogicalSize::new(window::DEFAULT_WIDTH, height as f64));
        Ok(())
    }

    /// 应用层提供的"开始拖拽窗口"命令。
    #[tauri::command]
    pub async fn start_dragging(
        state: State<'_, Arc<AppState>>,
        window: WebviewWindow,
    ) -> Result<(), String> {
        if let Ok(mut dragging) = state.is_dragging.lock() {
            *dragging = true;
        }

        window.start_dragging().map_err(|e| {
            println!("[ERROR] Failed to start dragging: {}", e);
            format!("Failed to start dragging: {e}")
        })?;

        Ok(())
    }

    /// 设置拖拽状态
    #[tauri::command]
    pub async fn set_dragging(
        state: State<'_, Arc<AppState>>,
        dragging: bool,
    ) -> Result<(), String> {
        if let Ok(mut is_dragging) = state.is_dragging.lock() {
            *is_dragging = dragging;
        }
        Ok(())
    }

    /// 退出整个应用。
    #[tauri::command]
    pub async fn quit_app(app: AppHandle) -> Result<(), String> {
        app.exit(0);
        Ok(())
    }
}
