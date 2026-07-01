use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::UI::WindowsAndMessaging::*;

pub fn is_window_visible(hwnd: HWND) -> bool {
    unsafe { IsWindowVisible(hwnd).as_bool() }
}

pub fn get_window_title(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut buffer = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut buffer);
        if len > 0 {
            Some(String::from_utf16_lossy(&buffer[..len as usize]))
        } else {
            None
        }
    }
}

pub fn get_window_rect(hwnd: HWND) -> Option<RECT> {
    unsafe {
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            Some(rect)
        } else {
            None
        }
    }
}

pub fn get_window_placement(hwnd: HWND) -> Option<WINDOWPLACEMENT> {
    unsafe {
        let mut placement = WINDOWPLACEMENT::default();
        placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
        if GetWindowPlacement(hwnd, &mut placement).is_ok() {
            Some(placement)
        } else {
            None
        }
    }
}

pub fn set_window_placement(hwnd: HWND, placement: &WINDOWPLACEMENT) -> anyhow::Result<()> {
    unsafe {
        SetWindowPlacement(hwnd, placement)
            .map_err(|e| anyhow::anyhow!("Failed to set window placement: {:?}", e))?;
    }
    Ok(())
}

pub fn set_window_pos(
    hwnd: HWND,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    flags: u32,
) -> anyhow::Result<()> {
    unsafe {
        SetWindowPos(
            hwnd,
            HWND::default(),
            x,
            y,
            width,
            height,
            flags,
        )
            .map_err(|e| anyhow::anyhow!("Failed to set window position: {:?}", e))?;
    }
    Ok(())
}
