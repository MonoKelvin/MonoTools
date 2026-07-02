use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::models::plugin::PluginManifest;
use crate::models::plugin::PluginInfo;
use crate::plugins::sandbox::PluginSandbox;
use crate::plugins::loader::PluginLoader;
use anyhow::Result;

/// 插件管理器
#[derive(Debug)]
pub struct PluginManager {
    plugins: Arc<RwLock<HashMap<String, PluginInfo>>>,
    builtin_plugins: Arc<RwLock<HashMap<String, PluginManifest>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            builtin_plugins: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 加载内置插件
    pub async fn load_builtin(&self, manifest: PluginManifest) -> Result<()> {
        let mut builtins = self.builtin_plugins.write().await;
        builtins.insert(manifest.id.clone(), manifest);
        Ok(())
    }

    /// 加载用户插件
    pub async fn load_plugin(&self, manifest: PluginManifest) -> Result<()> {
        // 校验权限
        Self::validate_permissions(&manifest)?;

        // 检查依赖
        let builtins = self.builtin_plugins.read().await.clone();
        Self::check_dependencies(&manifest, &builtins).await?;

        let info = PluginInfo {
            manifest: manifest.clone(),
            enabled: true,
            load_order: 0,
            installed_at: None,
        };

        let mut plugins = self.plugins.write().await;
        plugins.insert(manifest.id.clone(), info);

        Ok(())
    }

    /// 启用插件
    pub async fn enable_plugin(&self, id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        if let Some(info) = plugins.get_mut(id) {
            info.enabled = true;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Plugin not found: {}", id))
        }
    }

    /// 禁用插件
    pub async fn disable_plugin(&self, id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        if let Some(info) = plugins.get_mut(id) {
            info.enabled = false;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Plugin not found: {}", id))
        }
    }

    /// 列出所有插件
    pub async fn list_plugins(&self, enabled_only: bool, builtin_only: bool) -> Vec<PluginInfo> {
        let mut result = Vec::new();

        if builtin_only {
            let builtins = self.builtin_plugins.read().await;
            result.extend(builtins.values().map(|manifest| PluginInfo {
                manifest: manifest.clone(),
                enabled: true,
                load_order: 0,
                installed_at: None,
            }));
        } else {
            let plugins = self.plugins.read().await;
            result.extend(plugins.values().cloned().filter(|info| !enabled_only || info.enabled));
        }

        result
    }

    /// 校验插件权限
    fn validate_permissions(manifest: &PluginManifest) -> Result<()> {
        for perm in &manifest.permissions {
            // 核心权限只有内置插件可以申请
            if perm.starts_with("core:") && !manifest.is_builtin {
                return Err(anyhow::anyhow!("Forbidden permission: {}", perm));
            }
        }
        Ok(())
    }

    /// 检查依赖
    async fn check_dependencies(
        manifest: &PluginManifest,
        builtins: &HashMap<String, PluginManifest>,
    ) -> Result<()> {
        // TODO: 检查 MonoTools 版本依赖
        // TODO: 检查其他插件依赖
        Ok(())
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
