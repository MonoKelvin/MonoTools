//! 启动文件夹快捷方式扫描
use crate::error::{AppError, Result};
use crate::models::{StartupItem, StartupSource};

pub fn startup_folders() -> Result<Vec<std::path::PathBuf>> {
    let mut roots = Vec::new();

    if let Ok(roaming) = std::env::var("APPDATA") {
        let p = std::path::PathBuf::from(roaming)
            .join("Microsoft\\Windows\\Start Menu\\Programs\\Startup");
        if p.exists() {
            roots.push(p);
        }
    }

    if let Ok(prog) = std::env::var("ProgramData") {
        let p = std::path::PathBuf::from(prog)
            .join("Microsoft\\Windows\\Start Menu\\Programs\\Startup");
        if p.exists() {
            roots.push(p);
        }
    }
    Ok(roots)
}

pub fn read_items() -> Result<Vec<StartupItem>> {
    let mut out = Vec::new();
    for dir in startup_folders()? {
        let entries = std::fs::read_dir(&dir)?;
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                continue;
            }
            out.push(StartupItem {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                command: path.to_string_lossy().to_string(),
                args: vec![],
                working_dir: None,
                enabled: true,
                delay_seconds: 0,
                run_as_admin: false,
                source: StartupSource::StartupFolder,
                created_at: chrono::Utc::now().timestamp(),
            });
        }
    }
    Ok(out)
}

pub fn disable_item(_path: &str) -> Result<()> {
    Err(AppError::Other("需要将快捷方式重命名为 .disabled".into()))
}

pub fn enable_item(_path: &str) -> Result<()> {
    Err(AppError::Other("需要将 .disabled 重命名回 .lnk".into()))
}
