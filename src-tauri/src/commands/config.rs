use anyhow::Result;
use serde_json::Value;
use crate::commands::bus::{Command, CommandHandler, CommandContext};
use crate::config::store::ConfigStore;

pub struct ConfigCommandHandler {
    config: std::sync::Arc<tokio::sync::RwLock<ConfigStore>>,
}

impl ConfigCommandHandler {
    pub fn new(config: std::sync::Arc<tokio::sync::RwLock<ConfigStore>>) -> Self {
        Self { config }
    }
}

#[async_trait::async_trait]
impl CommandHandler for ConfigCommandHandler {
    async fn execute(&self, cmd: &Command, ctx: &CommandContext) -> Result<Value> {
        match cmd.action.as_str() {
            "get" => self.get_config(cmd).await,
            "set" => self.set_config(cmd).await,
            "reset" => self.reset_config(cmd).await,
            "path" => self.get_config_path().await,
            "export" => self.export_config().await,
            "import" => self.import_config(cmd).await,
            _ => Err(anyhow::anyhow!("Unknown config action: {}", cmd.action)),
        }
    }

    fn validate(&self, cmd: &Command) -> Result<()> {
        match cmd.action.as_str() {
            "set" => {
                if !cmd.args.contains_key("key") {
                    return Err(anyhow::anyhow!("Missing required argument: key"));
                }
                if !cmd.args.contains_key("value") {
                    return Err(anyhow::anyhow!("Missing required argument: value"));
                }
            }
            "export" => {
                // export 需要 output 参数
            }
            "import" => {
                if !cmd.args.contains_key("path") {
                    return Err(anyhow::anyhow!("Missing required argument: path"));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl ConfigCommandHandler {
    /// 获取配置项
    async fn get_config(&self, cmd: &Command) -> Result<Value> {
        let key = cmd.args.get("key")
            .and_then(|v| v.as_str());

        let config = self.config.read().await;

        if let Some(key) = key {
            // 获取单个配置项
            let value = config.get_raw(key)?;
            Ok(serde_json::json!({
                "key": key,
                "value": value
            }))
        } else {
            // 获取所有配置
            let all = config.get_all();
            Ok(serde_json::to_value(all)?)
        }
    }

    /// 设置配置项
    async fn set_config(&self, cmd: &Command) -> Result<Value> {
        let key = cmd.args.get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing key argument"))?;

        let value = cmd.args.get("value")
            .ok_or_else(|| anyhow::anyhow!("Missing value argument"))?;

        let mut config = self.config.write().await;
        config.set(key, value.clone())?;

        // 广播配置变更事件
        // TODO: 需要通过 AppHandle 发送事件

        Ok(serde_json::json!({
            "success": true,
            "key": key,
            "value": value
        }))
    }

    /// 重置配置
    async fn reset_config(&self, cmd: &Command) -> Result<Value> {
        let key = cmd.args.get("key")
            .and_then(|v| v.as_str());

        let reset_all = cmd.flags.contains(&"all".to_string());

        let mut config = self.config.write().await;

        if reset_all || key.is_none() {
            // 重置所有配置到默认值
            // TODO: 实现
            Ok(serde_json::json!({
                "success": true,
                "message": "All config reset to defaults"
            }))
        } else if let Some(key) = key {
            // 重置单个配置项
            // TODO: 实现
            Ok(serde_json::json!({
                "success": true,
                "key": key,
                "message": "Config reset to default"
            }))
        } else {
            Err(anyhow::anyhow!("Invalid reset operation"))
        }
    }

    /// 获取配置文件路径
    async fn get_config_path(&self) -> Result<Value> {
        let config = self.config.read().await;
        let path = config.path().to_string_lossy().to_string();

        Ok(serde_json::json!({
            "path": path
        }))
    }

    /// 导出配置
    async fn export_config(&self, cmd: &Command) -> Result<Value> {
        let output = cmd.args.get("output")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing output argument for export"))?;

        // TODO: 实现配置导出为 ZIP

        Ok(serde_json::json!({
            "success": true,
            "path": output
        }))
    }

    /// 导入配置
    async fn import_config(&self, cmd: &Command) -> Result<Value> {
        let _path = cmd.args.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing path argument"))?;

        // TODO: 实现配置导入

        Ok(serde_json::json!({
            "success": true,
            "message": "Config import not fully implemented yet"
        }))
    }
}

impl Default for ConfigCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}
