//! JSON-RPC 2.0 协议实现

use serde::{Deserialize, Serialize};

/// JSON-RPC 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(default = "default_params")]
    pub params: serde_json::Value,
}

fn default_params() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

impl JsonRpcRequest {
    /// 创建新的请求
    pub fn new(id: u64, method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }

    /// 序列化为 JSON 行（带换行符）
    pub fn to_line(&self) -> Result<String, serde_json::Error> {
        let mut s = serde_json::to_string(self)?;
        s.push('\n');
        Ok(s)
    }
}

/// JSON-RPC 成功响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcSuccess {
    pub jsonrpc: String,
    pub id: u64,
    pub result: serde_json::Value,
}

/// JSON-RPC 错误响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorData {
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub jsonrpc: String,
    pub id: u64,
    pub error: JsonRpcErrorData,
}

/// JSON-RPC 响应（成功或错误）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcResponse {
    Success(JsonRpcSuccess),
    Error(JsonRpcError),
}

impl JsonRpcResponse {
    /// 从 JSON 行解析
    pub fn from_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }

    /// 获取响应 ID
    pub fn id(&self) -> u64 {
        match self {
            JsonRpcResponse::Success(s) => s.id,
            JsonRpcResponse::Error(e) => e.id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = JsonRpcRequest::new(
            1,
            "test.method",
            serde_json::json!({"key": "value"}),
        );
        let line = req.to_line().unwrap();
        assert!(line.ends_with('\n'));

        let parsed: JsonRpcRequest = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(parsed.id, 1);
        assert_eq!(parsed.method, "test.method");
        assert_eq!(parsed.params["key"], "value");
    }

    #[test]
    fn test_success_response() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"ok":true}}"#;
        let resp = JsonRpcResponse::from_line(json).unwrap();
        match resp {
            JsonRpcResponse::Success(s) => {
                assert_eq!(s.id, 1);
                assert_eq!(s.result["ok"], true);
            }
            _ => panic!("Expected success response"),
        }
    }

    #[test]
    fn test_error_response() {
        let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#;
        let resp = JsonRpcResponse::from_line(json).unwrap();
        match resp {
            JsonRpcResponse::Error(e) => {
                assert_eq!(e.id, 1);
                assert_eq!(e.error.code, -32601);
                assert_eq!(e.error.message, "Method not found");
            }
            _ => panic!("Expected error response"),
        }
    }
}
