//! PE 版本信息提取 —— 从 Windows PE 文件的 VS_VERSION_INFO 资源中读取 FileVersion.
//!
//! TODO: windows 0.62 crate 中 GetFileVersionInfo 系列 API 的模块路径尚未确定。
//! 已尝试: Win32::System::Diagnostics::Debug, Win32::System::LibraryLoader,
//! Win32::System::WindowsProgramming — 均未找到。
//! 备选方案: 使用 windows-sys crate 或手动 FFI 绑定。
//! 当前 stub 不影响索引流程，版本字段保持 None。

use std::path::Path;

/// 从可执行文件 (.exe / .dll) 提取 FileVersion 字符串.
///
/// 当前为 stub 实现，始终返回 None.
#[cfg(windows)]
pub fn get_pe_version(_path: &Path) -> Option<String> {
    None
}

#[cfg(not(windows))]
pub fn get_pe_version(_path: &Path) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::get_pe_version;
    use std::path::Path;

    #[test]
    fn version_stub_returns_none() {
        assert!(get_pe_version(Path::new("C:\\Windows\\System32\\notepad.exe")).is_none());
    }
}
