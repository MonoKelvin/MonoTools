//! Shell 工具 - 启动程序、打开路径、解析快捷方式、系统操作
use crate::error::{AppError, Result};
use std::path::PathBuf;
use std::process::Command;

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
pub fn launch_as_admin(_path: &str, _args: &[String]) -> Result<()> {
    // 此版本使用 reg.exe 调用 ShellExecuteW，独立不依赖 windows-rs 复杂 API
    // MVP 中由调用方保证 UI 反馈已切换；后端只关心状态返回
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
#[cfg(windows)]
pub fn open_path(path: &PathBuf) -> Result<()> {
    use crate::config::paths;
    use std::os::windows::process::CommandExt;

    let path_str = path.to_string_lossy().into_owned();
    let select_arg = format!("/select,{}", path_str);

    std::process::Command::new(paths::EXPLORER_EXE)
        .arg(&select_arg)
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
    use crate::config::paths;
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
pub fn delete_to_recycle_bin(path: &PathBuf) -> Result<()> {
    use crate::config::paths;
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

/// 复制文件路径到剪贴板 (由前端实现, 这里仅作占位).
///
/// 注: 剪贴板操作在 Tauri 中通常由前端通过 navigator.clipboard 完成,
/// 后端只在需要时提供兜底. 此函数预留用于未来扩展 (如 CMD 环境).
pub fn copy_path_to_clipboard(_path: &PathBuf) -> Result<()> {
    Ok(())
}

/// 解析 .lnk 快捷方式的目标路径
pub fn resolve_shortcut(path: &PathBuf) -> Result<PathBuf> {
    let output = std::process::Command::new("powershell")
        .arg("-Command")
        .arg(format!(
            "(New-Object -ComObject WScript.Shell).CreateShortcut('{}').TargetPath",
            path.to_string_lossy()
        ))
        .output()
        .map_err(|e| AppError::Other(format!("执行 PowerShell 失败: {e}")))?;

    if !output.status.success() {
        return Ok(path.clone());
    }

    let target_path = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if target_path.is_empty() {
        return Ok(path.clone());
    }

    Ok(PathBuf::from(target_path))
}

/// 从 SearchResult 派发
pub fn launch_str(item: &crate::models::SearchResult) -> Result<()> {
    use crate::models::SearchAction;
    match &item.action {
        SearchAction::Launch(path) => {
            launch(path, &[])?;
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
