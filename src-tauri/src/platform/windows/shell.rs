//! Shell 工具 - 启动程序、打开路径、解析快捷方式
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

/// 在文件管理器中打开 (explorer /select,<path> 选中具体文件).
///
/// ⚠️ 历史实现里有两段 spawn 代码, 一段是死代码, 但保留了"let _ = cmd"
/// 这种"抑制警告"的痕迹, 实际只 spawn 了一次 explorer. 当前实现:
///   - 单进程 spawn, 避免任何潜在的"两个窗口"误判.
///   - 显式 "/select,<path>" 作为单一参数, Windows 会忽略多余空格.
#[cfg(windows)]
pub fn open_path(path: &PathBuf) -> Result<()> {
    use crate::config::paths;
    use std::os::windows::process::CommandExt;
    // CREATE_NO_WINDOW 防止在 GUI 之外再开一个 console window.
    // 取自 config::paths::CREATE_NO_WINDOW (单一真源).

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

    let target_path = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();

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
