//! 通用设置模块 —— 纯机制，无业务逻辑
//!
//! 提供设置项的存储、读取、更新等通用机制。
//! 各业务模块通过注册机制接入自己的设置项，
//! 或直接使用 SettingsRepo 管理自己的设置结构体。
//!
//! 包含：
//! - `Settings`: 默认全局设置结构体（通用底层设置，各模块可扩展）
//! - `SettingsRepo`: 设置仓储 trait（抽象存储层）
//! - `InMemorySettingsRepo`: 内存版实现
//! - `ipc`: 设置相关的 IPC 命令
//! - `cmd_settings`: 设置相关的 CLI 命令
//!
//! # 未来扩展
//! - 动态设置项注册机制
//! - 文件持久化实现
//! - SQLite 持久化实现

pub mod models;
pub mod repo;
pub mod ipc;
pub mod cmd_settings;
pub mod tray;

pub use models::{Settings, ThemeMode};
pub use repo::{InMemorySettingsRepo, SettingsRepo};
