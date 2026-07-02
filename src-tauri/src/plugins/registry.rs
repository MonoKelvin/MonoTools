use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::models::plugin::PluginManifest;
use crate::models::search::SearchResult;

/// 插件注册表 - 管理所有插件的注册项
pub struct PluginRegistry {
    search_providers: Arc<RwLock<HashMap<String, SearchProviderEntry>>>,
    commands: Arc<RwLock<HashMap<String, CommandEntry>>>,
    views: Arc<RwLock<HashMap<String, ViewEntry>>>,
    themes: Arc<RwLock<HashMap<String, ThemeEntry>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            search_providers: Arc::new(RwLock::new(HashMap::new())),
            commands: Arc::new(RwLock::new(HashMap::new())),
            views: Arc::new(RwLock::new(HashMap::new())),
            themes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册搜索提供者
    pub async fn register_search_provider(&self, plugin_id: String, provider: SearchProviderEntry) {
        let mut providers = self.search_providers.write().await;
        providers.insert(plugin_id.clone(), provider);
        tracing::info!("Registered search provider: {}", plugin_id);
    }

    /// 注销搜索提供者
    pub async fn unregister_search_provider(&self, plugin_id: &str) {
        let mut providers = self.search_providers.write().await;
        providers.remove(plugin_id);
        tracing::info!("Unregistered search provider: {}", plugin_id);
    }

    /// 注册命令
    pub async fn register_command(&self, plugin_id: String, command: CommandEntry) {
        let mut commands = self.commands.write().await;
        let key = format!("{}:{}", plugin_id, command.id);
        commands.insert(key.clone(), command);
        tracing::info!("Registered command: {}", key);
    }

    /// 注销命令
    pub async fn unregister_command(&self, plugin_id: &str, command_id: &str) {
        let mut commands = self.commands.write().await;
        let key = format!("{}:{}", plugin_id, command_id);
        commands.remove(&key);
        tracing::info!("Unregistered command: {}", key);
    }

    /// 注册视图
    pub async fn register_view(&self, plugin_id: String, route: String, view: ViewEntry) {
        let mut views = self.views.write().await;
        let key = format!("{}:{}", plugin_id, route);
        views.insert(key.clone(), view);
        tracing::info!("Registered view: {}", key);
    }

    /// 注册主题
    pub async fn register_theme(&self, plugin_id: String, theme: ThemeEntry) {
        let mut themes = self.themes.write().await;
        themes.insert(plugin_id.clone(), theme);
        tracing::info!("Registered theme: {}", plugin_id);
    }

    /// 获取所有搜索提供者
    pub async fn get_search_providers(&self) -> Vec<(String, SearchProviderEntry)> {
        let providers = self.search_providers.read().await;
        providers.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// 获取所有命令
    pub async fn get_commands(&self) -> Vec<(String, CommandEntry)> {
        let commands = self.commands.read().await;
        commands.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// 清空所有注册项（用于热重载）
    pub async fn clear(&self) {
        let mut providers = self.search_providers.write().await;
        providers.clear();

        let mut commands = self.commands.write().await;
        commands.clear();

        let mut views = self.views.write().await;
        views.clear();

        let mut themes = self.themes.write().await;
        themes.clear();
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 搜索提供者条目
#[derive(Debug, Clone)]
pub struct SearchProviderEntry {
    pub manifest: PluginManifest,
    pub triggers: Vec<String>,
    pub priority: u32,
}

/// 命令条目
#[derive(Debug, Clone)]
pub struct CommandEntry {
    pub manifest: PluginManifest,
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub shortcut: Option<String>,
}

/// 视图条目
#[derive(Debug, Clone)]
pub struct ViewEntry {
    pub manifest: PluginManifest,
    pub route: String,
    pub component_path: String,
}

/// 主题条目
#[derive(Debug, Clone)]
pub struct ThemeEntry {
    pub manifest: PluginManifest,
    pub css_path: std::path::PathBuf,
}
