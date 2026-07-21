//! 命令系统 —— 通用机制 + 自定义命令模型 + 系统内置命令
//!
//! 核心框架（纯机制，无业务逻辑）：
//! - `Command` trait / `CommandSpec`: 命令接口
//! - `CommandRegistry`: 命令注册与派发
//! - `CommandOutput`: 统一输出格式
//! - `CommandContext`: 命令执行上下文（基于 TypeMap 的依赖注入容器）
//!
//! 自定义命令模型与仓库：
//! - `CustomCommand`: 自定义命令数据结构
//! - `CommandRepo` trait + InMemoryCommandRepo: 自定义命令存储接口
//!
//! 系统内置命令（本模块保留，因为是通用系统功能）：
//! - `HelpCommand`: 帮助命令
//! - `VersionCommand`: 版本命令
//! - `CustomCommandHandler`: 自定义命令管理
//! - `ConfigCommand`: 设置管理（从 core::settings 重新导出）
//!
//! 业务命令已迁移到各自模块：
//! - Windows 平台命令 → `crate::platform::windows::commands`
//! - 搜索 & 索引命令 → `crate::search_engine::commands`

pub mod command_custom;
pub mod command_registry;
pub mod command_repo;
pub mod command_trait;
pub mod ipc;

pub mod cmd_custom;
pub mod cmd_help;
pub mod cmd_version;

pub use command_custom::CustomCommand;
pub use command_registry::{build_core_registry, dispatch, registry_dispatch, CommandRegistry};
pub use command_repo::{CommandRepo, InMemoryCommandRepo};
pub use command_trait::{Command, CommandSpec};

pub use cmd_custom::CustomCommandHandler;
pub use cmd_help::HelpCommand;
pub use cmd_version::VersionCommand;

pub use crate::core::settings::SettingsRepo;
use crate::core::type_map::TypeMap;

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommandOutput {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl CommandOutput {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
        }
    }

    pub fn ok_with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
        }
    }
}

/// 命令执行上下文
///
/// 基于 TypeMap 的依赖注入容器，core 层不关心具体有哪些依赖，
/// 各业务模块自行注入和获取所需的依赖。
///
/// # 示例
///
/// ```ignore
/// // 注入依赖
/// let mut ctx = CommandContext::new();
/// ctx.insert(Arc::new(AppSearchEngine::new(...)));
///
/// // 获取依赖（命令内部使用）
/// let app_search = ctx.get::<Arc<AppSearchEngine>>().unwrap();
/// ```
#[derive(Default)]
pub struct CommandContext {
    map: TypeMap,
}

impl CommandContext {
    /// 创建一个空的上下文
    pub fn new() -> Self {
        Self {
            map: TypeMap::new(),
        }
    }

    /// 注入一个依赖
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) {
        self.map.insert(value);
    }

    /// 获取依赖的引用
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.map.get::<T>()
    }

    /// 获取依赖的可变引用
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.map.get_mut::<T>()
    }

    /// 检查是否包含指定类型的依赖
    pub fn contains<T: 'static>(&self) -> bool {
        self.map.contains::<T>()
    }
}
