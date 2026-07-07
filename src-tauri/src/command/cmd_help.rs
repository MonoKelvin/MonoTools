//! help 命令
use crate::command::command_trait::{Command, CommandSpec};
use crate::command::{CommandContext, CommandOutput};

pub struct HelpCommand;

const HELP_TEXT: &str = r#"MonoTools CLI - 命令帮助

用法：
  monotools-cli <command> [args]

可用命令：
  search <query>           搜索应用/文件/命令
  launch <name>             启动应用
  open <path>                在文件管理器中打开路径
  startup <sub>              启动项管理
    ├ list                     列出所有
    ├ enable <id>              启用
    ├ disable <id>             禁用
    ├ add <name> <cmd>         添加
    └ remove <id>              删除
  command <sub>              自定义命令
    ├ list
    ├ run <id>
    ├ add <name> <kw> <cmd>
    └ remove <id>
  config [key] [val]        读取/设置配置
  help                      帮助
  version                   版本

示例：
  monotools-cli search chrome
  monotools-cli launch "Visual Studio Code"
  monotools-cli startup list
  monotools-cli config hotkey "Ctrl+Space"
"#;

#[async_trait::async_trait]
impl Command for HelpCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("help", "显示帮助").with_aliases(&["-h", "--help"])
    }

    async fn execute(
        &self,
        _args: &[String],
        _ctx: &CommandContext,
    ) -> crate::error::Result<CommandOutput> {
        Ok(CommandOutput::ok(HELP_TEXT))
    }
}
