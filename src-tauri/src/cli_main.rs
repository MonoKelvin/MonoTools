//! MonoTools CLI 入口 - 在终端直接调用，无需 UI
//!
//! 用法示例：
//!   monotools-cli search "chrome"
//!   monotools-cli launch "notepad"
//!   monotools-cli open "C:\Users"
//!   monotools-cli index build
//!   monotools-cli stats
//!   monotools-cli --help

use clap::{Parser, Subcommand};
use monotools_lib::core::command::{dispatch, CommandContext, CommandOutput};

#[derive(Parser, Debug)]
#[command(name = "monotools-cli")]
#[command(version, about = "MonoTools 命令行工具", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Search {
        query: String,
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },
    Launch {
        name: String,
    },
    Open {
        path: String,
    },
    Command {
        #[command(subcommand)]
        action: CommandAction,
    },
    Config {
        key: Option<String>,
        value: Option<String>,
    },
    Index {
        #[command(subcommand)]
        action: IndexAction,
    },
    Stats {
        #[arg(long)]
        detail: bool,
    },
    Help,
    Version,
}

#[derive(Subcommand, Debug)]
enum CommandAction {
    List,
    Run {
        id: String,
    },
    Add {
        name: String,
        keyword: String,
        command: String,
    },
    Remove {
        id: String,
    },
}

#[derive(Subcommand, Debug)]
enum IndexAction {
    Build,
    Update,
    Stats,
    AddRoot { path: String },
    RemoveRoot { path: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    // 自定义日志格式：时间 级别 消息（去掉模块路径）
    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            use std::io::Write;
            let timestamp = buf.timestamp();
            writeln!(buf, "[{} {}] {}", timestamp, record.level(), record.args())
        })
        .init();

    let cli = Cli::parse();
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
        Commands::Command { .. } => "command",
        Commands::Config { .. } => "config",
        Commands::Index { .. } => "index",
        Commands::Stats { .. } => "stats",
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
        Commands::Command { action } => match action {
            CommandAction::List => parts.push("list".into()),
            CommandAction::Run { id } => {
                parts.push("run".into());
                parts.push(id.clone());
            }
            CommandAction::Add {
                name,
                keyword,
                command,
            } => {
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
            if let Some(k) = key {
                parts.push(k.clone());
            }
            if let Some(v) = value {
                parts.push(quote(v));
            }
        }
        Commands::Index { action } => match action {
            IndexAction::Build => parts.push("build".into()),
            IndexAction::Update => parts.push("update".into()),
            IndexAction::Stats => parts.push("stats".into()),
            IndexAction::AddRoot { path } => {
                parts.push("add-root".into());
                parts.push(quote(path));
            }
            IndexAction::RemoveRoot { path } => {
                parts.push("remove-root".into());
                parts.push(quote(path));
            }
        },
        Commands::Stats { detail } => {
            if *detail {
                parts.push("detail".into());
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
                        let title = map
                            .get("title")
                            .and_then(|v| v.as_str())
                            .unwrap_or("(no name)");
                        let subtitle = map.get("subtitle").and_then(|v| v.as_str());
                        let score = map.get("score").and_then(|v| v.as_f64());

                        let mut line = format!("  {}", title);
                        if let Some(s) = subtitle {
                            line.push_str(&format!(" - {}", s));
                        }
                        if let Some(sc) = score {
                            line.push_str(&format!(" (score: {:.2})", sc));
                        }
                        println!("{}", line);
                    } else {
                        println!("  {}", item);
                    }
                }
            }
            serde_json::Value::Object(map) => {
                for (k, v) in map {
                    if let Some(s) = v.as_str() {
                        println!("  {}: {}", k, s);
                    } else if let Some(n) = v.as_number() {
                        println!("  {}: {}", k, n);
                    } else if let Some(arr) = v.as_array() {
                        println!("  {}:", k);
                        for item in arr {
                            if let Some(s) = item.as_str() {
                                println!("    - {}", s);
                            } else {
                                println!("    - {}", item);
                            }
                        }
                    } else {
                        println!("  {}: {}", k, v);
                    }
                }
            }
            _ => {}
        }
    }
}
