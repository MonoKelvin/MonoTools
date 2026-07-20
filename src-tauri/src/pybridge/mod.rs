//! PyBridge - 通用 Rust-Python 桥接模块
//!
//! 提供 Rust 与 Python 之间的通信基础设施，支持注册多个 Python 服务。
//! 设计为完全独立：通过 feature flag "pybridge" 控制编译，删除本目录不影响其他功能。
//!
//! # 架构
//!
//! ```text
//! Rust 侧                    Python 侧
//! ┌──────────────┐         ┌──────────────┐
//! │ PyBridge     │  stdio  │ PyBridge     │
//! │ Service      │◄────────►│ Server       │
//! │ Registry     │         │ (JSON-RPC)   │
//! └──────┬───────┘         └──────┬───────┘
//!        │                        │
//!        ▼                        ▼
//!   各业务模块             各业务服务
//! (recommend, ...)     (recommend, ...)
//! ```
//!
//! # 用法
//!
//! ```rust,ignore
//! use pybridge::PyBridge;
//!
//! let bridge = PyBridge::new(config);
//! bridge.register_service("recommend", recommend_service);
//! bridge.start().await?;
//! ```

pub mod config;
pub mod types;
pub mod process;
pub mod jsonrpc;
pub mod registry;

pub use config::PyBridgeConfig;
pub use types::{PyBridgeError, PyBridgeResult, ServiceHandle};
pub use registry::ServiceRegistry;
pub use process::PythonProcess;

use std::sync::Arc;
use tokio::sync::RwLock;

/// PyBridge 主入口 - 通用 Rust-Python 桥接器
///
/// 负责管理 Python 子进程生命周期、服务注册、JSON-RPC 通信。
/// 完全独立：删除本模块或禁用 feature 后不影响其他代码。
pub struct PyBridge {
    config: PyBridgeConfig,
    process: Arc<RwLock<Option<PythonProcess>>>,
    registry: Arc<ServiceRegistry>,
    started: std::sync::atomic::AtomicBool,
}

impl PyBridge {
    /// 创建新的 PyBridge 实例
    pub fn new(config: PyBridgeConfig) -> Self {
        Self {
            config,
            process: Arc::new(RwLock::new(None)),
            registry: Arc::new(ServiceRegistry::new()),
            started: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// 获取配置
    pub fn config(&self) -> &PyBridgeConfig {
        &self.config
    }

    /// 获取服务注册表
    pub fn registry(&self) -> &ServiceRegistry {
        &self.registry
    }

    /// 检查桥接器是否已启动
    pub fn is_started(&self) -> bool {
        self.started.load(std::sync::atomic::Ordering::Acquire)
    }

    /// 启动 Python 桥接进程
    pub async fn start(&self) -> PyBridgeResult<()> {
        if !self.config.enabled {
            log::info!("[pybridge] 未启用，跳过启动");
            return Ok(());
        }

        if self.started.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(());
        }

        log::info!("[pybridge] 正在启动 Python 桥接进程...");

        let process = PythonProcess::start(&self.config).await?;

        *self.process.write().await = Some(process);
        self.started
            .store(true, std::sync::atomic::Ordering::Release);

        log::info!("[pybridge] Python 桥接进程已启动");
        Ok(())
    }

    /// 停止 Python 桥接进程
    pub async fn stop(&self) {
        if !self.started.swap(false, std::sync::atomic::Ordering::AcqRel) {
            return;
        }

        log::info!("[pybridge] 正在停止 Python 桥接进程...");

        let mut process_guard = self.process.write().await;
        if let Some(mut proc) = process_guard.take() {
            proc.stop().await;
        }

        log::info!("[pybridge] Python 桥接进程已停止");
    }

    /// 发送 JSON-RPC 请求并等待响应
    pub async fn call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> PyBridgeResult<serde_json::Value> {
        if !self.config.enabled {
            return Err(PyBridgeError::NotEnabled);
        }

        let process_guard = self.process.read().await;
        let process = process_guard
            .as_ref()
            .ok_or(PyBridgeError::NotStarted)?;

        process.call(method, params).await
    }
}

impl Drop for PyBridge {
    fn drop(&mut self) {
        if self.started.load(std::sync::atomic::Ordering::Acquire) {
            log::warn!("[pybridge] PyBridge dropped without explicit stop");
        }
    }
}

/// 初始化 PyBridge 模块 - 独立模块注册入口
///
/// 在 Tauri 的 setup 阶段调用，负责：
/// 1. 创建 PyBridge 实例
/// 2. 通过 app.manage() 注册服务状态
///
/// # 独立性
/// 删除本模块后，只需移除调用此函数的代码即可。
pub fn init<R: tauri::Runtime>(app: &tauri::AppHandle<R>, config: PyBridgeConfig) {
    use tauri::Manager;
    let bridge = Arc::new(PyBridge::new(config));
    app.manage(bridge);
    log::info!("[pybridge] 模块初始化完成");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_bridge() {
        let config = PyBridgeConfig::default();
        let bridge = PyBridge::new(config);
        assert_eq!(bridge.is_started(), false);
        assert_eq!(bridge.config().enabled, false);
    }

    #[test]
    fn test_bridge_registry() {
        let config = PyBridgeConfig::default();
        let bridge = PyBridge::new(config);

        assert!(!bridge.registry().has_service("test"));

        bridge.registry().register(ServiceHandle {
            name: "test".to_string(),
            version: "1.0".to_string(),
            description: "Test service".to_string(),
        });

        assert!(bridge.registry().has_service("test"));
        let svc = bridge.registry().get_service("test").unwrap();
        assert_eq!(svc.name, "test");
        assert_eq!(svc.version, "1.0");
    }

    #[tokio::test]
    async fn test_bridge_not_enabled_start() {
        let config = PyBridgeConfig::default();
        let bridge = PyBridge::new(config);

        // 未启用的情况下 start 应该直接返回 Ok
        let result = bridge.start().await;
        assert!(result.is_ok());
        assert_eq!(bridge.is_started(), false);
    }

    #[tokio::test]
    async fn test_bridge_call_not_enabled() {
        let config = PyBridgeConfig::default();
        let bridge = PyBridge::new(config);

        let result = bridge
            .call("test.method", serde_json::json!({}))
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            PyBridgeError::NotEnabled => {}
            _ => panic!("Expected NotEnabled error"),
        }
    }
}
