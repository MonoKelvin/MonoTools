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
#[serde(rename_all = "lowercase")]
pub enum ResultType {
    SystemApp,
    UserApp,
    UwpApp,
    Directory,
    Document,
    Image,
    Video,
    Audio,
    Executable,
    Archive,
    OtherFile,
    Command,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchOptions {
    pub categories: Vec<SearchCategory>,
    pub max_results: u32,
    pub include_hidden: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            categories: vec![SearchCategory::All],
            max_results: 20,
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
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub icon: Option<String>,
    pub category: SearchCategory,
    pub result_type: ResultType,
    pub action: SearchAction,
    pub score: f32,
}
