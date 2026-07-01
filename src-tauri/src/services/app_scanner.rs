use anyhow::Result;
use rusqlite::Connection;
use serde_json::Value;
use std::collections::HashMap;
use walkdir::WalkDir;

pub struct AppScanner {
    db: Connection,
}

impl AppScanner {
    pub fn new(db: Connection) -> Self {
        Self { db }
    }

    /// 扫描开始菜单应用
    pub fn scan_start_menu(&mut self) -> Result<Vec<AppEntry>> {
        let mut apps = Vec::new();
        let start_menu_paths = Self::get_start_menu_paths();

        for path in start_menu_paths {
            if let Ok(mut entries) = self.scan_directory(&path, "start_menu") {
                apps.append(&mut entries);
            }
        }

        Ok(apps)
    }

    /// 扫描注册表中的应用
    pub fn scan_registry(&mut self) -> Result<Vec<AppEntry>> {
        let mut apps = Vec::new();

        #[cfg(target_os = "windows")]
        {
            use winreg::enums::*;
            use winreg::RegKey;

            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            let uninstall_path = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";

            if let Ok(uninstall_key) = hklm.open_subkey(uninstall_path) {
                for app_key in uninstall_key.enum_keys() {
                    if let Ok(app_name) = app_key {
                        if let Ok(app) = self.parse_registry_app(&hklm, &format!("{}\\{}", uninstall_path, app_name)) {
                            if let Some(app) = app {
                                apps.push(app);
                            }
                        }
                    }
                }
            }
        }

        Ok(apps)
    }

    /// 扫描单个目录
    fn scan_directory(&self, path: &str, source: &str) -> Result<Vec<AppEntry>> {
        let mut apps = Vec::new();

        if !std::path::Path::new(path).exists() {
            return Ok(apps);
        }

        for entry in WalkDir::new(path)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let file_path = entry.path();
            if file_path.extension().and_then(|s| s.to_str()) == Some("lnk") {
                if let Some(app) = self.parse_lnk_file(file_path, source) {
                    apps.push(app);
                }
            } else if file_path.extension().and_then(|s| s.to_str()) == Some("exe") {
                if let Some(app) = self.parse_exe_file(file_path, source) {
                    apps.push(app);
                }
            }
        }

        Ok(apps)
    }

    /// 解析 LNK 快捷方式
    fn parse_lnk_file(&self, path: &std::path::Path, source: &str) -> Option<AppEntry> {
        // 简化的 LNK 解析（实际应该使用 shell COM 接口）
        let name = path.file_stem()?.to_str()?.to_string();

        Some(AppEntry {
            id: format!("lnk:{}", path.display()),
            name,
            exe_path: String::new(),
            icon_path: None,
            description: None,
            keywords: vec![],
            pinyin: None,
            launch_count: 0,
            last_accessed: None,
            source: source.to_string(),
            created_at: chrono::Utc::now().timestamp(),
        })
    }

    /// 解析 EXE 文件
    fn parse_exe_file(&self, path: &std::path::Path, source: &str) -> Option<AppEntry> {
        let name = path.file_stem()?.to_str()?.to_string();
        let exe_path = path.to_string_lossy().to_string();

        Some(AppEntry {
            id: format!("exe:{}", exe_path),
            name,
            exe_path,
            icon_path: None,
            description: None,
            keywords: vec![],
            pinyin: None,
            launch_count: 0,
            last_accessed: None,
            source: source.to_string(),
            created_at: chrono::Utc::now().timestamp(),
        })
    }

    /// 解析注册表应用
    fn parse_registry_app(&self, _hklm: &winreg::RegKey, _path: &str) -> Result<Option<AppEntry>> {
        // TODO: 实现注册表解析
        Ok(None)
    }

    /// 获取开始菜单路径
    fn get_start_menu_paths() -> Vec<String> {
        let mut paths = Vec::new();

        // 用户开始菜单
        if let Some(appdata) = dirs::next_data_dir() {
            paths.push(appdata.join("Microsoft/Windows/Start Menu/Programs").to_string_lossy().to_string());
        }

        // 全局开始菜单
        paths.push("C:/ProgramData/Microsoft/Windows/Start Menu/Programs".to_string());

        paths
    }

    /// 保存到数据库
    pub fn save_to_db(&mut self, apps: &[AppEntry]) -> Result<()> {
        let tx = self.db.transaction()?;

        for app in apps {
            tx.execute(
                "INSERT OR REPLACE INTO apps (id, name, exe_path, icon_path, description, keywords, pinyin, launch_count, last_accessed, source, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                [
                    &app.id,
                    &app.name,
                    &app.exe_path,
                    &app.icon_path.as_deref().unwrap_or(""),
                    &app.description.as_deref().unwrap_or(""),
                    &serde_json::to_string(&app.keywords).unwrap_or_default(),
                    &app.pinyin.as_deref().unwrap_or(""),
                    &app.launch_count.to_string(),
                    &app.last_accessed.map(|t| t.to_string()).unwrap_or_default(),
                    &app.source,
                    &app.created_at.to_string(),
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub exe_path: String,
    pub icon_path: Option<String>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    pub pinyin: Option<String>,
    pub launch_count: u32,
    pub last_accessed: Option<i64>,
    pub source: String,
    pub created_at: i64,
}
