//! 应用搜索引擎 - 索引安装的应用 + 启动项 + 启动文件夹
use crate::config::search as search_cfg;
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

    /// 仅供测试: 构造一个空 engine (不连真实 settings).
    pub fn new_empty_for_tests() -> Self {
        use crate::repositories::settings_repo::InMemorySettingsRepo;
        Self::new_empty(Arc::new(InMemorySettingsRepo::new(crate::models::Settings::default())))
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
                    score: search_cfg::APP_EMPTY_QUERY_SCORE,
                })
                .collect();
        }

        let mut scored: Vec<(f32, AppEntry)> = Vec::new();
        for app in cache.values() {
            let s = score_app_match(&q, &app.name, &app.path.to_string_lossy(), app.launch_count);
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

impl crate::engines::search_source::SearchSource for AppSearchEngine {
    fn name(&self) -> &'static str {
        "app"
    }
    fn category(&self) -> crate::models::SearchCategory {
        crate::models::SearchCategory::Apps
    }
    fn search(&self, query: &str, limit: u32) -> Vec<crate::models::SearchResult> {
        self.search(query, limit)
    }
    fn total(&self) -> usize {
        self.total()
    }
    fn category_weight(&self) -> f32 {
        search_cfg::CATEGORY_WEIGHT_APPS
    }
}

/// 纯函数: 计算单个 app 在给定 query 下的匹配分数.
///
/// 评分规则 (与 search.rs 拆分以方便单测):
/// - 完全匹配 (lowercase) → APP_SCORE_EXACT
/// - 前缀匹配 → APP_SCORE_PREFIX
/// - 子串匹配 → APP_SCORE_SUBSTR
/// - 多 token: 每个 token 命中 name 加 APP_SCORE_FUZZY, 命中 path 加 APP_SCORE_TOKEN
/// - launch_count 加权: launch_count * APP_LAUNCH_COUNT_WEIGHT
///
/// 返回 0 表示不匹配, 调用方应据此过滤. 规则与 config::search::APP_SCORE_*
/// 一一对应, 改阈值只动 config.
pub fn score_app_match(query_lower: &str, name: &str, path_lower: &str, launch_count: u32) -> f32 {
    let name_lower = name.to_lowercase();
    let mut s = 0.0;
    if name_lower == query_lower {
        s += search_cfg::APP_SCORE_EXACT;
    } else if name_lower.starts_with(query_lower) {
        s += search_cfg::APP_SCORE_PREFIX;
    } else if name_lower.contains(query_lower) {
        s += search_cfg::APP_SCORE_SUBSTR;
    }
    for term in query_lower.split_whitespace() {
        if name_lower.contains(term) {
            s += search_cfg::APP_SCORE_FUZZY;
        }
        if path_lower.contains(term) {
            s += search_cfg::APP_SCORE_TOKEN;
        }
    }
    s += (launch_count as f32) * search_cfg::APP_LAUNCH_COUNT_WEIGHT;
    s
}

fn category_of(path: &PathBuf, name: &str) -> String {
    let p = path.to_string_lossy().to_lowercase();
    let n = name.to_lowercase();

    // 表驱动: (关键词列表, 分类). 顺序敏感, 先匹配先返回.
    // 同一分类 (如 Development) 出现多次时, 合并为一行.
    // 新增分类 / 关键词只需改本表, 不动逻辑.
    for (keywords, category) in APP_NAME_CATEGORIES {
        if keywords.iter().any(|k| n.contains(k)) {
            return (*category).to_string();
        }
    }
    // 路径级分类 (典型: "microsoft" 路径 → System).
    for keyword in SYSTEM_PATH_KEYWORDS {
        if p.contains(keyword) {
            return "System".to_string();
        }
    }
    DEFAULT_APP_CATEGORY.to_string()
}

/// 名称 → 分类查表. 见 [`category_of`] 说明.
const APP_NAME_CATEGORIES: &[(&[&str], &str)] = &[
    (
        &[
            "chrome",
            "firefox",
            "edge",
        ],
        "Browser",
    ),
    (
        &[
            "vscode",
            "visualcodium",
            " intellij",
            "rider",
            "git",
            "tortoisegit",
        ],
        "Development",
    ),
    (
        &["wechat", "slack", "discord"],
        "Communication",
    ),
    (
        &["spotify", "music"],
        "Media",
    ),
];

/// 路径关键字 → System 分类.
const SYSTEM_PATH_KEYWORDS: &[&str] = &["microsoft"];

/// 未匹配任何规则的默认分类.
const DEFAULT_APP_CATEGORY: &str = "Applications";

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

#[cfg(test)]
mod tests {
    use super::*;

    /// 完全匹配得分最高, 任何模糊/前缀匹配都比不过.
    #[test]
    fn score_exact_match_returns_full() {
        // exact: "chrome" == "Chrome" (lowercase) → APP_SCORE_EXACT
        let exact = score_app_match("chrome", "Chrome", r"c:\program files\chrome\chrome.exe", 0);
        // prefix: "chrome" 是 "chrome browser" 的前缀 → APP_SCORE_PREFIX
        let prefix = score_app_match("chrome", "Chrome Browser", r"c:\apps\chrome\browser.exe", 0);
        // substr: "chrome" 是 "Google Chrome" 的子串 (但不是前缀) → APP_SCORE_SUBSTR
        let substr = score_app_match("chrome", "Google Chrome", r"c:\program files\google\chrome.exe", 0);
        let _fuzzy = score_app_match("chrome", "Chrm", r"c:\apps\chrm\chrm.exe", 0);
        assert!(exact >= search_cfg::APP_SCORE_EXACT, "完全匹配应至少给 EXACT 分数, got {exact}");
        assert!(prefix >= search_cfg::APP_SCORE_PREFIX, "前缀匹配应至少给 PREFIX 分数, got {prefix}");
        assert!(substr >= search_cfg::APP_SCORE_SUBSTR, "子串匹配应至少给 SUBSTR 分数, got {substr}");
        // 完全 > 前缀 > 子串 (按阈值从大到小)
        assert!(
            exact >= prefix && prefix >= substr,
            "完全({exact}) >= 前缀({prefix}) >= 子串({substr})"
        );
    }

    /// 多 token 搜索: 拆分后每段独立加分, 加 launch_count 权重.
    #[test]
    fn score_multi_term_and_launch_count() {
        let s0 = score_app_match("vs code", "Visual Studio Code", r"c:\program files\vs\code.exe", 0);
        let s_high = score_app_match(
            "vs code",
            "Visual Studio Code",
            r"c:\program files\vs\code.exe",
            100,
        );
        // launch_count=100 应再加 100*0.5=50
        assert!(
            (s_high - s0 - 100.0 * search_cfg::APP_LAUNCH_COUNT_WEIGHT).abs() < 0.01,
            "launch_count 加权差值异常: s0={s0} s_high={s_high}"
        );
        // 多 token: "vs" 和 "code" 各加一次 FUZZY (命中 name)
        assert!(s0 > 0.0);
    }

    /// 不匹配返回 0 (而非负数, 调用方据此过滤).
    #[test]
    fn score_no_match_returns_zero() {
        let s = score_app_match("zzz", "Chrome", r"c:\chrome\chrome.exe", 100);
        // 没任何匹配 → 0 + launch_count 加权 50
        assert_eq!(s, 100.0 * search_cfg::APP_LAUNCH_COUNT_WEIGHT);
    }

    /// path 命中 token 应加 APP_SCORE_TOKEN.
    #[test]
    fn score_path_token_match() {
        let s_path_hit = score_app_match(
            "office",
            "MyApp",
            r"c:\program files\microsoft office\office16\outlook.exe",
            0,
        );
        // name 不含 "office" 但 path 命中 → 0 + APP_SCORE_TOKEN
        assert!(
            (s_path_hit - search_cfg::APP_SCORE_TOKEN).abs() < 0.01,
            "path token 命中应得 APP_SCORE_TOKEN, got {s_path_hit}"
        );
    }

    // === category_of 表驱动查表测试 ===

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn category_browser_keyword_match() {
        assert_eq!(category_of(&p(r"c:\x"), "Google Chrome"), "Browser");
        assert_eq!(category_of(&p(r"c:\x"), "Firefox Nightly"), "Browser");
        assert_eq!(category_of(&p(r"c:\x"), "Microsoft Edge"), "Browser");
    }

    #[test]
    fn category_development_keyword_match() {
        // "vscode" 子串匹配 (实际 bin 名为 "Code - Insiders.exe" 时也常含 "vscode")
        assert_eq!(category_of(&p(r"c:\x"), "vscode.exe"), "Development");
        // " intellij" 带前导空格, 避免误匹配 "intelligence" 等
        assert_eq!(category_of(&p(r"c:\x"), "JetBrains IntelliJ"), "Development");
        // "git" 是 "github" 的子串, 也命中
        assert_eq!(category_of(&p(r"c:\x"), "GitHub Desktop"), "Development");
        // "git" 单词本身
        assert_eq!(category_of(&p(r"c:\tools"), "git"), "Development");
    }

    #[test]
    fn category_communication_match() {
        assert_eq!(category_of(&p(r"c:\x"), "WeChat"), "Communication");
        assert_eq!(category_of(&p(r"c:\x"), "Slack"), "Communication");
        assert_eq!(category_of(&p(r"c:\x"), "Discord"), "Communication");
    }

    #[test]
    fn category_media_match() {
        assert_eq!(category_of(&p(r"c:\x"), "Spotify"), "Media");
        // "musicbee" 包含 "music" 关键字
        assert_eq!(category_of(&p(r"c:\x"), "MusicBee"), "Media");
    }

    #[test]
    fn category_system_via_path_keyword() {
        // 名称没命中任何规则, 但路径含 "microsoft" → System
        assert_eq!(
            category_of(&p(r"c:\program files\microsoft\foo.exe"), "FooApp"),
            "System"
        );
    }

    #[test]
    fn category_default_applications() {
        // 名称 / 路径都不匹配 → Applications
        assert_eq!(
            category_of(&p(r"c:\random\app.exe"), "Some Random Tool"),
            "Applications"
        );
    }

    /// 顺序敏感: Browser 在 Development 之前, "GitHub Chrome" 仍为 Browser.
    #[test]
    fn category_order_sensitive_browser_wins() {
        assert_eq!(
            category_of(&p(r"c:\x"), "GitHub Chrome Edition"),
            "Browser"
        );
    }

    /// 大小写不敏感: 名称大写也能命中 (内部 to_lowercase).
    #[test]
    fn category_case_insensitive() {
        assert_eq!(category_of(&p(r"c:\x"), "CHROME"), "Browser");
        assert_eq!(category_of(&p(r"c:\x"), "vScOdE"), "Development");
    }
}
