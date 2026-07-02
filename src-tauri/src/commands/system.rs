use anyhow::Result;
use serde_json::Value;
use crate::models::command::{Command, CommandHandler, CommandContext};

pub struct SystemCommandHandler;

impl SystemCommandHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl CommandHandler for SystemCommandHandler {
    async fn execute(&self, cmd: &Command, ctx: &CommandContext) -> Result<Value> {
        match cmd.action.as_str() {
            "shutdown" => self.shutdown(cmd).await,
            "lock" => self.lock().await,
            "empty-trash" => self.empty_trash().await,
            "open" => self.open(cmd).await,
            "sleep" => self.sleep().await,
            "hibernate" => self.hibernate().await,
            "restart" => self.restart().await,
            _ => Err(anyhow::anyhow!("Unknown system action: {}", cmd.action)),
        }
    }

    fn validate(&self, cmd: &Command) -> Result<()> {
        match cmd.action.as_str() {
            "open" => {
                if !cmd.args.contains_key("path") {
                    return Err(anyhow::anyhow!("Missing required argument: path"));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl SystemCommandHandler {
    /// 关机/重启
    async fn shutdown(&self, cmd: &Command) -> Result<Value> {
        let restart = cmd.flags.contains(&"restart".to_string());
        let sleep = cmd.flags.contains(&"sleep".to_string());
        let hibernate = cmd.flags.contains(&"hibernate".to_string());

        if sleep {
            self.execute_system_command("rundll32.exe", &["powrprof.dll,SetSuspendState", "0,1,0"])?;
        } else if hibernate {
            self.execute_system_command("rundll32.exe", &["powrprof.dll,SetHibernateState", "1"])?;
        } else if restart {
            self.execute_system_command("shutdown.exe", &["/r", "/t", "0"])?;
        } else {
            self.execute_system_command("shutdown.exe", &["/s", "/t", "0"])?;
        }

        Ok(serde_json::json!({
            "success": true,
            "action": if restart { "restart" } else if sleep { "sleep" } else if hibernate { "hibernate" } else { "shutdown" }
        }))
    }

    /// 锁定工作站
    async fn lock(&self) -> Result<Value> {
        #[cfg(target_os = "windows")]
        {
            unsafe {
                windows::Win32::System::Shutdown::LockWorkStation();
            }
        }

        Ok(serde_json::json!({
            "success": true,
            "message": "Workstation locked"
        }))
    }

    /// 清空回收站
    async fn empty_trash(&self) -> Result<Value> {
        #[cfg(target_os = "windows")]
        {
            unsafe {
                windows::Win32::UI::Shell::SHEmptyRecycleBinW(
                    None,
                    None,
                    windows::Win32::UI::Shell::SHERB_NOCONFIRMATION | windows::Win32::UI::Shell::SHERB_NOPROGRESSUI | windows::Win32::UI::Shell::SHERB_NOSOUND,
                );
            }
        }

        Ok(serde_json::json!({
            "success": true,
            "message": "Recycle bin emptied"
        }))
    }

    /// 打开路径
    async fn open(&self, cmd: &Command) -> Result<Value> {
        let path = cmd.args.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing path argument"))?;

        self.execute_system_command("explorer.exe", &[path])?;

        Ok(serde_json::json!({
            "success": true,
            "path": path
        }))
    }

    /// 睡眠
    async fn sleep(&self) -> Result<Value> {
        self.execute_system_command("rundll32.exe", &["powrprof.dll,SetSuspendState", "0,1,0"])?;
        Ok(serde_json::json!({"success": true}))
    }

    /// 休眠
    async fn hibernate(&self) -> Result<Value> {
        self.execute_system_command("rundll32.exe", &["powrprof.dll,SetHibernateState", "1"])?;
        Ok(serde_json::json!({"success": true}))
    }

    /// 重启
    async fn restart(&self) -> Result<Value> {
        self.execute_system_command("shutdown.exe", &["/r", "/t", "0"])?;
        Ok(serde_json::json!({"success": true}))
    }

    /// 执行系统命令
    fn execute_system_command(&self, program: &str, args: &[&str]) -> Result<()> {
        use std::process::Command;
        Command::new(program)
            .args(args)
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to execute {}: {}", program, e))?;
        Ok(())
    }
}

impl Default for SystemCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}
