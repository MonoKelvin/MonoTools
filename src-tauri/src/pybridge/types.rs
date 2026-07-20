//! PyBridge 类型定义

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// PyBridge 错误类型
#[derive(Debug, Error)]
pub enum PyBridgeError {
    #[error("PyBridge 未启用")]
    NotEnabled,

    #[error("PyBridge 未启动")]
    NotStarted,

    #[error("Python 进程启动失败: {0}")]
    StartFailed(String),

    #[error("请求超时")]
    Timeout,

    #[error("JSON-RPC 错误: code={code}, message={message}")]
    RpcError { code: i64, message: String },

    #[error("序列化错误: {0}")]
    Serialization(String),

    #[error("IO 错误: {0}")]
    Io(String),

    #[error("服务未找到: {0}")]
    ServiceNotFound(String),

    #[error("未知错误: {0}")]
    Other(String),
}

impl From<std::io::Error> for PyBridgeError {
    fn from(e: std::io::Error) -> Self {
        PyBridgeError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for PyBridgeError {
    fn from(e: serde_json::Error) -> Self {
        PyBridgeError::Serialization(e.to_string())
    }
}

/// PyBridge 结果类型
pub type PyBridgeResult<T> = Result<T, PyBridgeError>;

/// 服务句柄 - 注册到 Python 侧的服务信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHandle {
    /// 服务名称
    pub name: String,
    /// 服务版本
    pub version: String,
    /// 服务描述
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_handle() {
        let handle = ServiceHandle {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test service".to_string(),
        };
        assert_eq!(handle.name, "test");
        assert_eq!(handle.version, "1.0.0");
    }

    #[test]
    fn test_service_handle_serialization() {
        let handle = ServiceHandle {
            name: "recommend".to_string(),
            version: "2.0".to_string(),
            description: "Recommendation service".to_string(),
        };
        let json = serde_json::to_string(&handle).unwrap();
        let parsed: ServiceHandle = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, handle.name);
        assert_eq!(parsed.version, handle.version);
        assert_eq!(parsed.description, handle.description);
    }

    #[test]
    fn test_error_display() {
        let err = PyBridgeError::NotEnabled;
        assert_eq!(format!("{}", err), "PyBridge 未启用");

        let err = PyBridgeError::Timeout;
        assert_eq!(format!("{}", err), "请求超时");

        let err = PyBridgeError::ServiceNotFound("test".to_string());
        assert!(format!("{}", err).contains("test"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let py_err: PyBridgeError = io_err.into();
        match py_err {
            PyBridgeError::Io(_) => {}
            _ => panic!("Expected Io variant"),
        }
    }

    #[test]
    fn test_serde_error_conversion() {
        let serde_err = serde_json::from_str::<i32>("not a number").unwrap_err();
        let py_err: PyBridgeError = serde_err.into();
        match py_err {
            PyBridgeError::Serialization(_) => {}
            _ => panic!("Expected Serialization variant"),
        }
    }
}
