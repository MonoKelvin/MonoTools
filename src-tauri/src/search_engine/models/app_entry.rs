use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    /// 路径: 普通应用为 exe 路径; 特殊快捷方式为 .lnk 文件路径
    pub path: PathBuf,
    pub icon_path: Option<PathBuf>,
    pub category: String,
    pub last_launched: Option<i64>,
    pub launch_count: u32,
    /// 唯一短键，用于排序
    pub alias: Option<String>,
    /// 是否为特殊快捷方式（不指向真实文件的 .lnk，如运行、控制面板等）
    pub is_special_shortcut: bool,
    /// 特殊快捷方式的启动命令（如 explorer.exe）
    pub special_command: Option<String>,
    /// 特殊快捷方式的启动参数
    pub special_args: Option<Vec<String>>,
}
