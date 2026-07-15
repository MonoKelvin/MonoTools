//! 数据模型
//!
//! 通用设置模型在此定义。
//! 搜索相关模型位于 `crate::search_engine::models`，
//! 自定义命令模型位于 `crate::core::command::command_custom`。

pub mod settings;

pub use settings::{Settings, ThemeMode};
