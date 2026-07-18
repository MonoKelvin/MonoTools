//! 窗口监控服务 - 跟踪系统当前激活的应用窗口
//!
//! 通过 Windows API (GetForegroundWindow) 定期轮询当前活动窗口,
//! 用于智能推荐: 当用户在不同应用间切换时, 推荐列表会动态调整.
//!
//! 设计原则:
//! - 轮询间隔 2 秒, 平衡实时性和 CPU 占用
//! - 使用防抖: 只有当活动窗口持续 N 次不变时才确认切换 (避免短暂切换噪音)
//! - 记录最近 N 个活动应用, 用于推荐算法的"当前上下文"

use parking_lot::Mutex;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

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
    /// 上次推荐刷新的时间.
    pub last_recommend_refresh: Instant,
}

impl Default for WindowMonitorState {
    fn default() -> Self {
        Self {
            active_app_path: String::new(),
            active_app_title: String::new(),
            recent_apps: Vec::new(),
            last_recommend_refresh: Instant::now(),
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

/// 窗口监控服务.
pub struct WindowMonitorService {
    state: Arc<Mutex<WindowMonitorState>>,
    stop_tx: Option<std::sync::mpsc::Sender<()>>,
}

impl WindowMonitorService {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(WindowMonitorState::default())),
            stop_tx: None,
        }
    }

    /// 获取当前状态快照.
    pub fn snapshot(&self) -> WindowMonitorState {
        self.state.lock().clone()
    }

    /// 获取最近活动应用列表.
    pub fn recent_apps(&self) -> Vec<ActiveAppEntry> {
        self.state.lock().recent_apps.clone()
    }

    /// 启动监控循环.
    pub fn start(&mut self, app_handle: AppHandle) {
        let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
        self.stop_tx = Some(stop_tx);

        let state = Arc::clone(&self.state);

        std::thread::spawn(move || {
            let mut stable_count: u32 = 0;
            let mut last_detected_path = String::new();
            let mut last_detected_title = String::new();

            loop {
                // 检查停止信号
                if stop_rx.try_recv().is_ok() {
                    log::info!("[window_monitor] 监控循环已停止");
                    break;
                }

                // 获取当前活动窗口
                let (path, title) = get_foreground_window_info();

                if path != last_detected_path || title != last_detected_title {
                    // 窗口变化, 重置防抖
                    stable_count = 1;
                    last_detected_path = path.clone();
                    last_detected_title = title.clone();
                } else {
                    stable_count += 1;
                }

                // 只有稳定 N 次后才确认切换
                if stable_count >= DEBOUNCE_COUNT && !path.is_empty() {
                    let mut s = state.lock();
                    let now = Instant::now();

                    let changed = s.active_app_path != path;
                    if changed {
                        // 旧应用加入历史记录
                        if !s.active_app_path.is_empty() {
                            let entry = ActiveAppEntry {
                                path: s.active_app_path.clone(),
                                title: s.active_app_title.clone(),
                                first_seen: now,
                                last_seen: now,
                                switch_count: 1,
                            };
                            s.recent_apps.insert(0, entry);
                            if s.recent_apps.len() > MAX_RECENT_APPS {
                                s.recent_apps.truncate(MAX_RECENT_APPS);
                            }
                        }

                        s.active_app_path = path.clone();
                        s.active_app_title = title.clone();

                        log::info!(
                            "[window_monitor] 活动窗口切换: {} ({})",
                            s.active_app_title,
                            s.active_app_path
                        );

                        // 通知前端窗口已切换
                        let _ = app_handle.emit(
                            "window_changed",
                            serde_json::json!({
                                "path": s.active_app_path,
                                "title": s.active_app_title,
                                "recent_count": s.recent_apps.len(),
                            }),
                        );
                    }

                    stable_count = 0;
                }

                std::thread::sleep(POLL_INTERVAL);
            }
        });

        log::info!(
            "[window_monitor] 窗口监控已启动 (poll_interval={:?}, debounce={})",
            POLL_INTERVAL,
            DEBOUNCE_COUNT
        );
    }

    /// 停止监控.
    pub fn stop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Default for WindowMonitorService {
    fn default() -> Self {
        Self::new()
    }
}

/// 获取当前活动窗口的进程路径和窗口标题.
#[cfg(windows)]
fn get_foreground_window_info() -> (String, String) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId, GetWindowTextW, GetWindowTextLengthW,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return (String::new(), String::new());
        }

        // 获取进程 ID
        let mut pid: u32 = 0;
        let _tid = GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return (String::new(), String::new());
        }

        // 获取进程路径
        let path = get_process_path(pid).unwrap_or_default();

        // 获取窗口标题
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
    use std::ffi::c_void;
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
    // 非 Windows 平台返回空
    (String::new(), String::new())
}
