//! Windows 特殊快捷方式处理
//!
//! 处理那些不指向真实文件的特殊 .lnk 快捷方式，比如：
//! - 运行 (Run) - `shell:::{2559a1f3-21d7-11d4-bdaf-00c04f60b9f0}`
//! - 控制面板 - `shell:::{5399E694-6CE5-4D6C-8FCE-1D8870FDCBA0}`
//! - 此电脑 - `shell:::{20D04FE0-3AEA-1069-A2D8-08002B30309D}`
//! - 回收站 - `shell:::{645FF040-5081-101B-9F08-00AA002F954E}`
//! - 网络 - `shell:::{F02C1A0D-BE21-4350-88B0-7367FC96EF3C}`
//! - 等等...
//!
//! 这些快捷方式的特点：
//! - TargetPath 为空或指向 CLSID / shell: 协议
//! - 文件系统中不存在目标文件
//! - 需要通过 ShellExecute 或 explorer.exe 来启动
//! - 有自己的系统图标

use std::path::{Path, PathBuf};

/// 特殊快捷方式的定义
pub struct SpecialShortcut {
    /// 匹配关键字（小写，用于匹配 .lnk 文件名或目标路径）
    pub match_keywords: &'static [&'static str],
    /// 显示名称
    pub display_name: &'static str,
    /// 启动命令（用于执行）
    pub launch_command: &'static str,
    /// 启动参数
    pub launch_args: &'static [&'static str],
    /// 图标资源 (dll 路径, 图标索引) - 用于提取系统图标
    pub icon_resource: &'static str,
    /// 图标索引
    pub icon_index: i32,
    /// 结果类型
    pub result_type: SpecialShortcutType,
}

/// 特殊快捷方式类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialShortcutType {
    /// 系统工具（运行、控制面板等）
    SystemTool,
    /// 系统文件夹（此电脑、回收站等）
    SystemFolder,
    /// 搜索
    Search,
}

/// 内置的特殊快捷方式列表
///
/// 匹配优先级：按顺序匹配，第一个匹配的生效。
pub const SPECIAL_SHORTCUTS: &[SpecialShortcut] = &[
    // === 运行 ===
    SpecialShortcut {
        match_keywords: &["run", "运行"],
        display_name: "运行",
        launch_command: "explorer.exe",
        launch_args: &["shell:::{2559a1f3-21d7-11d4-bdaf-00c04f60b9f0}"],
        icon_resource: "shell32.dll",
        icon_index: 24,
        result_type: SpecialShortcutType::SystemTool,
    },
    // === 控制面板 ===
    SpecialShortcut {
        match_keywords: &["control", "control panel", "控制面板"],
        display_name: "控制面板",
        launch_command: "explorer.exe",
        launch_args: &["shell:::{5399E694-6CE5-4D6C-8FCE-1D8870FDCBA0}"],
        icon_resource: "shell32.dll",
        icon_index: 21,
        result_type: SpecialShortcutType::SystemTool,
    },
    // === 此电脑 / 我的电脑 ===
    SpecialShortcut {
        match_keywords: &["my computer", "this pc", "此电脑", "我的电脑", "computer"],
        display_name: "此电脑",
        launch_command: "explorer.exe",
        launch_args: &["shell:::{20D04FE0-3AEA-1069-A2D8-08002B30309D}"],
        icon_resource: "imageres.dll",
        icon_index: 102,
        result_type: SpecialShortcutType::SystemFolder,
    },
    // === 回收站 ===
    SpecialShortcut {
        match_keywords: &["recycle", "recycle bin", "回收站", "垃圾回收站"],
        display_name: "回收站",
        launch_command: "explorer.exe",
        launch_args: &["shell:::{645FF040-5081-101B-9F08-00AA002F954E}"],
        icon_resource: "shell32.dll",
        icon_index: 31,
        result_type: SpecialShortcutType::SystemFolder,
    },
    // === 网络 ===
    SpecialShortcut {
        match_keywords: &["network", "网络", "网上邻居"],
        display_name: "网络",
        launch_command: "explorer.exe",
        launch_args: &["shell:::{F02C1A0D-BE21-4350-88B0-7367FC96EF3C}"],
        icon_resource: "shell32.dll",
        icon_index: 18,
        result_type: SpecialShortcutType::SystemFolder,
    },
    // === 搜索 ===
    SpecialShortcut {
        match_keywords: &["search", "搜索", "查找"],
        display_name: "搜索",
        launch_command: "explorer.exe",
        launch_args: &["search:query="],
        icon_resource: "shell32.dll",
        icon_index: 22,
        result_type: SpecialShortcutType::Search,
    },
    // === 任务视图 ===
    SpecialShortcut {
        match_keywords: &["task view", "任务视图", "taskview"],
        display_name: "任务视图",
        launch_command: "explorer.exe",
        launch_args: &["shell:::{3080F90E-D7AD-11D9-BD98-0000947B0257}"],
        icon_resource: "imageres.dll",
        icon_index: 118,
        result_type: SpecialShortcutType::SystemTool,
    },
    // === 快速访问 ===
    SpecialShortcut {
        match_keywords: &["quick access", "快速访问", "homegroup"],
        display_name: "快速访问",
        launch_command: "explorer.exe",
        launch_args: &["shell:::{679F8556-0641-422A-BD55-75FC09C83736}"],
        icon_resource: "imageres.dll",
        icon_index: 117,
        result_type: SpecialShortcutType::SystemFolder,
    },
    // === 日期和时间 ===
    SpecialShortcut {
        match_keywords: &["date and time", "日期和时间", "日期时间"],
        display_name: "日期和时间",
        launch_command: "control.exe",
        launch_args: &["timedate.cpl"],
        icon_resource: "shell32.dll",
        icon_index: 12,
        result_type: SpecialShortcutType::SystemTool,
    },
    // === 显示设置 ===
    SpecialShortcut {
        match_keywords: &["display", "显示设置", "屏幕设置"],
        display_name: "显示设置",
        launch_command: "explorer.exe",
        launch_args: &["ms-settings:display"],
        icon_resource: "imageres.dll",
        icon_index: 109,
        result_type: SpecialShortcutType::SystemTool,
    },
    // === 声音设置 ===
    SpecialShortcut {
        match_keywords: &["sound", "声音", "音量"],
        display_name: "声音设置",
        launch_command: "explorer.exe",
        launch_args: &["ms-settings:sound"],
        icon_resource: "mmsys.cpl",
        icon_index: 0,
        result_type: SpecialShortcutType::SystemTool,
    },
];

/// 检查一个 .lnk 文件是否是特殊快捷方式
///
/// 判断逻辑：
/// 1. 先看 LNK 文件名（不区分大小写）是否匹配关键字
/// 2. 再看目标路径是否包含 shell::: 或 CLSID 格式
pub fn is_special_shortcut(lnk_path: &Path, target_path: &Path) -> bool {
    // 检查目标路径是否是 shell: 协议或 CLSID 格式
    let target_str = target_path.to_string_lossy().to_lowercase();
    if target_str.contains("shell:::") || target_str.starts_with("::{") {
        return true;
    }
    if target_str.starts_with("ms-settings:") {
        return true;
    }

    // 检查文件名是否匹配关键字
    let file_name = lnk_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    for sc in SPECIAL_SHORTCUTS {
        for keyword in sc.match_keywords {
            if file_name.contains(&keyword.to_lowercase()) {
                return true;
            }
        }
    }

    false
}

/// 获取特殊快捷方式的信息
///
/// 先按文件名匹配，再按目标路径匹配。
pub fn get_special_shortcut(lnk_path: &Path, target_path: &Path) -> Option<&'static SpecialShortcut> {
    let file_name = lnk_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    let target_str = target_path.to_string_lossy().to_lowercase();

    // 1. 先按文件名精确匹配
    for sc in SPECIAL_SHORTCUTS {
        for keyword in sc.match_keywords {
            let kw = keyword.to_lowercase();
            if file_name == kw || file_name.contains(&kw) {
                return Some(sc);
            }
        }
    }

    // 2. 按目标路径特征匹配
    if target_str.contains("shell:::{2559a1f3") {
        return Some(&SPECIAL_SHORTCUTS[0]); // Run
    }
    if target_str.contains("shell:::{5399e694") || target_str.contains("control.exe") {
        return Some(&SPECIAL_SHORTCUTS[1]); // Control Panel
    }
    if target_str.contains("shell:::{20d04fe0") {
        return Some(&SPECIAL_SHORTCUTS[2]); // My Computer
    }
    if target_str.contains("shell:::{645ff040") {
        return Some(&SPECIAL_SHORTCUTS[3]); // Recycle Bin
    }
    if target_str.contains("shell:::{f02c1a0d") {
        return Some(&SPECIAL_SHORTCUTS[4]); // Network
    }
    if target_str.starts_with("search:") {
        return Some(&SPECIAL_SHORTCUTS[5]); // Search
    }
    if target_str.starts_with("ms-settings:") {
        // 通用设置类 - 用显示设置图标兜底
        return Some(&SPECIAL_SHORTCUTS[9]);
    }

    None
}

/// 执行特殊快捷方式
#[cfg(windows)]
pub fn launch_special_shortcut(sc: &SpecialShortcut) -> crate::core::error::Result<()> {
    use crate::core::config::paths;
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    let mut cmd = Command::new(sc.launch_command);
    for arg in sc.launch_args {
        cmd.arg(arg);
    }
    cmd.creation_flags(paths::CREATE_NO_WINDOW);

    cmd.spawn()
        .map(|_| ())
        .map_err(|e| crate::core::error::AppError::Other(format!("启动特殊快捷方式失败: {e}")))
}

#[cfg(not(windows))]
pub fn launch_special_shortcut(_sc: &SpecialShortcut) -> crate::core::error::Result<()> {
    Ok(())
}

/// 从系统 DLL 中提取图标资源的路径
///
/// 返回 system32 目录下的 DLL 完整路径。
pub fn get_system_icon_dll_path(dll_name: &str) -> PathBuf {
    let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
    PathBuf::from(&system_root).join("System32").join(dll_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_special_shortcut_by_name() {
        let lnk = PathBuf::from("C:\\Users\\test\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\Run.lnk");
        let target = PathBuf::from("");
        assert!(is_special_shortcut(&lnk, &target));
    }

    #[test]
    fn test_is_special_shortcut_by_target() {
        let lnk = PathBuf::from("C:\\test\\something.lnk");
        let target = PathBuf::from("shell:::{2559a1f3-21d7-11d4-bdaf-00c04f60b9f0}");
        assert!(is_special_shortcut(&lnk, &target));
    }

    #[test]
    fn test_get_special_shortcut_run() {
        let lnk = PathBuf::from("Run.lnk");
        let target = PathBuf::from("");
        let sc = get_special_shortcut(&lnk, &target);
        assert!(sc.is_some());
        assert_eq!(sc.unwrap().display_name, "运行");
    }

    #[test]
    fn test_normal_exe_not_special() {
        let lnk = PathBuf::from("Chrome.lnk");
        let target = PathBuf::from("C:\\Program Files\\Chrome\\chrome.exe");
        assert!(!is_special_shortcut(&lnk, &target));
    }
}
