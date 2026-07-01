use anyhow::Result;
use serde_json::Value;
use crate::commands::bus::{Command, CommandHandler, CommandContext};
use crate::plugins::manager::PluginManager;

pub struct PluginCommandHandler {
    plugin_manager: std::sync::Arc<tokio::sync::RwLock<PluginManager>>,
}

impl PluginCommandHandler {
    pub fn new(plugin_manager: std::sync::Arc<tokio::sync::RwLock<PluginManager>>) -> Self {
        Self { plugin_manager }
    }
}

#[async_trait::async_trait]
impl CommandHandler for PluginCommandHandler {
    async fn execute(&self, cmd: &Command, ctx: &CommandContext) -> Result<Value> {
        match cmd.action.as_str() {
            "list" => self.list_plugins(cmd).await,
            "install" => self.install_plugin(cmd).await,
            "uninstall" => self.uninstall_plugin(cmd).await,
            "enable" => self.enable_plugin(cmd).await,
            "disable" => self.disable_plugin(cmd).await,
            "reload" => self.reload_plugin(cmd).await,
            "config" => self.get_plugin_config(cmd).await,
            "logs" => self.get_plugin_logs(cmd).await,
            _ => Err(anyhow::anyhow!("Unknown plugin action: {}", cmd.action)),
        }
    }

    fn validate(&self, cmd: &Command) -> Result<()> {
        match cmd.action.as_str() {
            "uninstall" | "enable" | "disable" | "reload" | "get" | "delete" | "export" => {
                if !cmd.args.contains_key("id") {
                    return Err(anyhow::anyhow!("Missing required argument: id"));
                }
            }
            "install" => {
                if !cmd.args.contains_key("path") {
                    return Err(anyhow::anyhow!("Missing required argument: path"));
                }
            }
            "config" => {
                if !cmd.args.contains_key("id") {
                    return Err(anyhow::anyhow!("Missing required argument: id"));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl PluginCommandHandler {
    /// 列出插件
    async fn list_plugins(&self, cmd: &Command) -> Result<Value> {
        let enabled_only = cmd.flags.contains(&"enabled-only".to_string());
        let builtin_only = cmd.flags.contains(&"builtin-only".to_string());

        let manager = self.plugin_manager.read().await;
        let plugins = manager.list_plugins(enabled_only, builtin_only).await;

        let plugin_list: Vec<Value> = plugins.into_iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.manifest.id,
                    "name": p.manifest.name,
                    "version": p.manifest.version,
                    "description": p.manifest.description,
                    "type": p.manifest.plugin_type,
                    "enabled": p.enabled,
                    "builtin": p.manifest.is_builtin,
                    "author": p.manifest.author
                })
            })
            .collect();

        Ok(serde_json::json!({
            "plugins": plugin_list,
            "total": plugin_list.len()
        }))
    }

    /// 安装插件
    async fn install_plugin(&self, cmd: &Command) -> Result<Value> {
        let path = cmd.args.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing path argument"))?;

        // TODO: 实现插件安装
        // 1. 验证 plugin.json
        // 2. 校验权限
        // 3. 复制到 plugins/ 目录
        // 4. 加载插件

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Plugin installation from {} not fully implemented yet", path)
        }))
    }

    /// 卸载插件
    async fn uninstall_plugin(&self, cmd: &Command) -> Result<Value> {
        let id = cmd.args.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing id argument"))?;

        let _force = cmd.flags.contains(&"force".to_string());

        // TODO: 实现插件卸载
        // 1. 调用 deactivate 钩子
        // 2. 注销所有提供者、命令、视图
        // 3. 删除插件目录

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Plugin {} uninstall initiated", id)
        }))
    }

    /// 启用插件
    async fn enable_plugin(&self, cmd: &Command) -> Result<Value> {
        let id = cmd.args.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing id argument"))?;

        let manager = self.plugin_manager.write().await;
        manager.enable_plugin(id).await?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Plugin {} enabled", id)
        }))
    }

    /// 禁用插件
    async fn disable_plugin(&self, cmd: &Command) -> Result<Value> {
        let id = cmd.args.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing id argument"))?;

        let manager = self.plugin_manager.write().await;
        manager.disable_plugin(id).await?;

        Ok(serde_json::json!({
            "success": true,
            "message": format!("Plugin {} disabled", id)
        }))
    }

    /// 热重载插件
    async fn reload_plugin(&self, cmd: &Command) -> Result<Value> {
        let _id = cmd.args.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing id argument"))?;

        // TODO: 实现热重载
        // 1. 调用 deactivate
        // 2. 重新加载代码
        // 3. 调用 activate

        Ok(serde_json::json!({
            "success": true,
            "message": "Plugin reload not fully implemented yet"
        }))
    }

    /// 获取插件配置
    async fn get_plugin_config(&self, cmd: &Command) -> Result<Value> {
        let id = cmd.args.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing id argument"))?;

        // TODO: 从配置中读取插件配置

        Ok(serde_json::json!({
            "id": id,
            "config": {}
        }))
    }

    /// 获取插件日志
    async fn get_plugin_logs(&self, cmd: &Command) -> Result<Value> {
        let _id = cmd.args.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing id argument"))?;

        let tail = cmd.args.get("tail")
            .and_then(|v| v.as_u64())
            .unwrap_or(100);

        // TODO: 读取插件日志文件

        Ok(serde_json::json!({
            "logs": [],
            "tail": tail
        }))
    }
}

impl Default for PluginCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}
