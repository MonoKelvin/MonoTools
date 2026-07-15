//! 应用搜索引擎 - 索引安装的应用 + 启动项 + 启动文件夹
use crate::core::config::search as search_cfg;
use crate::core::error::Result;
use crate::search_engine::models::{AppEntry, ResultType, SearchAction, SearchResult};
use crate::platform::windows::shell::resolve_shortcut;
use crate::platform::windows::special_shortcuts::get_special_shortcut;
use crate::repositories::SettingsRepo;
use crate::utils::path::is_executable;
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
        Self::new_empty(Arc::new(InMemorySettingsRepo::new(
            crate::models::Settings::default(),
        )))
    }

    /// 重建索引；扫描开始菜单、桌面快捷方式、启动文件夹。
    /// 全量一次性完成，保持向后兼容。
    pub async fn refresh_index(&self) -> Result<()> {
        self.refresh_index_incremental(|_, _| {}).await
    }

    /// 增量式重建索引。每完成一个扫描目录就调用一次 `on_progress`，
    /// 调用方可通过 IPC 事件把"已就绪 N 个应用"实时推给前端，
    /// 让用户一启动就能看到内容逐步出现，而非等全部扫完才显示。
    ///
    /// `on_progress(count, phase)`:
    /// - `count`: 当前已索引的应用总数
    /// - `phase`: 当前阶段名（"common_start_menu" / "user_start_menu" / "desktop"）
    pub async fn refresh_index_incremental<F>(&self, mut on_progress: F) -> Result<()>
    where
        F: FnMut(usize, &str) + Send + 'static,
    {
        // 先清空缓存
        {
            let mut cache = self.cache.write();
            cache.clear();
        }

        // 分阶段扫描，每阶段结束后释放写锁并通知进度，
        // 这样读侧（search）能立即拿到已就绪的部分结果。
        let phases: [(&str, Option<PathBuf>); 3] = [
            (
                "common_start_menu",
                std::env::var("ProgramData")
                    .ok()
                    .map(|v| PathBuf::from(v).join("Microsoft\\Windows\\Start Menu\\Programs")),
            ),
            (
                "user_start_menu",
                std::env::var("APPDATA")
                    .ok()
                    .map(|v| PathBuf::from(v).join("Microsoft\\Windows\\Start Menu\\Programs")),
            ),
            (
                "desktop",
                std::env::var("USERPROFILE")
                    .ok()
                    .map(|v| PathBuf::from(v).join("Desktop")),
            ),
        ];

        for (phase, dir) in phases {
            if let Some(dir) = dir {
                let count = Self::scan_dir_and_count(&dir, &self.cache);
                on_progress(count, phase);
            }
        }

        // 额外添加常用系统应用（白名单）
        let count = Self::add_system_apps(&self.cache);
        on_progress(count, "system_apps");

        let total = self.total();
        log::info!("App index: {} applications", total);
        Ok(())
    }

    /// 扫描单个目录，把结果追加到缓存中，返回扫描后的总应用数。
    /// 使用写锁只保护"插入"这一临界区，使扫描 I/O 与读缓存并行。
    fn scan_dir_and_count(dir: &PathBuf, cache: &RwLock<HashMap<String, AppEntry>>) -> usize {
        if !dir.is_dir() {
            return cache.read().len();
        }

        // 先在本地收集本目录的所有条目，再一次性写入缓存，
        // 避免逐次加锁带来的开销。应用数量通常不大，本地缓冲完全 OK。
        let mut batch: Vec<AppEntry> = Vec::new();

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

            // 检查是否是特殊快捷方式（不指向真实文件的 .lnk，如"运行"）
            let special_sc = if is_lnk {
                get_special_shortcut(p, &target)
            } else {
                None
            };

            if special_sc.is_some() {
                // 特殊快捷方式：直接加入，不需要检查目标是否存在
                let sc = special_sc.unwrap();
                let name = sc.display_name.to_string();
                let category = "System Tools".to_string();
                let entry = AppEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    name,
                    path: p.to_path_buf(), // 存 LNK 本身的路径，用于图标提取
                    icon_path: None,
                    category,
                    last_launched: None,
                    launch_count: 0,
                    alias: None,
                    is_special_shortcut: true,
                    special_command: Some(sc.launch_command.to_string()),
                    special_args: Some(sc.launch_args.iter().map(|s| s.to_string()).collect()),
                };
                batch.push(entry);
                continue;
            }

            // 有效性校验: 目标路径必须存在, 避免列出已卸载的程序残留 LNK.
            // 同时排除系统目录中的可执行文件, 这些不是用户应用.
            if !target.exists() {
                continue;
            }
            if is_system_executable(&target) {
                continue;
            }

            // 应用名称：对于 LNK 文件，使用目标程序的名称（避免 LNK 文件名误导），
            // 但仍保留 LNK 自己的 file_stem 作为 fallback（防止目标解析失败）。
            let name = if is_lnk {
                target
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| {
                        p.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_string()
                    })
            } else {
                p.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string()
            };
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
                is_special_shortcut: false,
                special_command: None,
                special_args: None,
            };
            batch.push(entry);
        }

        if !batch.is_empty() {
            let mut cache_write = cache.write();
            for entry in batch {
                cache_write.insert(entry.path.to_string_lossy().to_string(), entry);
            }
        }

        cache.read().len()
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
                .map(|a| {
                    let action = if a.is_special_shortcut {
                        SearchAction::Run {
                            command: a.special_command.clone().unwrap_or_default(),
                            args: a.special_args.clone().unwrap_or_default(),
                        }
                    } else {
                        SearchAction::Launch(a.path.to_string_lossy().to_string())
                    };
                    // 普通应用 subtitle 为空 (AppResultItem 不显示路径),
                    // 特殊快捷方式 subtitle 存 .lnk 文件路径 (用于 tooltip / 右键菜单).
                    let subtitle = if a.is_special_shortcut {
                        a.path.to_string_lossy().to_string()
                    } else {
                        String::new()
                    };
                    SearchResult {
                        id: a.id.clone(),
                        title: a.name.clone(),
                        subtitle,
                        meta: None,
                        icon: None,
                        category: crate::models::SearchCategory::Apps,
                        result_type: app_type_of(&a),
                        action,
                        score: search_cfg::APP_EMPTY_QUERY_SCORE,
                    }
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
            .map(|(s, a)| {
                let action = if a.is_special_shortcut {
                    SearchAction::Run {
                        command: a.special_command.clone().unwrap_or_default(),
                        args: a.special_args.clone().unwrap_or_default(),
                    }
                } else {
                    SearchAction::Launch(a.path.to_string_lossy().to_string())
                };
                let subtitle = if a.is_special_shortcut {
                    a.path.to_string_lossy().to_string()
                } else {
                    String::new()
                };
                SearchResult {
                    id: a.id.clone(),
                    title: a.name.clone(),
                    subtitle,
                    meta: None,
                    icon: None,
                    category: crate::models::SearchCategory::Apps,
                    result_type: app_type_of(&a),
                    action,
                    score: s,
                }
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

impl crate::search_engine::search_source::SearchSource for AppSearchEngine {
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
    (&["chrome", "firefox", "edge"], "Browser"),
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
    (&["wechat", "slack", "discord"], "Communication"),
    (&["spotify", "music"], "Media"),
];

/// 路径关键字 → System 分类.
const SYSTEM_PATH_KEYWORDS: &[&str] = &["microsoft"];

/// 未匹配任何规则的默认分类.
const DEFAULT_APP_CATEGORY: &str = "Applications";

/// 系统目录列表. 这些目录中的可执行文件不是用户应用, 应从应用索引中排除
const SYSTEM_DIR_KEYWORDS: &[&str] = &[
    "\\windows\\system32\\",
    "\\windows\\syswow64\\",
    "\\windows\\winsxs\\",
    "\\windows\\servicing\\",
    "\\windows\\softwaredistribution\\",
    "\\windows\\assembly\\",
];

/// 判断给定路径是否位于系统目录中, 是则不应作为用户应用索引.
fn is_system_executable(path: &std::path::Path) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();
    SYSTEM_DIR_KEYWORDS.iter().any(|kw| path_str.contains(kw))
}

fn app_type_of(app: &AppEntry) -> ResultType {
    // 特殊快捷方式（运行、控制面板等）统一标记为系统应用
    if app.is_special_shortcut {
        return ResultType::SystemApp;
    }

    let p = app.path.to_string_lossy().to_lowercase();

    if p.contains("windowsapps\\") {
        return ResultType::UwpApp;
    }

    if p.starts_with("c:\\windows\\") {
        return ResultType::SystemApp;
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

/// 常用系统应用白名单 —— 这些应用虽然在系统目录中，但用户经常使用，
/// 应该被索引并标记为 system-app。直接从 system32 目录中添加，
/// 避免 WalkDir 扫描整个 system32 导致索引大量无用的系统工具。
const SYSTEM_APP_WHITELIST: &[(&str, &str)] = &[
    ("notepad.exe", "记事本"),
    ("calc.exe", "计算器"),
    ("mspaint.exe", "画图"),
    ("explorer.exe", "文件资源管理器"),
    ("taskmgr.exe", "任务管理器"),
    ("cmd.exe", "命令提示符"),
    ("powershell.exe", "Windows PowerShell"),
    ("control.exe", "控制面板"),
    ("ms-settings:", "设置"),
    ("notepad++.exe", "Notepad++"),
    ("write.exe", "写字板"),
    ("mstsc.exe", "远程桌面连接"),
    ("mmc.exe", "管理控制台"),
    ("devmgmt.msc", "设备管理器"),
    ("diskmgmt.msc", "磁盘管理"),
    ("services.msc", "服务"),
    ("taskschd.msc", "任务计划程序"),
    ("eventvwr.msc", "事件查看器"),
    ("perfmon.msc", "性能监视器"),
    ("resmon.exe", "资源监视器"),
    ("msinfo32.exe", "系统信息"),
    ("dxdiag.exe", "DirectX 诊断工具"),
    ("cleanmgr.exe", "磁盘清理"),
    ("dfrgui.exe", "磁盘碎片整理"),
    ("charmap.exe", "字符映射表"),
    ("snippingtool.exe", "截图工具"),
    ("magnify.exe", "放大镜"),
    ("osk.exe", "屏幕键盘"),
    ("narrator.exe", "讲述人"),
];

impl AppSearchEngine {
    /// 添加常用系统应用到缓存中。返回添加后的总应用数。
    ///
    /// 为什么不用 WalkDir 扫描系统目录:
    /// - system32 中有上千个 exe, 绝大多数是用户永远不会手动启动的系统工具
    /// - 扫描全部会导致应用列表杂乱无章, 搜索质量下降
    /// - 白名单方式更可控, 只添加用户真正可能用到的系统应用
    fn add_system_apps(cache: &RwLock<HashMap<String, AppEntry>>) -> usize {
        let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
        let system32 = PathBuf::from(&system_root).join("System32");
        let syswow64 = PathBuf::from(&system_root).join("SysWOW64");

        let mut batch: Vec<AppEntry> = Vec::new();

        for (exe_name, display_name) in SYSTEM_APP_WHITELIST {
            // 先在 system32 中查找
            let path = system32.join(exe_name);
            if path.exists() {
                batch.push(AppEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: display_name.to_string(),
                    path: path.clone(),
                    icon_path: None,
                    category: "System".to_string(),
                    last_launched: None,
                    launch_count: 0,
                    alias: None,
                    is_special_shortcut: false,
                    special_command: None,
                    special_args: None,
                });
                continue;
            }
            // 再在 syswow64 中查找
            let path = syswow64.join(exe_name);
            if path.exists() {
                batch.push(AppEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: display_name.to_string(),
                    path: path.clone(),
                    icon_path: None,
                    category: "System".to_string(),
                    last_launched: None,
                    launch_count: 0,
                    alias: None,
                    is_special_shortcut: false,
                    special_command: None,
                    special_args: None,
                });
            }
        }

        // 尝试添加 UWP 应用（从开始菜单中获取，因为 UWP 应用通常在开始菜单中有快捷方式）
        // 注意: UWP 应用的真实 exe 在 WindowsApps 目录下, 需要通过快捷方式访问

        if !batch.is_empty() {
            let mut cache_write = cache.write();
            for entry in batch {
                let key = entry.path.to_string_lossy().to_string();
                // 避免覆盖已有的同名应用（用户可能自己安装了其他版本）
                if !cache_write.contains_key(&key) {
                    cache_write.insert(key, entry);
                }
            }
        }

        cache.read().len()
    }
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
        let substr = score_app_match(
            "chrome",
            "Google Chrome",
            r"c:\program files\google\chrome.exe",
            0,
        );
        let _fuzzy = score_app_match("chrome", "Chrm", r"c:\apps\chrm\chrm.exe", 0);
        assert!(
            exact >= search_cfg::APP_SCORE_EXACT,
            "完全匹配应至少给 EXACT 分数, got {exact}"
        );
        assert!(
            prefix >= search_cfg::APP_SCORE_PREFIX,
            "前缀匹配应至少给 PREFIX 分数, got {prefix}"
        );
        assert!(
            substr >= search_cfg::APP_SCORE_SUBSTR,
            "子串匹配应至少给 SUBSTR 分数, got {substr}"
        );
        // 完全 > 前缀 > 子串 (按阈值从大到小)
        assert!(
            exact >= prefix && prefix >= substr,
            "完全({exact}) >= 前缀({prefix}) >= 子串({substr})"
        );
    }

    /// 多 token 搜索: 拆分后每段独立加分, 加 launch_count 权重.
    #[test]
    fn score_multi_term_and_launch_count() {
        let s0 = score_app_match(
            "vs code",
            "Visual Studio Code",
            r"c:\program files\vs\code.exe",
            0,
        );
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
        assert_eq!(
            category_of(&p(r"c:\x"), "JetBrains IntelliJ"),
            "Development"
        );
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
        assert_eq!(category_of(&p(r"c:\x"), "GitHub Chrome Edition"), "Browser");
    }

    /// 大小写不敏感: 名称大写也能命中 (内部 to_lowercase).
    #[test]
    fn category_case_insensitive() {
        assert_eq!(category_of(&p(r"c:\x"), "CHROME"), "Browser");
        assert_eq!(category_of(&p(r"c:\x"), "vScOdE"), "Development");
    }
}
