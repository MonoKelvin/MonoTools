use std::path::PathBuf;
use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "monotools")]
#[command(about = "MonoTools - High-performance desktop launcher")]
#[command(version, long_version = option_env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    /// 静默模式（不输出日志）
    #[arg(short, long)]
    silent: bool,

    /// 输出 JSON 格式
    #[arg(long)]
    json: bool,

    #[command(subcommand)]
    command: Option<Commands>,

    /// 数据目录（覆盖默认路径）
    #[arg(long, env = "MONOTOOLS_DATA_DIR")]
    data_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 启动守护进程（正常启动）
    Daemon,
    /// 搜索相关
    Search {
        #[command(subcommand)]
        action: SearchCommands,
    },
    /// 工作区管理
    Workspace {
        #[command(subcommand)]
        action: WorkspaceCommands,
    },
    /// 插件管理
    Plugin {
        #[command(subcommand)]
        action: PluginCommands,
    },
    /// 直接执行命令字符串
    Exec {
        command: String,
    },
    /// 配置管理
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
}

#[derive(Subcommand)]
pub enum SearchCommands {
    Files {
        query: String,
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
    Apps {
        query: String,
    },
}

#[derive(Subcommand)]
pub enum WorkspaceCommands {
    Save {
        name: String,
        #[arg(short, long)]
        description: Option<String>,
        #[arg(long)]
        auto_start: bool,
    },
    Restore {
        id: String,
        #[arg(long)]
        force: bool,
    },
    List,
}

#[derive(Subcommand)]
pub enum PluginCommands {
    List {
        #[arg(long)]
        enabled_only: bool,
        #[arg(long)]
        builtin_only: bool,
    },
    Install {
        path: PathBuf,
    },
    Uninstall {
        id: String,
        #[arg(long)]
        force: bool,
    },
    Reload {
        id: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    Get {
        key: Option<String>,
    },
    Set {
        key: String,
        value: String,
    },
    Path,
}

pub fn parse() -> Result<Cli> {
    Ok(Cli::parse())
}
