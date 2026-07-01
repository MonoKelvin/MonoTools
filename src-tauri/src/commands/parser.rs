use crate::models::command::{Command, Value};
use anyhow::{Context, Result, anyhow};

pub struct CommandParser;

impl CommandParser {
    /// 解析命令字符串
    pub fn parse(input: &str) -> Result<Command> {
        let input = input.trim();

        if input.is_empty() {
            return Err(anyhow!("Empty command"));
        }

        let mut parts = input.split_whitespace();

        // 解析 namespace:action
        let head = parts.next().unwrap();
        let (namespace, action) = if head.contains(':') {
            let mut split = head.split(':');
            (
                split.next().unwrap().to_string(),
                split.next().unwrap().to_string(),
            )
        } else {
            // 简写形式，默认 namespace 为 core
            ("core".to_string(), head.to_string())
        };

        // 解析参数
        let mut args = HashMap::new();
        let mut flags = Vec::new();

        for part in parts {
            if part.starts_with("--") {
                // 布尔开关
                flags.push(part.trim_start_matches("--").to_string());
            } else if part.contains('=') {
                // 键值对
                let mut split = part.splitn(2, '=');
                let key = split.next().unwrap().to_string();
                let value = Self::parse_value(split.next().unwrap_or(""));
                args.insert(key, value);
            } else if part.starts_with('"') && part.ends_with('"') {
                // 带引号的字符串
                args.insert(
                    "query".to_string(),
                    Value::String(part[1..part.len() - 1].to_string()),
                );
            } else if part.starts_with("'") && part.ends_with("'") {
                args.insert(
                    "query".to_string(),
                    Value::String(part[1..part.len() - 1].to_string()),
                );
            } else {
                // 位置参数，放入 "arg"
                args.insert("arg".to_string(), Value::String(part.to_string()));
            }
        }

        Ok(Command {
            namespace,
            action,
            args,
            flags,
        })
    }

    /// 解析值
    fn parse_value(s: &str) -> Value {
        if let Ok(n) = s.parse::<i64>() {
            Value::Number(n.into())
        } else if let Ok(f) = s.parse::<f64>() {
            Value::Number(
                serde_json::Number::from_f64(f)
                    .unwrap_or_else(|| serde_json::Number::from(0)),
            )
        } else if let Ok(b) = s.parse::<bool>() {
            Value::Bool(b)
        } else if s == "null" {
            Value::Null
        } else {
            Value::String(s.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_command() {
        let cmd = CommandParser::parse("search:files query=report.docx").unwrap();
        assert_eq!(cmd.namespace, "search");
        assert_eq!(cmd.action, "files");
        assert_eq!(cmd.args.get("query"), Some(&Value::String("report.docx".to_string())));
    }

    #[test]
    fn test_parse_command_with_flags() {
        let cmd = CommandParser::parse("plugin:list --enabled-only").unwrap();
        assert_eq!(cmd.namespace, "plugin");
        assert_eq!(cmd.action, "list");
        assert!(cmd.flags.contains(&"enabled-only".to_string()));
    }

    #[test]
    fn test_parse_short_command() {
        let cmd = CommandParser::parse("show").unwrap();
        assert_eq!(cmd.namespace, "core");
        assert_eq!(cmd.action, "show");
    }

    #[test]
    fn test_parse_number_value() {
        let cmd = CommandParser::parse("search:files query=test limit=50").unwrap();
        assert_eq!(cmd.args.get("limit"), Some(&Value::Number(50.into())));
    }

    #[test]
    fn test_parse_boolean_value() {
        let cmd = CommandParser::parse("config:set startup=true").unwrap();
        assert_eq!(cmd.args.get("startup"), Some(&Value::Bool(true)));
    }
}
