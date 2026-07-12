//! 应用搜索引擎 - 索引安装的应用 + 启动项 + 启动文件夹
use crate::error::Result;
use crate::models::{AppEntry, ResultType, SearchAction, SearchResult};
use crate::repositories::{
    SettingsRepo,
};
use crate::utils::path::is_executable;
use crate::platform::windows::shell::resolve_shortcut;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use walkdir::WalkDir;

pub struct AppSearchEngine {
    pub settings: Arc<dyn SettingsRepo>,
    pub cache: RwLock<HashMap<String, AppEntry>>, // key: path string
}

impl AppSearchEngine {
    /// 同步构造空缓存引擎，不执行任何 I/O。用于启动阶段避免阻塞 setup。
    /// 真正的索引构建通过 `refresh_index()` 在后台进行。
    pub fn new_empty(settings: Arc<dyn SettingsRepo>) -> Self {
        Self {
            settings,
            cache: RwLock::new(HashMap::new()),
        }
    }

    pub async fn new(settings: Arc<dyn SettingsRepo>) -> Result<Self> {
        Ok(Self::new_empty(settings))
    }

    /// 重建索引；扫描开始菜单、桌面快捷方式、启动文件夹
    pub async fn refresh_index(&self) -> Result<()> {
        let mut cache = self.cache.write();
        cache.clear();

        // 公共开始菜单
        if let Ok(common) = std::env::var("ProgramData") {
            let p = PathBuf::from(common)
                .join("Microsoft\\Windows\\Start Menu\\Programs");
            Self::scan_dir(&p, &mut cache);
        }
        // 用户开始菜单
        if let Ok(roaming) = std::env::var("APPDATA") {
            let p = PathBuf::from(roaming)
                .join("Microsoft\\Windows\\Start Menu\\Programs");
            Self::scan_dir(&p, &mut cache);
        }
        // 桌面
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let p = PathBuf::from(userprofile).join("Desktop");
            Self::scan_dir(&p, &mut cache);
        }
        // 自定义 path (removed in simplified mode)

        log::info!("App index: {} applications", cache.len());
        Ok(())
    }

    fn scan_dir(dir: &PathBuf, cache: &mut HashMap<String, AppEntry>) {
        if !dir.is_dir() {
            return;
        }
        for entry in WalkDir::new(dir)
            .max_depth(5)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase());
            let is_lnk = ext.as_deref() == Some("lnk");
            let is_exe = is_executable(p);
            if !(is_lnk || is_exe) {
                continue;
            }

            let target = if is_lnk {
                resolve_shortcut(&p.to_path_buf()).unwrap_or_else(|_| p.to_path_buf())
            } else {
                p.to_path_buf()
            };

            let name = p
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                continue;
            }
            let category = category_of(&target, &name);
            let entry = AppEntry {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                path: target,
                icon_path: None,
                category,
                last_launched: None,
                launch_count: 0,
                alias: None,
            };
            cache.insert(entry.path.to_string_lossy().to_string(), entry);
        }
    }

    pub fn search(&self, query: &str, limit: u32) -> Vec<SearchResult> {
        let cache = self.cache.read();
        if cache.is_empty() {
            return vec![];
        }
        let q = query.to_lowercase();
        if q.is_empty() {
            // 空查询: 返回全部应用 (按 launch_count 倒序), 不截断 limit.
            // 这样首屏"所有应用"分组能展示所有已索引的桌面应用, 而非前 N 个片段.
            let mut v: Vec<AppEntry> = cache.values().cloned().collect();
            v.sort_by(|a, b| {
                b.launch_count
                    .cmp(&a.launch_count)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
            return v
                .into_iter()
                .map(|a| SearchResult {
                    id: a.id.clone(),
                    title: a.name.clone(),
                    // 应用结果明确不暴露文件路径; 前端 AppResultItem 不显示副标题.
                    subtitle: String::new(),
                    // 应用没有次级元信息 (不使用文件大小等).
                    meta: None,
                    // icon 由前端走三级兜底: 后端 IPC / 静态资源 / Lucide 通用图标.
                    icon: None,
                    category: crate::models::SearchCategory::Apps,
                    result_type: app_type_of(&a.path),
                    action: SearchAction::Launch(a.path.to_string_lossy().to_string()),
                    score: 0.5,
                })
                .collect();
        }

        let mut scored: Vec<(f32, AppEntry)> = Vec::new();
        for app in cache.values() {
            let name = app.name.to_lowercase();
            let path = app.path.to_string_lossy().to_lowercase();

            let mut s = 0.0;
            if name == q {
                s += 100.0;
            } else if name.starts_with(&q) {
                s += 80.0;
            } else if name.contains(&q) {
                s += 50.0;
            }
            for term in q.split_whitespace() {
                if name.contains(term) {
                    s += 20.0;
                }
                if path.contains(term) {
                    s += 5.0;
                }
            }
            s += (app.launch_count as f32) * 0.5;
            if s > 0.0 {
                scored.push((s, app.clone()));
            }
        }
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit as usize);
        scored
            .into_iter()
            .map(|(s, a)| SearchResult {
                id: a.id.clone(),
                title: a.name.clone(),
                // 应用结果明确不暴露文件路径; 前端 AppResultItem 不显示副标题.
                subtitle: String::new(),
                meta: None,
                icon: None,
                category: crate::models::SearchCategory::Apps,
                result_type: app_type_of(&a.path),
                action: SearchAction::Launch(a.path.to_string_lossy().to_string()),
                score: s,
            })
            .collect()
    }

    pub fn record_launch(&self, key: &str) {
        let mut cache = self.cache.write();
        if let Some(app) = cache.get_mut(key) {
            app.launch_count += 1;
            app.last_launched = Some(chrono::Utc::now().timestamp());
        }
    }

    pub fn total(&self) -> usize {
        self.cache.read().len()
    }
}

fn category_of(path: &PathBuf, name: &str) -> String {
    let p = path.to_string_lossy().to_lowercase();
    let n = name.to_lowercase();
    if n.contains("chrome") || n.contains("firefox") || n.contains("edge") {
        "Browser".into()
    } else if n.contains("vscode") || n.contains("visualcodium") || n.contains(" intellij")
        || n.contains("rider")
    {
        "Development".into()
    } else if n.contains("git") || n.contains("tortoisegit") {
        "Development".into()
    } else if n.contains("wechat") || n.contains("slack") || n.contains("discord") {
        "Communication".into()
    } else if n.contains("spotify") || n.contains("music") {
        "Media".into()
    } else if p.contains("microsoft") {
        "System".into()
    } else {
        "Applications".into()
    }
}

fn app_type_of(path: &PathBuf) -> ResultType {
    let p = path.to_string_lossy().to_lowercase();

    if p.starts_with("c:\\windows\\") || p.starts_with("c:\\program files\\windowsapps\\") {
        return ResultType::SystemApp;
    }

    if p.contains("windowsapps\\") {
        return ResultType::UwpApp;
    }

    if p.starts_with("c:\\program files\\") || p.starts_with("c:\\program files (x86)\\") {
        if p.contains("microsoft") && (p.contains("office") || p.contains("windows")) {
            return ResultType::SystemApp;
        }
        return ResultType::UserApp;
    }

    if p.starts_with("c:\\users\\") {
        return ResultType::UserApp;
    }

    ResultType::UserApp
}
