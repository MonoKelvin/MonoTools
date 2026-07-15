//! 窗口管理 - 显示/隐藏/居中等
use crate::core::error::Result;
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
