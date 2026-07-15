//! 跨平台适配 - 在 Windows 上含 NTFS USN Journal + 注册表访问

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use windows::*;

#[cfg(not(windows))]
pub mod stub;

/// 非 Windows 占位实现
#[cfg(not(windows))]
pub mod stub {
    use crate::core::error::{AppError, Result};

    pub fn register_hotkey(_hotkey: &str) -> Result<()> {
        Err(AppError::Other("全局快捷键仅支持 Windows 平台".into()))
    }

    pub fn unregister_hotkey() -> Result<()> {
        Ok(())
    }

    pub async fn read_usn_journal() -> Result<()> {
        Err(AppError::Other("USN Journal 仅支持 Windows".into()))
    }
}
