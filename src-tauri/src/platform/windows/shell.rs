//! Shell 工具 - 启动程序、打开路径、解析快捷方式、系统操作
use crate::core::error::{AppError, Result};
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(windows)]
use windows::core::{w, PCWSTR};
#[cfg(windows)]
use windows::Win32::Foundation::HWND;
#[cfg(windows)]
use windows::Win32::System::Registry::HKEY;
#[cfg(windows)]
use windows::Win32::UI::Shell::ShellExecuteW;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;

/// 启动应用（非阻塞）
pub fn launch(path: &str, args: &[String]) -> Result<u32> {
    let mut cmd = Command::new(path);
    cmd.args(args);
    cmd.spawn()
        .map(|c| c.id())
        .map_err(|e| AppError::Other(format!("启动失败: {e}")))
}

/// 以管理员权限启动
#[cfg(windows)]
pub fn launch_as_admin(path: &str, _args: &[String]) -> Result<()> {
    use windows::core::w;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let wide_path: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let result = unsafe {
        ShellExecuteW(
            Some(HWND::default()),
            w!("runas"),
            PCWSTR(wide_path.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };

    let result_val = result.0 as usize;
    if result_val <= 32 {
        return Err(AppError::Other(format!("以管理员启动失败，错误码: {}", result_val)));
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn launch_as_admin(path: &str, args: &[String]) -> Result<()> {
    launch(path, args).map(|_| ())
}

/// 打开文件所在目录并选中文件 (explorer /select,<完整路径>).
///
/// 注意: 直接传入文件完整路径即可, `/select,` 会自动打开父目录并选中该文件.
/// 不要先取 parent 再传进来, 否则会多往上跳一级导致定位错误.
///
/// 使用 PowerShell 包裹调用以正确处理路径中的特殊字符 (如逗号).
#[cfg(windows)]
pub fn open_path(path: &PathBuf) -> Result<()> {
    use crate::core::config::paths;
    use std::os::windows::process::CommandExt;

    let path_str = path.to_string_lossy().into_owned();

    // 使用 PowerShell 包裹 explorer 调用, 避免路径含逗号时参数被截断.
    // 例如: "C:\Some, App\app.exe" 直接传给 explorer /select, 会被逗号分隔.
    let ps_arg = format!(
        "Start-Process -FilePath 'explorer.exe' -ArgumentList '/select,\"{}\"'",
        path_str.replace("'", "''")  // PowerShell 单引号转义
    );

    std::process::Command::new("powershell")
        .arg("-Command")
        .arg(&ps_arg)
        .creation_flags(paths::CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| AppError::Other(format!("打开路径失败: {e}")))?;
    Ok(())
}

#[cfg(not(windows))]
pub fn open_path(path: &PathBuf) -> Result<()> {
    std::process::Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map_err(|e| AppError::Other(format!("打开路径失败: {e}")))?;
    Ok(())
}

/// 打开文件所在目录并选中该文件 (与 open_path 等价, 语义更明确).
///
/// 这是前端 "打开文件所在路径" 菜单项的后端实现.
/// 使用 `explorer /select,<完整路径>` 直接打开父目录并选中文件,
/// 不需要先手动取 parent (那样会多跳一级导致路径错误).
pub fn open_containing_folder(path: &PathBuf) -> Result<()> {
    open_path(path)
}

/// 显示文件属性对话框 (explorer /select,<path> 不够, 需要用 ShellExecuteW 调用 properties 谓词).
///
/// 通过 rundll32.exe 调用 shell32.dll 的 ShellExecuteW 来打开属性页.
/// 这是 Windows 上最可靠的"打开文件属性"方式之一。
#[cfg(windows)]
pub fn show_file_properties(path: &PathBuf) -> Result<()> {
    use crate::core::config::paths;
    use std::os::windows::process::CommandExt;

    // 使用 PowerShell 调用 Shell.Application 的 InvokeVerb 方式打开属性页
    std::process::Command::new("powershell")
        .arg("-Command")
        .arg(format!(
            "$shell = New-Object -ComObject Shell.Application; $item = $shell.Namespace('{}').ParseName('{}'); $item.InvokeVerb('properties')",
            path.parent().map(|p| p.to_string_lossy()).unwrap_or_default(),
            path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()
        ))
        .creation_flags(paths::CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| AppError::Other(format!("打开属性失败: {e}")))?;
    Ok(())
}

#[cfg(not(windows))]
pub fn show_file_properties(_path: &PathBuf) -> Result<()> {
    // Linux/macOS 上暂不实现
    Ok(())
}

/// 删除文件到回收站 (Windows 上使用 Shell 的删除方式, 带确认对话框).
///
/// 注意: 为了安全起见, 默认使用"移动到回收站"而非永久删除.
/// 如果回收站不可用或非 Windows 平台, 则永久删除 (调用方应自行确认).
#[cfg(windows)]
pub fn delete_to_recycle_bin(path: &Path) -> Result<()> {
    use crate::core::config::paths;
    use std::os::windows::process::CommandExt;

    let path_str = path.to_string_lossy().into_owned();

    // 使用 PowerShell 调用 Shell.Application 的 MoveHere 方式删除到回收站
    // 这是最可靠的"删除到回收站"方式
    std::process::Command::new("powershell")
        .arg("-Command")
        .arg(format!(
            "$shell = New-Object -ComObject Shell.Application; $recycle = $shell.NameSpace(10); $recycle.MoveHere('{}')",
            path_str
        ))
        .creation_flags(paths::CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| AppError::Other(format!("删除失败: {e}")))?;
    Ok(())
}

#[cfg(not(windows))]
pub fn delete_to_recycle_bin(path: &PathBuf) -> Result<()> {
    // 非 Windows 平台: 永久删除 (简化实现)
    std::fs::remove_file(path).map_err(|e| AppError::Other(format!("删除失败: {e}")))?;
    Ok(())
}

/// 永久删除文件 (不经过回收站).
///
/// ⚠️ 危险操作! 调用方必须确保用户已确认.
pub fn delete_permanently(path: &PathBuf) -> Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path).map_err(|e| AppError::Other(format!("删除目录失败: {e}")))?;
    } else {
        std::fs::remove_file(path).map_err(|e| AppError::Other(format!("删除文件失败: {e}")))?;
    }
    Ok(())
}

/// 复制文件路径到剪贴板。
///
/// 使用 PowerShell Set-Clipboard 实现，兼容 CLI 和 GUI 环境。
pub fn copy_path_to_clipboard(path: &Path) -> Result<()> {
    let path_str = path.to_string_lossy();
    let output = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-Command")
        .arg(format!(
            "Set-Clipboard -Value '{}'",
            path_str.replace('\'', "''")
        ))
        .output()
        .map_err(|e| AppError::Other(format!("执行 PowerShell 失败: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Other(format!(
            "复制到剪贴板失败: {}",
            stderr.trim()
        )));
    }
    Ok(())
}

/// 解析 .lnk 快捷方式的目标路径
pub fn resolve_shortcut(path: &Path) -> Result<PathBuf> {
    let output = std::process::Command::new("powershell")
        .arg("-Command")
        .arg(format!(
            "(New-Object -ComObject WScript.Shell).CreateShortcut('{}').TargetPath",
            path.to_string_lossy()
        ))
        .output()
        .map_err(|e| AppError::Other(format!("执行 PowerShell 失败: {e}")))?;

    if !output.status.success() {
        return Ok(path.to_path_buf());
    }

    let target_path = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if target_path.is_empty() {
        return Ok(path.to_path_buf());
    }

    Ok(PathBuf::from(target_path))
}

/// 通过 ShellExecuteW 启动 .lnk 快捷方式文件.
///
/// 与 `Command::new(path)` 直接启动 exe 不同, ShellExecuteW 会让 Windows
/// 解析 .lnk 文件的所有属性 (目标路径、工作目录、启动参数、窗口状态等),
/// 确保快捷方式按设计者意图正确启动.
#[cfg(windows)]
pub fn launch_lnk(lnk_path: &Path) -> Result<()> {
    let wide: Vec<u16> = lnk_path
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let result = unsafe {
        ShellExecuteW(
            Some(HWND::default()),
            w!("open"),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOW,
        )
    };

    // ShellExecuteW 返回值 <= 32 表示失败 (SE_ERR_* 错误码).
    // HINSTANCE 在内部是一个指针, 转换为 usize 后与 32 比较.
    let result_val = result.0 as usize;
    if result_val <= 32 {
        return Err(AppError::Other(format!(
            "ShellExecuteW 启动快捷方式失败, 错误码: {}",
            result_val
        )));
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn launch_lnk(_lnk_path: &Path) -> Result<()> {
    Err(AppError::Other("非 Windows 平台不支持 .lnk 快捷方式".to_string()))
}

/// 从 SearchResult 派发
pub fn launch_str(item: &crate::search_engine::models::SearchResult) -> Result<()> {
    use crate::search_engine::models::SearchAction;
    match &item.action {
        SearchAction::Launch(path) => {
            // 对于 .lnk 快捷方式, 使用 ShellExecuteW 而非 Command::new,
            // 确保 Windows 正确解析快捷方式的所有属性 (目标/工作目录/参数).
            let path_buf = std::path::PathBuf::from(path);
            let is_lnk = path_buf
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("lnk"))
                .unwrap_or(false);
            if is_lnk {
                #[cfg(windows)]
                launch_lnk(&path_buf)?;
                #[cfg(not(windows))]
                launch(path, &[])?;
            } else {
                launch(path, &[])?;
            }
        }
        SearchAction::Open(path) => {
            open_path(&PathBuf::from(path))?;
        }
        SearchAction::Run { command, args } => {
            launch(command, args)?;
        }
        SearchAction::Navigate(path) => {
            open_path(&PathBuf::from(path))?;
        }
    }
    Ok(())
}

/// 读取 Windows 文件关联 (HKCR), 返回 { 扩展名 → 关联的可执行文件路径 } 映射.
///
/// 步骤:
/// 1. 打开 HKEY_CLASSES_ROOT, 枚举所有以 "." 开头的子键 (文件扩展名).
/// 2. 读取扩展名键的默认值 → ProgId (如 ".txt" → "txtfile").
/// 3. 打开 HKCR\{ProgId}\shell\open\command, 读取默认值 → 命令行字符串.
/// 4. 从命令行字符串中提取可执行文件路径 (去掉引号和参数).
/// 5. 返回 HashMap, key 为扩展名 (含点, 如 ".txt"), value 为 exe 路径.
///
/// 失败时返回空 HashMap (不 panic).
#[cfg(windows)]
pub fn get_file_associations() -> std::collections::HashMap<String, PathBuf> {
    use std::collections::HashMap;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegOpenKeyExW,
        HKEY, HKEY_CLASSES_ROOT, KEY_READ, KEY_WOW64_64KEY,
    };

    let mut result: HashMap<String, PathBuf> = HashMap::new();

    // 打开 HKEY_CLASSES_ROOT
    let mut hkcr = HKEY::default();
    let rc = unsafe {
        RegOpenKeyExW(
            HKEY_CLASSES_ROOT,
            w!(""),
            None,
            KEY_READ | KEY_WOW64_64KEY,
            &mut hkcr,
        )
    };
    if rc != ERROR_SUCCESS || hkcr.is_invalid() {
        // 尝试不带 WOW64 标志
        let rc2 = unsafe {
            RegOpenKeyExW(HKEY_CLASSES_ROOT, w!(""), None, KEY_READ, &mut hkcr)
        };
        if rc2 != ERROR_SUCCESS || hkcr.is_invalid() {
            return result;
        }
    }

    // 缓冲区: 枚举扩展名键名
    let mut name_buf: Vec<u16> = vec![0u16; 260];
    let mut idx = 0u32;

    loop {
        let mut name_len = name_buf.len() as u32;
        let rc = unsafe {
            RegEnumKeyExW(
                hkcr,
                idx,
                Some(windows::core::PWSTR::from_raw(name_buf.as_mut_ptr())),
                &mut name_len,
                None,
                None,
                None,
                None,
            )
        };
        if rc != ERROR_SUCCESS {
            break;
        }

        let name_slice = &name_buf[..name_len as usize];
        // 只处理以 "." 开头的扩展名键, 且不是单独的 "." 或 ".."
        if name_slice.first() == Some(&u16::from(b'.')) && name_len > 1 {
            let ext_name = String::from_utf16_lossy(name_slice);

            // 读取扩展名的默认值 → ProgId
            if let Some(prog_id) = read_default_value_wide(hkcr, &ext_name) {
                if !prog_id.is_empty() {
                    let command_key = format!("{}\\shell\\open\\command", prog_id);
                    if let Some(cmd) = read_default_value_full_path(&command_key) {
                        if let Some(exe_path) = extract_exe_from_command(&cmd) {
                            result.insert(ext_name, exe_path);
                        }
                    }
                }
            }
        }

        idx += 1;
    }

    let _ = unsafe { RegCloseKey(hkcr) };
    result
}

/// 读取指定子键的默认值 ("") 为 UTF-16 字符串.
#[cfg(windows)]
fn read_default_value_wide(parent: HKEY, subkey: &str) -> Option<String> {
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, KEY_READ, REG_NONE, REG_SZ,
    };
    use windows::core::PCWSTR;

    let subkey_wide: Vec<u16> = OsStr::new(subkey).encode_wide().chain(Some(0)).collect();
    let mut key = HKEY::default();
    let rc = unsafe {
        RegOpenKeyExW(parent, PCWSTR::from_raw(subkey_wide.as_ptr()), None, KEY_READ, &mut key)
    };
    if rc != ERROR_SUCCESS || key.is_invalid() {
        return None;
    }

    let mut data: Vec<u8> = vec![0u8; 1024];
    let mut data_len = data.len() as u32;
    let mut val_type = REG_NONE;

    let rc = unsafe {
        RegQueryValueExW(
            key,
            w!(""),
            None,
            Some(&mut val_type),
            Some(data.as_mut_ptr()),
            Some(&mut data_len),
        )
    };

    let _ = unsafe { RegCloseKey(key) };

    if rc != ERROR_SUCCESS || data_len == 0 {
        return None;
    }

    // REG_SZ 是 UTF-16LE 编码, 以双 null 结尾
    if val_type == REG_SZ {
        let words = data[..data_len as usize]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect::<Vec<u16>>();
        // 去掉尾部 null
        let trimmed: Vec<u16> = words
            .split(|&w| w == 0)
            .next()
            .unwrap_or(&[])
            .to_vec();
        Some(String::from_utf16_lossy(&trimmed))
    } else {
        None
    }
}

/// 读取完整注册表路径的默认值 (用于 HKCR\ProgId\shell\open\command).
#[cfg(windows)]
fn read_default_value_full_path(subkey_full: &str) -> Option<String> {
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CLASSES_ROOT, KEY_READ, REG_NONE, REG_SZ,
    };
    use windows::core::PCWSTR;

    let subkey_wide: Vec<u16> = OsStr::new(subkey_full).encode_wide().chain(Some(0)).collect();
    let mut key = HKEY::default();
    let rc = unsafe {
        RegOpenKeyExW(HKEY_CLASSES_ROOT, PCWSTR::from_raw(subkey_wide.as_ptr()), None, KEY_READ, &mut key)
    };
    if rc != ERROR_SUCCESS || key.is_invalid() {
        return None;
    }

    let mut data: Vec<u8> = vec![0u8; 2048];
    let mut data_len = data.len() as u32;
    let mut val_type = REG_NONE;

    let rc = unsafe {
        RegQueryValueExW(
            key,
            w!(""),
            None,
            Some(&mut val_type),
            Some(data.as_mut_ptr()),
            Some(&mut data_len),
        )
    };

    let _ = unsafe { RegCloseKey(key) };

    if rc != ERROR_SUCCESS || data_len == 0 {
        return None;
    }

    if val_type == REG_SZ {
        let words = data[..data_len as usize]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect::<Vec<u16>>();
        let trimmed: Vec<u16> = words
            .split(|&w| w == 0)
            .next()
            .unwrap_or(&[])
            .to_vec();
        Some(String::from_utf16_lossy(&trimmed))
    } else {
        None
    }
}

/// 从命令行字符串中提取可执行文件路径.
///
/// 例:
/// - `"C:\Program Files\Chrome\chrome.exe" --%1` → `C:\Program Files\Chrome\chrome.exe`
/// - `C:\Windows\notepad.exe %1` → `C:\Windows\notepad.exe`
/// - `"C:\Apps\My App\app.exe"` → `C:\Apps\My App\app.exe`
fn extract_exe_from_command(cmd: &str) -> Option<PathBuf> {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        return None;
    }

    let exe = if let Some(inner) = cmd.strip_prefix('"') {
        // 引号内的整个路径
        let end = inner.find('"')?;
        &inner[..end]
    } else {
        // 第一个空格前的部分
        cmd.split_whitespace().next()?
    };

    let path = PathBuf::from(exe);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

#[cfg(not(windows))]
pub fn get_file_associations() -> std::collections::HashMap<String, PathBuf> {
    std::collections::HashMap::new()
}
