use std::sync::Arc;
use tauri::AppHandle;
use tauri::Manager;
use serde_json::Value;
use crate::models::search::SearchResult;

/// 插件 API - 提供给插件使用的接口
pub struct PluginApi {
    app_handle: AppHandle,
    plugin_id: String,
}

impl PluginApi {
    pub fn new(app_handle: AppHandle, plugin_id: String) -> Self {
        Self {
            app_handle,
            plugin_id,
        }
    }

    /// 获取插件 ID
    pub fn id(&self) -> &str {
        &self.plugin_id
    }

    /// 发送事件到前端
    pub async fn emit(&self, event: &str, payload: Value) -> Result<(), String> {
        // 暂时禁用，等待 Tauri 2.0 API 确认
        // self.app_handle
        //     .emit(format!("plugin:{}:{}", self.plugin_id, event), payload)
        //     .map_err(|e| e.to_string())
        Ok(())
    }

    /// 获取配置
    pub async fn get_config(&self, key: Option<&str>) -> Result<Value, String> {
        // TODO: 从配置存储中读取
        Ok(Value::Null)
    }

    /// 设置配置
    pub async fn set_config(&self, key: &str, value: Value) -> Result<(), String> {
        // TODO: 写入配置存储
        Ok(())
    }

    /// 记录日志
    pub fn log(&self, level: &str, message: &str) {
        // 手动解析日志级别
        let log_level = match level.to_lowercase().as_str() {
            "error" => tracing::Level::ERROR,
            "warn" => tracing::Level::WARN,
            "info" => tracing::Level::INFO,
            "debug" => tracing::Level::DEBUG,
            "trace" => tracing::Level::TRACE,
            _ => tracing::Level::INFO,
        };
        match log_level {
            tracing::Level::ERROR => tracing::error!("{}", message),
            tracing::Level::WARN => tracing::warn!("{}", message),
            tracing::Level::INFO => tracing::info!("{}", message),
            tracing::Level::DEBUG => tracing::debug!("{}", message),
            tracing::Level::TRACE => tracing::trace!("{}", message),
        }
    }

    /// 注册命令（供插件在运行时动态注册）
    pub async fn register_command(&self, _command_id: &str, _handler: Arc<dyn Fn(Value) -> Result<Value, String> + Send + Sync>) -> Result<(), String> {
        // TODO: 动态注册命令到命令总线
        Ok(())
    }

    /// 注销命令
    pub async fn unregister_command(&self, _command_id: &str) -> Result<(), String> {
        // TODO: 从命令总线注销
        Ok(())
    }
}

/// 插件上下文
pub struct PluginContext {
    pub api: PluginApi,
}

impl PluginContext {
    pub fn new(app_handle: AppHandle, plugin_id: String) -> Self {
        Self {
            api: PluginApi::new(app_handle, plugin_id),
        }
    }

    pub fn api(&self) -> &PluginApi {
        &self.api
    }
}

/// 插件生命周期钩子
#[async_trait::async_trait]
pub trait PluginHooks: Send + Sync {
    /// 插件激活
    async fn on_activate(&self, _ctx: &PluginContext) -> anyhow::Result<()> {
        Ok(())
    }

    /// 插件停用
    async fn on_deactivate(&self, _ctx: &PluginContext) -> anyhow::Result<()> {
        Ok(())
    }

    /// 搜索处理
    async fn on_search(&self, _query: &str) -> anyhow::Result<Vec<SearchResult>> {
        Ok(vec![])
    }

    /// 执行结果处理
    async fn on_execute(&self, _item: &SearchResult) -> anyhow::Result<Value> {
        Ok(Value::Null)
    }

    /// 配置变更处理
    async fn on_config_change(&self, _config: Value) -> anyhow::Result<()> {
        Ok(())
    }
}

/// 空实现（用于不实现所有钩子的插件）
pub struct NoopHooks;

#[async_trait::async_trait]
impl PluginHooks for NoopHooks {}
