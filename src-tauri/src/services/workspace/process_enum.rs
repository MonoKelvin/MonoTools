use crate::models::workspace::{AppSnapshot, WindowRect, WindowState};
use anyhow::Result;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, HANDLE};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows::Win32::Foundation::CloseHandle;
use std::collections::VecDeque;

pub struct WindowEnumerator;

impl WindowEnumerator {
    pub fn capture_workspace() -> Result<Vec<AppSnapshot>> {
        let mut snapshots = Vec::new();

        unsafe {
            EnumWindows(
                Some(Self::enum_callback),
                LPARAM(&mut snapshots as *mut _ as isize),
            )
            .map_err(|e| anyhow::anyhow!("Failed to enumerate windows: {:?}", e))?;
        }

        Ok(snapshots)
    }

    extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            // 过滤不可见窗口
            if !IsWindowVisible(hwnd).as_bool() {
                return BOOL(1);
            }

            // 获取窗口标题
            let mut title = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title);
            if len == 0 {
                return BOOL(1);
            }
            let title = String::from_utf16_lossy(&title[..len as usize]);

            // 过滤系统窗口
            if Self::is_system_window(&title) {
                return BOOL(1);
            }

            // 获取进程 ID
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));

            // 获取窗口位置和状态
            let mut placement = WINDOWPLACEMENT::default();
            placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
            if GetWindowPlacement(hwnd, &mut placement).is_err() {
                return BOOL(1);
            }

            let rect = placement.rcNormalPosition;

            // 获取 EXE 路径
            let exe_path = Self::get_process_path(pid);

            let snapshots = &mut *(lparam.0 as *mut Vec<AppSnapshot>);
            snapshots.push(AppSnapshot {
                id: uuid::Uuid::new_v4().to_string(),
                exe_path,
                args: vec![],
                working_dir: None,
                window_title: title,
                window_rect: WindowRect {
                    x: rect.left,
                    y: rect.top,
                    width: rect.right - rect.left,
                    height: rect.bottom - rect.top,
                },
                window_state: match placement.showCmd {
                    2 => WindowState::Minimized,  // SW_SHOWMINIMIZED
                    3 => WindowState::Maximized,  // SW_SHOWMAXIMIZED
                    _ => WindowState::Normal,
                },
                launch_order: 0,
                launch_delay_ms: 500,
                require_admin: false,
            });

            BOOL(1)
        }
    }

    unsafe fn get_process_path(pid: u32) -> String {
        let handle = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false,
            pid,
        );

        let handle = match handle {
            Ok(h) => h,
            Err(_) => return String::new(),
        };

        let mut path = [0u16; 512];
        let len = GetModuleFileNameExW(handle, None, &mut path);
        let result = String::from_utf16_lossy(&path[..len as usize]);

        let _ = CloseHandle(handle);
        result
    }

    fn is_system_window(title: &str) -> bool {
        let system_titles = [
            "Program Manager",
            "Windows Input Experience",
            "Search",
            "MonoTools", // 排除自己
        ];
        system_titles.contains(&title)
    }
}
