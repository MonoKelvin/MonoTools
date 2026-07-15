//! 数据模型（向后兼容层）
//!
//! 搜索相关模型已迁移至 `crate::search_engine::models`，
//! 自定义命令模型已迁移至 `crate::core::command::models`，
//! 这里只是重新导出以减少改动面。
//! 新代码请直接从对应模块导入。

pub mod settings;

pub use settings::{Settings, ThemeMode};

// 搜索相关模型 —— 从 search_engine 模块重新导出
pub use crate::search_engine::models::{
    AppEntry, FileResult, ResultType, SearchAction, SearchCategory, SearchOptions, SearchResult,
};

// 自定义命令模型 —— 从 core::command 模块重新导出
pub use crate::core::command::command_custom::CustomCommand;
