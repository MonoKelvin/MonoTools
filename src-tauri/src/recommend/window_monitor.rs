//! 窗口监控 - 推荐模块的内部功能
//!
//! 跟踪系统当前激活的应用窗口，用于智能推荐的上下文感知。
//!
//! 设计原则:
//! - 轮询间隔 2 秒，平衡实时性和 CPU 占用
//! - 防抖：连续 N 次检测到同一窗口才确认切换
//! - 记录最近 N 个活动应用，用于推荐算法的"当前上下文"

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use crate::recommend::RecommendService;

/// 轮询间隔: 每 2 秒检查一次活动窗口.
const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// 防抖计数: 连续 3 次检测到同一窗口才确认切换 (6 秒稳定期).
const DEBOUNCE_COUNT: u32 = 3;

/// 最近活动应用历史记录上限.
const MAX_RECENT_APPS: usize = 10;

/// 窗口监控状态.
#[derive(Debug, Clone)]
pub struct WindowMonitorState {
    /// 当前激活的应用路径.
    pub active_app_path: String,
    /// 当前激活的应用标题.
    pub active_app_title: String,
    /// 最近激活的应用列表 (最新在前).
    pub recent_apps: Vec<ActiveAppEntry>,
}

impl Default for WindowMonitorState {
    fn default() -> Self {
        Self {
            active_app_path: String::new(),
            active_app_title: String::new(),
            recent_apps: Vec::new(),
        }
    }
}

/// 单个活动应用记录.
#[derive(Debug, Clone)]
pub struct ActiveAppEntry {
    pub path: String,
    pub title: String,
    pub first_seen: Instant,
    pub last_seen: Instant,
    pub switch_count: u32,
}

/// 窗口监控器 - 推荐模块内部使用
pub struct WindowMonitor {
    state: RwLock<WindowMonitorState>,
}

impl WindowMonitor {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(WindowMonitorState::default()),
        }
    }

    /// 获取当前状态快照.
    pub async fn snapshot(&self) -> WindowMonitorState {
        self.state.read().await.clone()
    }

    /// 获取当前活动应用路径.
    pub async fn active_app_path(&self) -> String {
        self.state.read().await.active_app_path.clone()
    }

    /// 获取当前活动应用标题.
    pub async fn active_app_title(&self) -> String {
        self.state.read().await.active_app_title.clone()
    }

    /// 更新活动窗口（由监控循环调用）.
    pub(crate) async fn update(&self, path: String, title: String) -> bool {
        let mut state = self.state.write().await;
        let changed = state.active_app_path != path;

        if changed {
            // 旧应用加入历史记录
            if !state.active_app_path.is_empty() {
                let now = Instant::now();
                let entry = ActiveAppEntry {
                    path: state.active_app_path.clone(),
                    title: state.active_app_title.clone(),
                    first_seen: now,
                    last_seen: now,
                    switch_count: 1,
                };
                state.recent_apps.insert(0, entry);
                if state.recent_apps.len() > MAX_RECENT_APPS {
                    state.recent_apps.truncate(MAX_RECENT_APPS);
                }
            }

            state.active_app_path = path.clone();
            state.active_app_title = title.clone();

            log::info!(
                "[recommend] 活动窗口切换: {} ({})",
                state.active_app_title,
                state.active_app_path
            );
        }

        changed
    }
}

impl Default for WindowMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// 启动窗口监控线程
pub fn start_window_monitor<R: tauri::Runtime + 'static>(
    app: &tauri::AppHandle<R>,
    service: Arc<RecommendService>,
    monitor: Arc<WindowMonitor>,
) {
    let app_handle = app.clone();

    std::thread::spawn(move || {
        let mut stable_count: u32 = 0;
        let mut last_detected_path = String::new();
        let mut last_detected_title = String::new();

        loop {
            let (path, title) = get_foreground_window_info();

            if path != last_detected_path || title != last_detected_title {
                stable_count = 1;
                last_detected_path = path.clone();
                last_detected_title = title.clone();
            } else {
                stable_count += 1;
            }

            if stable_count >= DEBOUNCE_COUNT && !path.is_empty() {
                let app_handle = app_handle.clone();
                let service = service.clone();
                let monitor = monitor.clone();
                let path_clone = path.clone();
                let title_clone = title.clone();

                tauri::async_runtime::spawn(async move {
                    let changed = monitor
                        .update(path_clone.clone(), title_clone.clone())
                        .await;

                    // 同步更新推荐服务的前台应用信息
                    service
                        .update_foreground(path_clone.clone(), title_clone.clone())
                        .await;

                    // 通知前端
                    if changed {
                        use tauri::Emitter;
                        let recent_count = monitor.state.read().await.recent_apps.len();
                        let _ = app_handle.emit(
                            "window_changed",
                            serde_json::json!({
                                "path": path_clone,
                                "title": title_clone,
                                "recent_count": recent_count,
                            }),
                        );
                    }
                });

                stable_count = 0;
            }

            std::thread::sleep(POLL_INTERVAL);
        }
    });

    log::info!(
        "[recommend] 窗口监控已启动 (poll_interval={:?}, debounce={})",
        POLL_INTERVAL,
        DEBOUNCE_COUNT
    );
}

/// 获取当前活动窗口的进程路径和窗口标题.
#[cfg(windows)]
fn get_foreground_window_info() -> (String, String) {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return (String::new(), String::new());
        }

        let mut pid: u32 = 0;
        let _tid = GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return (String::new(), String::new());
        }

        let path = get_process_path(pid).unwrap_or_default();

        let title_len = GetWindowTextLengthW(hwnd);
        let title = if title_len > 0 {
            let mut buf: Vec<u16> = vec![0; (title_len + 1) as usize];
            let copied = GetWindowTextW(hwnd, &mut buf);
            if copied > 0 {
                OsString::from_wide(&buf[..copied as usize])
                    .to_string_lossy()
                    .to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        (path, title)
    }
}

#[cfg(windows)]
fn get_process_path(pid: u32) -> Option<String> {
    use std::ffi::{c_void, OsString};
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);
        if handle.is_err() {
            return None;
        }
        let handle = handle.ok()?;

        let mut buf: Vec<u16> = vec![0; 1024];
        let mut len: u32 = buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            HANDLE(handle.0 as *mut c_void),
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut len,
        );

        if result.is_ok() && len > 0 {
            Some(
                OsString::from_wide(&buf[..len as usize])
                    .to_string_lossy()
                    .to_string(),
            )
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
fn get_foreground_window_info() -> (String, String) {
    (String::new(), String::new())
}
