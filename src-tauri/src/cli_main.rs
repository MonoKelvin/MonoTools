//! MonoTools CLI 入口 - 在终端直接调用，无需 UI
//! 用法示例：
//!   monotools-cli search "chrome"
//!   monotools-cli launch "C:\\Program Files\\..."
//!   monotools-cli startup list
//!   monotools-cli startup toggle <id>
//!   monotools-cli --help

use clap::{Parser, Subcommand};
use monotools_lib::command::{dispatch, CommandContext, CommandOutput};

#[derive(Parser, Debug)]
#[command(name = "monotools-cli")]
#[command(version, about = "MonoTools 命令行工具", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// 输出 JSON 格式
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// 搜索应用/文件/命令
    Search {
        /// 搜索关键字
        query: String,
        /// 最大结果数
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },
    /// 启动应用（按名称）
    Launch {
        /// 应用名称
        name: String,
    },
    /// 打开文件或目录
    Open {
        /// 路径
        path: String,
    },
    /// 启动项管理
    Startup {
        #[command(subcommand)]
        action: StartupAction,
    },
    /// 自定义命令管理
    Command {
        #[command(subcommand)]
        action: CommandAction,
    },
    /// 获取 / 设置偏好
    Config {
        /// 设置 key（如 theme / hotkey）
        key: Option<String>,
        /// 要写入的值（省略则读取当前值）
        value: Option<String>,
    },
    /// 显示帮助
    Help,
    /// 输出版本
    Version,
}

#[derive(Subcommand, Debug)]
enum StartupAction {
    /// 列出所有启动项
    List,
    /// 启用启动项
    Enable { id: String },
    /// 禁用启动项
    Disable { id: String },
    /// 添加自定义启动项
    Add {
        name: String,
        command: String,
        #[arg(long)]
        args: Vec<String>,
        #[arg(long, default_value_t = 0)]
        delay: u32,
    },
    /// 删除启动项
    Remove { id: String },
}

#[derive(Subcommand, Debug)]
enum CommandAction {
    List,
    Run { id: String },
    Add {
        name: String,
        keyword: String,
        command: String,
    },
    Remove { id: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let cli = Cli::parse();

    // 构造命令上下文（headless mode）
    let ctx = CommandContext::new_headless().await?;
    let input = build_input_string(&cli);

    match dispatch(&input, &ctx).await {
        Ok(output) => {
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                print_output(&output);
            }
            std::process::exit(if output.success { 0 } else { 1 });
        }
        Err(e) => {
            if cli.json {
                println!(
                    "{}",
                    serde_json::json!({ "success": false, "error": e.to_string() })
                );
            } else {
                eprintln!("错误: {}", e);
            }
            std::process::exit(2);
        }
    }
}

fn quote(s: &str) -> String {
    shell_words::quote(s).into_owned()
}

fn cmd_name(s: &Commands) -> &'static str {
    match s {
        Commands::Search { .. } => "search",
        Commands::Launch { .. } => "launch",
        Commands::Open { .. } => "open",
        Commands::Startup { .. } => "startup",
        Commands::Command { .. } => "command",
        Commands::Config { .. } => "config",
        Commands::Help => "help",
        Commands::Version => "version",
    }
}

fn build_input_string(cli: &Cli) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push(cmd_name(&cli.command).to_string());
    match &cli.command {
        Commands::Search { query, limit } => {
            parts.push(format!("--limit {}", limit));
            parts.push(quote(query));
        }
        Commands::Launch { name } => {
            parts.push(quote(name));
        }
        Commands::Open { path } => {
            parts.push(quote(path));
        }
        Commands::Startup { action } => match action {
            StartupAction::List => parts.push("list".into()),
            StartupAction::Enable { id } => {
                parts.push("enable".into());
                parts.push(id.clone());
            }
            StartupAction::Disable { id } => {
                parts.push("disable".into());
                parts.push(id.clone());
            }
            StartupAction::Add { name, command, args, delay } => {
                parts.push("add".into());
                parts.push(quote(name));
                parts.push(quote(command));
                if !args.is_empty() {
                    parts.push("--args".into());
                    parts.push(args.join(" "));
                }
                if *delay > 0 {
                    parts.push(format!("--delay {}", delay));
                }
            }
            StartupAction::Remove { id } => {
                parts.push("remove".into());
                parts.push(id.clone());
            }
        },
        Commands::Command { action } => match action {
            CommandAction::List => parts.push("list".into()),
            CommandAction::Run { id } => {
                parts.push("run".into());
                parts.push(id.clone());
            }
            CommandAction::Add { name, keyword, command } => {
                parts.push("add".into());
                parts.push(quote(name));
                parts.push(quote(keyword));
                parts.push(quote(command));
            }
            CommandAction::Remove { id } => {
                parts.push("remove".into());
                parts.push(id.clone());
            }
        },
        Commands::Config { key, value } => {
            parts.push(key.clone().unwrap_or_default());
            if let Some(v) = value {
                parts.push(quote(v));
            }
        }
        Commands::Help => parts.push("help".into()),
        Commands::Version => parts.push("version".into()),
    }
    parts.join(" ")
}

fn print_output(out: &CommandOutput) {
    if out.success {
        if !out.message.is_empty() {
            println!("✓ {}", out.message);
        }
    } else {
        eprintln!("✗ {}", out.message);
    }
    if let Some(data) = &out.data {
        match data {
            serde_json::Value::Array(arr) => {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        println!("  {}", s);
                    } else if let Some(map) = item.as_object() {
                        let id_str = map
                            .get("id")
                            .and_then(|v| v.as_str())
                            .map(|s| format!(" ({})", s))
                            .unwrap_or_default();
                        let name = map
                            .get("name")
                            .and_then(|v| v.as_str())
                            .or_else(|| map.get("title").and_then(|v| v.as_str()))
                            .unwrap_or("(no name)");
                        println!("  {}{}", name, id_str);
                    } else {
                        println!("  {}", item);
                    }
                }
            }
            _ => {}
        }
    }
}
