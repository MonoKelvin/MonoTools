// command_trait.rs
use crate::core::command::{CommandContext, CommandOutput};

#[async_trait::async_trait]
pub trait Command: Send + Sync {
    fn spec(&self) -> CommandSpec;

    async fn execute(
        &self,
        args: &[String],
        ctx: &CommandContext,
    ) -> crate::core::error::Result<CommandOutput>;
}

#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub aliases: Vec<&'static str>,
    pub usage: &'static str,
}

impl CommandSpec {
    pub fn new(name: &'static str, description: &'static str) -> Self {
        Self {
            name,
            description,
            aliases: vec![],
            usage: name,
        }
    }

    pub fn with_aliases(mut self, aliases: &[&'static str]) -> Self {
        self.aliases = aliases.to_vec();
        self
    }

    pub fn with_usage(mut self, usage: &'static str) -> Self {
        self.usage = usage;
        self
    }
}
