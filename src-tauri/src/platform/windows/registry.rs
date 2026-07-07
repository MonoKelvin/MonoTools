//! Windows 注册表操作 - 启动项读取/写入
//!
//! 本文件旨在无 fail 的可编译 shell 实现。对于复杂的 Win32 调用，
//! 我们改用 `reg.exe` 命令行 wrapper（registry 命令行工具）以避免
//! 直接与 windows-rs v0.58 复杂的类型交互。

use crate::error::{AppError, Result};
use crate::models::{StartupItem, StartupSource};
use std::path::PathBuf;
use std::process::Command;

const HKCU_RUN: &str = "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const HKLM_RUN: &str = "HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Run";

pub fn read_run_key(hklm: bool, source: StartupSource) -> Result<Vec<StartupItem>> {
    let hive = if hklm { "HKLM" } else { "HKCU" };
    let path = if hklm { HKLM_RUN } else { HKCU_RUN };

    let output = Command::new("reg").args(["query", path]).output();
    let out = match output {
        Ok(o) => o,
        Err(e) => {
            log::warn!("reg query 调用失败: {e}");
            return Ok(vec![]);
        }
    };
    if !out.status.success() {
        // 没有 Run 子键
        return Ok(vec![]);
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut items = Vec::new();
    for line in stdout.lines() {
        // 形如: "    AppName    REG_SZ    \"C:\\path\""
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let name = parts[0].to_string();
        if name.starts_with('(') {
            // header like (Default)
            continue;
        }
        let value = parts[2..].join(" ");
        let value = value.trim_matches('"').to_string();
        if value.is_empty() {
            continue;
        }
        items.push(StartupItem {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            command: value,
            args: vec![],
            working_dir: None,
            enabled: !starts_with_dot(&line),
            delay_seconds: 0,
            run_as_admin: false,
            source: source.clone(),
            created_at: chrono::Utc::now().timestamp(),
        });
    }

    log::info!("读取 {}{}: {} 项", hive, HKCU_RUN, items.len());
    Ok(items)
}

fn starts_with_dot(s: &str) -> bool {
    s.trim().starts_with('.')
}

pub fn write_run_key(hklm: bool, name: &str, command: &str) -> Result<()> {
    let path = if hklm { HKLM_RUN } else { HKCU_RUN };
    let status = Command::new("reg")
        .args(["add", path, "/v", name, "/t", "REG_SZ", "/d", command, "/f"])
        .status()
        .map_err(|e| AppError::Other(format!("reg add 调用失败: {e}")))?;
    if !status.success() {
        return Err(AppError::Other(format!(
            "reg add {} 失败 (status={:?})",
            path,
            status.code()
        )));
    }
    Ok(())
}

pub fn delete_run_value(hklm: bool, name: &str) -> Result<()> {
    let path = if hklm { HKLM_RUN } else { HKCU_RUN };
    let status = Command::new("reg")
        .args(["delete", path, "/v", name, "/f"])
        .status()
        .map_err(|e| AppError::Other(format!("reg delete 调用失败: {e}")))?;
    if !status.success() {
        return Err(AppError::Other(format!(
            "reg delete 失败 (status={:?})",
            status.code()
        )));
    }
    Ok(())
}

pub fn backup_run_keys() -> Result<(Vec<StartupItem>, Vec<StartupItem>)> {
    Ok((
        read_run_key(false, StartupSource::RegistryRun)?,
        read_run_key(true, StartupSource::RegistryRun)?,
    ))
}

#[allow(dead_code)]
pub fn try_unused(_: &PathBuf) {}
