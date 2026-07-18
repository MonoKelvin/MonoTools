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
    /// 中文名称的拼音首字母 (小写, 无分隔). 例: "微信" → "wx".
    /// 用于拼音搜索: 用户输入 "wj" 能命中 "微信".
    /// 无中文字符时为 None (不存储, 节省空间).
    pub pinyin_initials: Option<String>,
    /// 中文名称的完整拼音 (小写, 无空格无声调). 例: "微信" → "weixin".
    /// 用于拼音全拼搜索: 用户输入 "weixin" 能命中 "微信".
    pub pinyin_full: Option<String>,
    /// PE 文件版本号 (从 FileVersion 资源提取). 提取失败为 None.
    pub version: Option<String>,
    /// 该应用关联的文件扩展名列表 (从 HKCR 文件关联读取).
    /// 例: [".psd", ".ai"] 表示该应用可打开这些文件类型.
    pub file_types: Vec<String>,
}
