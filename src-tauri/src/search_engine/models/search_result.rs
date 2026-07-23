use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum SearchCategory {
    Apps,
    Files,
    Commands,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ResultType {
    SystemApp,
    UserApp,
    UwpApp,
    Directory,
    Document,
    Image,
    Svg,
    Video,
    Audio,
    Executable,
    StaticLib,
    DynamicLib,
    Archive,
    Shortcut,
    Html,
    Font,
    Config,
    Code,
    OtherFile,
    Command,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchOptions {
    pub categories: Vec<SearchCategory>,
    pub limit: u32,
    pub include_hidden: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            categories: vec![SearchCategory::All],
            limit: 20,
            include_hidden: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SearchAction {
    #[serde(rename = "launch")]
    Launch(String),
    #[serde(rename = "open")]
    Open(String),
    #[serde(rename = "run")]
    Run { command: String, args: Vec<String> },
    #[serde(rename = "navigate")]
    Navigate(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    /// 主副标题: 常规文件显示**绝对路径**; 应用为空字符串; 命令显示"command args".
    pub subtitle: String,
    /// 次级元信息 (右侧灰色文本): 文件显示人类可读大小, 其他类型可空.
    /// 与 subtitle 解耦, 让"路径"是路径, "大小"是大小, 视觉不混淆.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<String>,
    pub icon: Option<String>,
    pub category: SearchCategory,
    pub result_type: ResultType,
    pub action: SearchAction,
    pub score: f32,
    /// 文件大小 (字节), 仅文件类结果有意义.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
    /// 修改时间 (Unix timestamp), 仅文件类结果有意义.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<i64>,
    /// 启动次数, 仅应用类结果有意义.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_count: Option<u32>,
}
