//! 核心模块 —— 纯通用框架，无业务逻辑
//!
//! 核心模块只提供基础设施和扩展机制：
//! - `command`: 命令系统框架（trait + registry + 系统内置命令）
//! - `config`: 全局配置常量
//! - `error`: 全局错误类型
//!
//! 所有业务逻辑都在各业务模块中实现，通过注册机制接入核心。

pub mod command;
pub mod config;
pub mod error;

pub use command::{
    build_default_registry, dispatch, registry_dispatch, Command, CommandContext, CommandOutput,
    CommandRegistry, CommandSpec, HelpCommand, VersionCommand,
};

pub use error::{AppError, Result};
