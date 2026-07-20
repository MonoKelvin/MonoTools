//! PyBridge 配置

use serde::{Deserialize, Serialize};

/// PyBridge 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PyBridgeConfig {
    /// 是否启用 Python 桥接
    pub enabled: bool,

    /// Python 解释器路径
    pub python_path: String,

    /// Python 服务脚本路径
    pub script_path: String,

    /// 启动超时时间（毫秒）
    pub startup_timeout_ms: u64,

    /// 请求超时时间（毫秒）
    pub request_timeout_ms: u64,

    /// 最大重启次数
    pub max_restarts: u32,

    /// 健康检查间隔（秒）
    pub health_check_interval_secs: u64,
}

impl Default for PyBridgeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            python_path: "python".to_string(),
            script_path: "python/pybridge/server.py".to_string(),
            startup_timeout_ms: 5000,
            request_timeout_ms: 500,
            max_restarts: 3,
            health_check_interval_secs: 30,
        }
    }
}

impl PyBridgeConfig {
    /// 创建启用的配置
    pub fn enabled(python_path: impl Into<String>, script_path: impl Into<String>) -> Self {
        Self {
            enabled: true,
            python_path: python_path.into(),
            script_path: script_path.into(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PyBridgeConfig::default();
        assert_eq!(config.enabled, false);
        assert_eq!(config.python_path, "python");
        assert_eq!(config.startup_timeout_ms, 5000);
        assert_eq!(config.request_timeout_ms, 500);
        assert_eq!(config.max_restarts, 3);
        assert_eq!(config.health_check_interval_secs, 30);
    }

    #[test]
    fn test_enabled_config() {
        let config = PyBridgeConfig::enabled("/usr/bin/python3", "/path/to/server.py");
        assert_eq!(config.enabled, true);
        assert_eq!(config.python_path, "/usr/bin/python3");
        assert_eq!(config.script_path, "/path/to/server.py");
    }

    #[test]
    fn test_config_serialization() {
        let config = PyBridgeConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: PyBridgeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.enabled, config.enabled);
        assert_eq!(parsed.python_path, config.python_path);
        assert_eq!(parsed.request_timeout_ms, config.request_timeout_ms);
    }
}
