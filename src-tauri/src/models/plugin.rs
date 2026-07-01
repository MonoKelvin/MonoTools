use serde::{Deserialize, Serialize};

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
    pub config_schema: Option<serde_json::Value>,
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
    pub installed_at: Option<chrono::DateTime<chrono::Utc>>,
}
