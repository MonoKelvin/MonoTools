use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub parent_path: String,
    pub ext: Option<String>,
    pub size: i64,
    pub created_at: i64,
    pub modified_at: i64,
    pub accessed_at: i64,
    pub is_dir: bool,
    pub ascii_sum: i32,
    pub pinyin: Option<String>,
    pub priority: i32,
    pub volume: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub exe_path: String,
    pub icon_path: Option<String>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    pub pinyin: Option<String>,
    pub launch_count: u32,
    pub last_accessed: Option<i64>,
    pub source: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<String>,
    pub source: String,
    pub score: f32,
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub raw: String,
    pub prefix: Option<String>,
    pub tokens: Vec<String>,
    pub is_empty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSnapshot {
    pub id: String,
    pub exe_path: String,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub window_title: String,
    pub window_rect: WindowRect,
    pub window_state: WindowState,
    pub launch_order: u32,
    pub launch_delay_ms: u64,
    pub require_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_restored_at: Option<DateTime<Utc>>,
    pub apps: Vec<AppSnapshot>,
    pub auto_start: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub plugin_type: String,
    pub entry: PluginEntry,
    pub permissions: Vec<String>,
    pub capabilities: PluginCapabilities,
    pub hooks: PluginHooks,
    pub config_schema: Option<Value>,
    pub dependencies: PluginDependencies,
    pub resources: PluginResources,
    pub is_builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    pub frontend: Option<String>,
    pub backend: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginCapabilities {
    pub search: Option<SearchCapability>,
    pub commands: Vec<CommandCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchCapability {
    pub enabled: bool,
    pub triggers: Vec<String>,
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandCapability {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub shortcut: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginHooks {
    pub activate: Option<String>,
    pub deactivate: Option<String>,
    pub search: Option<String>,
    pub execute: Option<String>,
    pub config_change: Option<String>,
    pub theme_change: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginDependencies {
    pub monotools: Option<String>,
    pub plugins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginResources {
    pub memory_limit: Option<String>,
    pub cpu_limit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub manifest: PluginManifest,
    pub enabled: bool,
    pub load_order: u32,
    pub installed_at: Option<DateTime<Utc>>,
}
