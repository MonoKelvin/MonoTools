//! Windows 11 Mica / Acrylic 效果模块
//!
//! 使用 DWM (Desktop Window Manager) API 启用 Mica 背景效果。

#![cfg(windows)]

use windows_sys::Win32::Foundation::HWND;

/// DWM 系统属性: 启用 Mica 背景
const DWMWA_SYSTEMBACKDROP_TYPE: u32 = 38;
/// DWM 属性: 圆角窗口
const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;

/// Mica 背景类型
const DWMSBT_MAINWINDOW: u32 = 2; // Mica
const DWMSBT_TRANSIENTWINDOW: u32 = 3; // Acrylic
const DWMSBT_AUTO: u32 = 0;
/// 圆角 (Windows 11 风格)
const DWMWCP_ROUND: u32 = 2;

type DwmSetWindowAttributeFn = unsafe extern "system" fn(
    HWND,
    u32,
    *const core::ffi::c_void,
    u32,
) -> i32;

/// Windows 11 21H2 (build 22000) 之后支持 Mica
pub fn is_mica_supported() -> bool {
    use windows_sys::Win32::System::SystemInformation::OSVERSIONINFOW;

    unsafe {
        // 使用 Windows 8+ 推荐方法: 通过 ntdll.dll 的 RtlGetVersion 读取真实版本
        // 这里通过 kernel32 的 GetVersionExW 作为后备 (虽然可能受 manifest 影响)
        let mut ver: OSVERSIONINFOW = std::mem::zeroed();
        ver.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOW>() as u32;

        // 尝试 ntdll!RtlGetVersion
        let ntdll: Vec<u8> = b"ntdll.dll\0".to_vec();
        let module = windows_sys::Win32::System::LibraryLoader::LoadLibraryA(ntdll.as_ptr());
        if !module.is_null() {
            let proc_name: Vec<u8> = b"RtlGetVersion\0".to_vec();
            let proc = windows_sys::Win32::System::LibraryLoader::GetProcAddress(
                module,
                proc_name.as_ptr(),
            );
            if let Some(p) = proc {
                type RtlGetVersionFn = unsafe extern "system" fn(*mut OSVERSIONINFOW) -> i32;
                let rtl_get: RtlGetVersionFn = std::mem::transmute(p);
                if rtl_get(&mut ver) == 0 {
                    return ver.dwBuildNumber >= 22000;
                }
            }
        }

        // 后备: 用 dwMajorVersion 简单判断 Win11 = 10.0
        ver.dwMajorVersion == 10 && ver.dwMinorVersion == 0
    }
}

/// 获取 DwmSetWindowAttribute 函数指针
unsafe fn get_dwm_set_attr() -> Option<DwmSetWindowAttributeFn> {
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
    // dwmapi.dll 名称作为 ASCII 字符串
    let lib_name: Vec<u8> = b"dwmapi.dll\0".to_vec();
    let module = LoadLibraryA(lib_name.as_ptr());
    if module.is_null() {
        return None;
    }
    let proc_name: Vec<u8> = b"DwmSetWindowAttribute\0".to_vec();
    let proc = GetProcAddress(module, proc_name.as_ptr());
    if proc.is_none() {
        return None;
    }
    Some(std::mem::transmute(proc.unwrap()))
}

unsafe fn set_dwm_attr(
    dwm_set: DwmSetWindowAttributeFn,
    hwnd: HWND,
    attr: u32,
    value: u32,
) -> i32 {
    dwm_set(
        hwnd,
        attr,
        &value as *const u32 as *const core::ffi::c_void,
        std::mem::size_of::<u32>() as u32,
    )
}

/// 通过 DWM API 启用 Mica 背景
pub fn enable_mica(hwnd: HWND) -> bool {
    if !is_mica_supported() {
        log::info!("[mica] 系统不支持 Mica (需 Windows 11 21H2+)");
        return false;
    }
    unsafe {
        if let Some(dwm_set) = get_dwm_set_attr() {
            let result = set_dwm_attr(dwm_set, hwnd, DWMWA_SYSTEMBACKDROP_TYPE, DWMSBT_MAINWINDOW);
            if result == 0 {
                log::info!("[mica] Mica 背景已启用");
                true
            } else {
                log::warn!("[mica] DwmSetWindowAttribute 失败: {}", result);
                false
            }
        } else {
            log::warn!("[mica] 无法加载 DwmSetWindowAttribute");
            false
        }
    }
}

/// 同时启用 Mica 背景 + 圆角窗口
pub fn enable_mica_with_rounded_corners(hwnd: HWND) -> bool {
    let mica_ok = enable_mica(hwnd);
    unsafe {
        if let Some(dwm_set) = get_dwm_set_attr() {
            let _ = set_dwm_attr(dwm_set, hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND);
        }
    }
    mica_ok
}

/// 启用亚克力 (Acrylic) 效果
pub fn enable_acrylic(hwnd: HWND) -> bool {
    if !is_mica_supported() {
        return false;
    }
    unsafe {
        if let Some(dwm_set) = get_dwm_set_attr() {
            let result = set_dwm_attr(
                dwm_set,
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                DWMSBT_TRANSIENTWINDOW,
            );
            return result == 0;
        }
    }
    false
}

/// 移除窗口背景效果
pub fn disable_mica(hwnd: HWND) -> bool {
    unsafe {
        if let Some(dwm_set) = get_dwm_set_attr() {
            let result = set_dwm_attr(dwm_set, hwnd, DWMWA_SYSTEMBACKDROP_TYPE, DWMSBT_AUTO);
            return result == 0;
        }
    }
    false
}
