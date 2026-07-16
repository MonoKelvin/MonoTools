//! Windows 平台相关命令
//!
//! 文件操作、应用启动等与 Windows Shell 相关的命令实现。
//! 这些命令属于平台业务模块，通过注册机制接入核心命令系统。

use crate::core::command::{Command, CommandContext, CommandOutput, CommandSpec};
use crate::platform::windows::shell;

/// open 命令 - 在文件管理器中打开
pub struct OpenCommand;

#[async_trait::async_trait]
impl Command for OpenCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("open", "在文件管理器中打开路径").with_usage("open <path>")
    }

    async fn execute(
        &self,
        args: &[String],
        _ctx: &CommandContext,
    ) -> crate::core::error::Result<CommandOutput> {
        if args.is_empty() {
            return Ok(CommandOutput::err("用法：open <path>"));
        }
        let path = args.join(" ");
        let path_buf = std::path::PathBuf::from(&path);
        shell::open_path(&path_buf)?;
        Ok(CommandOutput::ok(format!("已打开 {path}")))
    }
}

/// open-location 命令 - 打开文件所在目录
pub struct OpenLocationCommand;

#[async_trait::async_trait]
impl Command for OpenLocationCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("open-location", "打开文件所在目录").with_usage("open-location <path>")
    }

    async fn execute(
        &self,
        args: &[String],
        _ctx: &CommandContext,
    ) -> crate::core::error::Result<CommandOutput> {
        if args.is_empty() {
            return Ok(CommandOutput::err("用法：open-location <path>"));
        }
        let path = args.join(" ");
        let path_buf = std::path::PathBuf::from(&path);
        shell::open_containing_folder(&path_buf)?;
        Ok(CommandOutput::ok(format!("已打开 {path} 的所在目录")))
    }
}

/// delete-file 命令 - 删除文件到回收站
pub struct DeleteFileCommand;

#[async_trait::async_trait]
impl Command for DeleteFileCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("delete-file", "删除文件到回收站").with_usage("delete-file <path>")
    }

    async fn execute(
        &self,
        args: &[String],
        _ctx: &CommandContext,
    ) -> crate::core::error::Result<CommandOutput> {
        if args.is_empty() {
            return Ok(CommandOutput::err("用法：delete-file <path>"));
        }
        let path = args.join(" ");
        let path_buf = std::path::PathBuf::from(&path);
        shell::delete_to_recycle_bin(&path_buf)?;
        Ok(CommandOutput::ok(format!("已删除 {path}")))
    }
}

/// show-properties 命令 - 显示文件属性
pub struct ShowPropertiesCommand;

#[async_trait::async_trait]
impl Command for ShowPropertiesCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("show-properties", "显示文件属性").with_usage("show-properties <path>")
    }

    async fn execute(
        &self,
        args: &[String],
        _ctx: &CommandContext,
    ) -> crate::core::error::Result<CommandOutput> {
        if args.is_empty() {
            return Ok(CommandOutput::err("用法：show-properties <path>"));
        }
        let path = args.join(" ");
        let path_buf = std::path::PathBuf::from(&path);
        shell::show_file_properties(&path_buf)?;
        Ok(CommandOutput::ok(format!("已显示 {path} 的属性")))
    }
}

/// launch 命令 - 启动应用
pub struct LaunchCommand;

#[async_trait::async_trait]
impl Command for LaunchCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("launch", "按名称或路径启动应用")
            .with_aliases(&["run", "open-app"])
            .with_usage("launch <name-or-path>")
    }

    async fn execute(
        &self,
        args: &[String],
        ctx: &CommandContext,
    ) -> crate::core::error::Result<CommandOutput> {
        if args.is_empty() {
            return Ok(CommandOutput::err("用法：launch <name-or-path>"));
        }
        let name = args.join(" ");
        let results = ctx.app_search.search(&name, 1);
        if let Some(r) = results.first() {
            let path = r.subtitle.clone();
            shell::launch(&path, &[])?;
            return Ok(CommandOutput::ok(format!("已启动 {}", r.title)));
        }
        shell::launch(&name, &[])?;
        Ok(CommandOutput::ok(format!("已启动 {name}")))
    }
}

/// copy-path 命令 - 复制文件路径到剪贴板
pub struct CopyPathCommand;

#[async_trait::async_trait]
impl Command for CopyPathCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec::new("copy-path", "复制文件路径到剪贴板")
            .with_aliases(&["cp", "copy"])
            .with_usage("copy-path <path>")
    }

    async fn execute(
        &self,
        args: &[String],
        _ctx: &CommandContext,
    ) -> crate::core::error::Result<CommandOutput> {
        if args.is_empty() {
            return Ok(CommandOutput::err("用法：copy-path <path>"));
        }
        let path = args.join(" ");
        let path_buf = std::path::PathBuf::from(&path);
        shell::copy_path_to_clipboard(&path_buf)?;
        Ok(CommandOutput::ok(format!("已复制路径: {path}")))
    }
}

/// 注册所有 Windows 平台命令到注册表
pub fn register_commands(reg: &mut crate::core::command::CommandRegistry) {
    reg.register(OpenCommand);
    reg.register(OpenLocationCommand);
    reg.register(DeleteFileCommand);
    reg.register(ShowPropertiesCommand);
    reg.register(LaunchCommand);
    reg.register(CopyPathCommand);
}
