//! 命令引擎 - 提供给 Tauri Command 复用的统一入口
use crate::command::command_trait::Command;
use crate::command::{CommandContext, CommandOutput};
use crate::command::command_registry::CommandRegistry;

pub struct CommandEngine {
    registry: CommandRegistry,
}

impl CommandEngine {
    pub fn new() -> Self {
        let registry = CommandRegistry::new();
        Self { registry }
    }

    pub fn registry(&self) -> &CommandRegistry {
        &self.registry
    }
}

impl Default for CommandEngine {
    fn default() -> Self {
        Self::new()
    }
}
