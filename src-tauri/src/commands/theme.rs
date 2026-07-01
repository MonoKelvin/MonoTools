use anyhow::Result;
use serde_json::Value;
use crate::commands::bus::{Command, CommandHandler, CommandContext};

pub struct ThemeCommandHandler;

impl ThemeCommandHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl CommandHandler for ThemeCommandHandler {
    async fn execute(&self, cmd: &Command, ctx: &CommandContext) -> Result<Value> {
        match cmd.action.as_str() {
            "list" => self.list_themes().await,
            "set" => self.set_theme(cmd, ctx).await,
            "get" => self.get_current_theme().await,
            "preview" => self.preview_theme(cmd).await,
            _ => Err(anyhow::anyhow!("Unknown theme action: {}", cmd.action)),
        }
    }

    fn validate(&self, cmd: &Command) -> Result<()> {
        match cmd.action.as_str() {
            "set" | "preview" => {
                if !cmd.args.contains_key("id") {
                    return Err(anyhow::anyhow!("Missing required argument: id"));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl ThemeCommandHandler {
    /// 列出所有可用主题
    async fn list_themes(&self) -> Result<Value> {
        let themes = vec![
            serde_json::json!({
                "id": "builtin:default-theme",
                "name": "默认深色",
                "description": "Linear 风格深色主题",
                "builtin": true,
                "modes": ["dark", "light"]
            }),
            // TODO: 添加用户安装的主题
        ];

        Ok(serde_json::json!({
            "themes": themes
        }))
    }

    /// 设置当前主题
    async fn set_theme(&self, cmd: &Command, ctx: &CommandContext) -> Result<Value> {
        let id = cmd.args.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing id argument"))?;

        let mode = cmd.args.get("mode")
            .and_then(|v| v.as_str());

        // TODO: 加载主题插件
        // 1. 查找主题插件
        // 2. 读取主题 CSS 变量
        // 3. 发送 theme:changed 事件到前端
        // 4. 更新配置

        // 发送事件到前端
        let css_variables = serde_json::json!({
            "--mt-primary": "#5e6ad2",
            "--mt-surface-1": "#0f1011",
        });

        let _ = ctx.app_handle.emit("theme:changed", serde_json::json!({
            "theme_id": id,
            "mode": mode.unwrap_or("dark"),
            "cssVariables": css_variables
        }));

        // 更新配置
        // TODO: 调用 config store

        Ok(serde_json::json!({
            "success": true,
            "theme": {
                "id": id,
                "mode": mode.unwrap_or("dark")
            }
        }))
    }

    /// 获取当前主题
    async fn get_current_theme(&self) -> Result<Value> {
        // TODO: 从配置中读取当前主题
        Ok(serde_json::json!({
            "id": "builtin:default-theme",
            "name": "默认深色",
            "mode": "dark"
        }))
    }

    /// 预览主题（不保存）
    async fn preview_theme(&self, cmd: &Command) -> Result<Value> {
        let _id = cmd.args.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing id argument"))?;

        // TODO: 临时应用主题并发送事件

        Ok(serde_json::json!({
            "success": true,
            "message": "Theme preview not fully implemented yet"
        }))
    }
}

impl Default for ThemeCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}
