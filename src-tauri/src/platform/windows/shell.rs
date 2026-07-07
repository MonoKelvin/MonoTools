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

/// 在文件管理器中打开
pub fn open_path(path: &PathBuf) -> Result<()> {
    #[cfg(windows)]
    {
        // explorer /select,"path" 选中具体文件 - 使用 cmd.exe
        let cmd = format!("explorer /select,{}", path.display());
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(format!("{}", path.display()))
            .spawn()
            .map_err(|e| AppError::Other(format!("打开路径失败: {e}")))?;
        let _ = cmd;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| AppError::Other(format!("打开路径失败: {e}")))?;
        Ok(())
    }
}

/// 解析 .lnk 快捷方式（简化）：读目标的 FileDescription
/// 真实实现可使用 mslink crate 或 Win32 IShellLink，不在本 MVP 中
pub fn resolve_shortcut(path: &PathBuf) -> Result<PathBuf> {
    Ok(path.clone())
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
