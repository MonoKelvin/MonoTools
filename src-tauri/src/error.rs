//! 集中错误类型
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Windows API error: {0}")]
    WindowsApi(String),

    #[error("Tauri error: {0}")]
    Tauri(#[from] tauri::Error),

    #[error("Hotkey already registered: {0}")]
    HotkeyAlreadyRegistered(String),

    #[error("Startup item not found: {0}")]
    StartupItemNotFound(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Permission denied")]
    PermissionDenied,

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("IO timeout")]
    Timeout,

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, AppError>;

// 兼容 Some(anyhow::Error) → AppError
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Other(err.to_string())
    }
}

impl From<log::Record<'_>> for AppError {
    fn from(_: log::Record) -> Self {
        AppError::Other("log conversion".into())
    }
}

// 让 AppError 能自动从字符串创建
impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Other(s.into())
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Other(s)
    }
}
