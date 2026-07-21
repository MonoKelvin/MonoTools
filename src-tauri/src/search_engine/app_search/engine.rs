//! 应用搜索引擎 - 索引安装的应用 + 启动项 + 启动文件夹
use crate::core::config::search as search_cfg;
use crate::core::error::Result;
use crate::core::settings::SettingsRepo;
use crate::platform::windows::shell::resolve_shortcut;
use crate::platform::windows::special_shortcuts::get_special_shortcut;
use crate::search_engine::models::{AppEntry, ResultType, SearchAction, SearchResult};
use crate::utils::path::is_executable;
use crate::utils::pinyin::to_pinyin;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
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
        use crate::core::settings::InMemorySettingsRepo;
        Self::new_empty(Arc::new(InMemorySettingsRepo::new(
            crate::core::settings::Settings::default(),
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
    /// - `phase`: 当前阶段名（"common_start_menu" / "user_start_menu" / "desktop" / "registry"）
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
            // 阶段间 yield: 让 async runtime 处理其他任务 (前端事件、UI 渲染等).
            // 避免连续扫描阻塞 runtime 线程, 导致窗口拖动/滚动卡顿.
            tokio::task::yield_now().await;
        }

        // 从注册表 Uninstall 键发现更多应用 (HKLM + HKCU).
        let count = Self::scan_registry_apps(&self.cache);
        on_progress(count, "registry");
        tokio::task::yield_now().await;

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

            if let Some(sc) = special_sc {
                // 特殊快捷方式：直接加入，不需要检查目标是否存在
                let name = sc.display_name.to_string();
                let category = "System Tools".to_string();
                let (pinyin_initials, pinyin_full) = pinyin_of(&name);
                let entry = AppEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: name.clone(),
                    name_lower: name.to_lowercase(),
                    path: p.to_path_buf(), // 存 LNK 本身的路径，用于图标提取
                    icon_path: None,
                    category,
                    last_launched: None,
                    launch_count: 0,
                    alias: None,
                    is_special_shortcut: true,
                    special_command: Some(sc.launch_command.to_string()),
                    special_args: Some(sc.launch_args.iter().map(|s| s.to_string()).collect()),
                    pinyin_initials,
                    pinyin_full,
                    version: None,
                    file_types: Vec::new(),
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
            let (pinyin_initials, pinyin_full) = pinyin_of(&name);
            let entry = AppEntry {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.clone(),
                name_lower: name.to_lowercase(),
                path: target,
                icon_path: None,
                category,
                last_launched: None,
                launch_count: 0,
                alias: None,
                is_special_shortcut: false,
                special_command: None,
                special_args: None,
                pinyin_initials,
                pinyin_full,
                version: None,
                file_types: Vec::new(),
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
            let mut v: Vec<&AppEntry> = cache.values().collect();
            v.sort_by(|a, b| {
                b.launch_count
                    .cmp(&a.launch_count)
                    .then_with(|| a.name_lower.cmp(&b.name_lower))
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
                        category: crate::search_engine::models::SearchCategory::Apps,
                        result_type: app_type_of(&a),
                        action,
                        score: search_cfg::APP_EMPTY_QUERY_SCORE,
                    }
                })
                .collect();
        }

        let mut scored: Vec<(f32, AppEntry)> = Vec::new();
        for app in cache.values() {
            let s = score_app_match(
                &q,
                &app.name_lower, // 使用预计算的小写缓存, 避免重复 to_lowercase()
                &app.path.to_string_lossy(),
                app.launch_count,
                app.pinyin_initials.as_deref().unwrap_or(""),
                app.pinyin_full.as_deref().unwrap_or(""),
            );
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
                    category: crate::search_engine::models::SearchCategory::Apps,
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
    fn category(&self) -> crate::search_engine::models::SearchCategory {
        crate::search_engine::models::SearchCategory::Apps
    }
    fn search(&self, query: &str, limit: u32) -> Vec<crate::search_engine::models::SearchResult> {
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
/// - 拼音首字母命中 → PINYIN_SCORE_INITIALS (例: "wj" → "微信")
/// - 拼音全拼命中   → PINYIN_SCORE_FULL (例: "weixin" → "微信")
/// - 位置敏感加分: 匹配位置越靠前, 加分越高 (鼓励"开头匹配")
/// - launch_count 加权: 使用对数缩放, 避免高频应用垄断
///
/// 返回 0 表示不匹配, 调用方应据此过滤. 规则与 config::search::APP_SCORE_*
/// 一一对应, 改阈值只动 config.
pub fn score_app_match(
    query_lower: &str,
    name_lower: &str, // 预计算的小写名称, 避免调用方重复 to_lowercase()
    path_lower: &str,
    launch_count: u32,
    pinyin_initials: &str,
    pinyin_full: &str,
) -> f32 {
    let mut s = 0.0;
    let mut best_match_pos = usize::MAX;

    if name_lower == query_lower {
        s += search_cfg::APP_SCORE_EXACT;
        best_match_pos = 0;
    } else if name_lower.starts_with(query_lower) {
        s += search_cfg::APP_SCORE_PREFIX;
        best_match_pos = 0;
    } else if let Some(pos) = name_lower.find(query_lower) {
        s += search_cfg::APP_SCORE_SUBSTR;
        best_match_pos = pos;
    }

    // 位置敏感加分: 匹配越靠前, 额外加分越多
    if best_match_pos != usize::MAX {
        let pos_bonus = match best_match_pos {
            0 => 15.0,    // 开头匹配
            1..=3 => 8.0, // 前几个字符内
            _ => 3.0,     // 其他位置
        };
        s += pos_bonus;
    }

    for term in query_lower.split_whitespace() {
        if let Some(pos) = name_lower.find(term) {
            s += search_cfg::APP_SCORE_FUZZY;
            // term 级位置加分
            if pos <= 1 {
                s += 5.0;
            }
        }
        if path_lower.contains(term) {
            s += search_cfg::APP_SCORE_TOKEN;
        }
        // 拼音匹配: 仅当 term 长度 ≤ 拼音长度时才有意义 (避免 "w" 误匹配 "x")
        if !pinyin_initials.is_empty() && pinyin_initials.contains(term) {
            s += search_cfg::PINYIN_SCORE_INITIALS;
        }
        if !pinyin_full.is_empty() && pinyin_full.contains(term) {
            s += search_cfg::PINYIN_SCORE_FULL;
        }
    }

    // 启动次数使用对数缩放: ln(count + 1) * weight
    // 这样 1 次=0.35, 10次=1.2, 100次=2.3, 1000次=3.45
    // 避免 1000 次启动的应用永远排在 100 次前面.
    // 关键: 仅当已有其他匹配 (s > 0) 时才加权, 防止无名称匹配的应用
    // 仅凭高频启动就出现在搜索结果中.
    if s > 0.0 {
        let count_factor = ((launch_count as f32) + 1.0).ln();
        s += count_factor * search_cfg::APP_LAUNCH_COUNT_WEIGHT;
    }

    s
}

/// 计算名称的拼音 (首字母 + 完整拼音).
///
/// 纯函数, 无 I/O. 仅当名称含中文字符时才返回 Some, 否则返回 None
/// (避免对 "Chrome" 等纯英文应用做无用计算).
fn pinyin_of(name: &str) -> (Option<String>, Option<String>) {
    let info = to_pinyin(name);
    (info.initials, info.full)
}

fn category_of(path: &Path, name: &str) -> String {
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

/// 非应用程序可执行文件名模式 —— 匹配这些名称的文件不应作为应用索引.
/// 包含卸载程序、更新程序、辅助工具等用户不会直接启动的可执行文件.
const NON_APP_PATTERNS: &[&str] = &[
    "unins000.exe",
    "uninstall.exe",
    "uninstall_helper.exe",
    "uninstall_helper_64.exe",
    "unins.exe",
    "uninstaller.exe",
    "update.exe",
    "updater.exe",
    "_setup.exe",
    "setup.exe",
    "installer.exe",
    "install_helper.exe",
    "config.exe",
    "config64.exe",
    "diagnostics.exe",
    "diagnostic_tool.exe",
    "crash_reporter.exe",
    "crashpad_handler.exe",
    "crash_handler.exe",
    "debug.exe",
    "debugger.exe",
    "test.exe",
    "tests.exe",
    "benchmark.exe",
    "launcher_helper.exe",
    "migration_tool.exe",
    "repair.exe",
    "maintenance_tool.exe",
];

/// 判断给定路径是否位于系统目录中, 是则不应作为用户应用索引.
fn is_system_executable(path: &std::path::Path) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();
    if SYSTEM_DIR_KEYWORDS.iter().any(|kw| path_str.contains(kw)) {
        return true;
    }
    // 排除非应用程序可执行文件 (卸载程序、更新程序等).
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    NON_APP_PATTERNS.iter().any(|p| file_name == *p)
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
    /// 从注册表 Uninstall 键发现已安装应用.
    ///
    /// Windows 注册表存储了所有已安装程序的卸载信息, 包含 DisplayIcon 字段,
    /// 指向程序的主可执行文件. 这是 Start Menu 扫描的重要补充, 能发现那些
    /// 没有在开始菜单创建快捷方式的程序 (如 Steam 游戏、某些绿色软件等).
    ///
    /// 扫描范围:
    /// - HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*
    /// - HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*
    /// - HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\* (32-bit apps on 64-bit Windows)
    fn scan_registry_apps(cache: &RwLock<HashMap<String, AppEntry>>) -> usize {
        let mut batch: Vec<AppEntry> = Vec::new();

        // 需要扫描的注册表路径列表
        let registry_paths = [
            (
                "HKLM",
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
            ),
            (
                "HKCU",
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
            ),
            (
                "HKLM_WOW64",
                "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
            ),
        ];

        for (hive, subkey) in registry_paths {
            let _ = Self::scan_registry_key(hive, subkey, &mut batch);
        }

        if !batch.is_empty() {
            let mut cache_write = cache.write();
            for entry in batch {
                let key = entry.path.to_string_lossy().to_string();
                // 不覆盖已存在的条目 (Start Menu 扫描优先)
                cache_write.entry(key).or_insert(entry);
            }
        }

        cache.read().len()
    }

    /// 扫描单个注册表 Uninstall 键下的所有子键.
    #[cfg(windows)]
    fn scan_registry_key(
        hive: &str,
        _subkey: &str,
        batch: &mut Vec<AppEntry>,
    ) -> std::io::Result<()> {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use windows::core::{PCWSTR, PWSTR};
        use windows::Win32::System::Registry::{
            RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, HKEY, KEY_READ,
        };

        let hive_key = match hive {
            "HKLM" => windows::Win32::System::Registry::HKEY_LOCAL_MACHINE,
            "HKCU" => windows::Win32::System::Registry::HKEY_CURRENT_USER,
            "HKLM_WOW64" => windows::Win32::System::Registry::HKEY_LOCAL_MACHINE,
            _ => return Ok(()),
        };

        let wide_subkey: Vec<u16> = if hive == "HKLM_WOW64" {
            "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect()
        } else {
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect()
        };

        let mut hkey: HKEY = HKEY::default();
        let result = unsafe {
            RegOpenKeyExW(
                hive_key,
                PCWSTR(wide_subkey.as_ptr()),
                Some(0),
                KEY_READ,
                &mut hkey,
            )
        };
        if result.is_err() {
            log::debug!("[registry] failed to open {}: {:?}", hive, result);
            return Ok(());
        }

        let mut idx = 0u32;
        loop {
            let mut name_buf = [0u16; 256];
            let mut name_len = name_buf.len() as u32;
            let ret = unsafe {
                RegEnumKeyExW(
                    hkey,
                    idx,
                    Some(PWSTR(name_buf.as_mut_ptr())),
                    &mut name_len,
                    None,
                    None,
                    None,
                    None,
                )
            };
            if ret.is_err() {
                break;
            }

            let sub_name = OsString::from_wide(&name_buf[..name_len as usize])
                .to_string_lossy()
                .to_string();

            // 读取 DisplayIcon 和 DisplayName
            let mut hsub: HKEY = HKEY::default();
            let wide_sub: Vec<u16> = sub_name.encode_utf16().chain(std::iter::once(0)).collect();
            let open_result = unsafe {
                RegOpenKeyExW(
                    hkey,
                    PCWSTR(wide_sub.as_ptr()),
                    Some(0),
                    KEY_READ,
                    &mut hsub,
                )
            };
            if open_result.is_ok() {
                let display_icon = Self::read_registry_string(hsub, "DisplayIcon");
                let display_name = Self::read_registry_string(hsub, "DisplayName");
                unsafe {
                    let _ = RegCloseKey(hsub);
                }

                if let (Some(icon), Some(name)) = (display_icon, display_name) {
                    // DisplayIcon 可能包含逗号分隔的参数 (如 "path.exe,0"), 取第一部分
                    let exe_path = icon.split(',').next().unwrap_or(&icon).to_string();
                    let path = PathBuf::from(&exe_path);

                    // 有效性检查: 文件存在、不是系统目录、不是卸载程序
                    if path.exists() && !is_system_executable(&path) && !name.is_empty() {
                        let (pinyin_initials, pinyin_full) = pinyin_of(&name);
                        batch.push(AppEntry {
                            id: uuid::Uuid::new_v4().to_string(),
                            name: name.clone(),
                            name_lower: name.to_lowercase(),
                            path,
                            icon_path: None,
                            category: "Applications".to_string(),
                            last_launched: None,
                            launch_count: 0,
                            alias: None,
                            is_special_shortcut: false,
                            special_command: None,
                            special_args: None,
                            pinyin_initials,
                            pinyin_full,
                            version: None,
                            file_types: Vec::new(),
                        });
                    }
                }
            }

            idx += 1;
        }

        unsafe {
            let _ = RegCloseKey(hkey);
        }
        Ok(())
    }

    /// 读取注册表字符串值.
    #[cfg(windows)]
    fn read_registry_string(
        hkey: windows::Win32::System::Registry::HKEY,
        name: &str,
    ) -> Option<String> {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use windows::core::PCWSTR;
        use windows::Win32::System::Registry::{
            RegQueryValueExW, REG_EXPAND_SZ, REG_SZ, REG_VALUE_TYPE,
        };

        let wide_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let mut data_type: REG_VALUE_TYPE = REG_VALUE_TYPE::default();
        let mut buf = [0u8; 512];
        let mut buf_len = buf.len() as u32;

        let result = unsafe {
            RegQueryValueExW(
                hkey,
                PCWSTR(wide_name.as_ptr()),
                None,
                Some(&mut data_type),
                Some(buf.as_mut_ptr()),
                Some(&mut buf_len),
            )
        };
        if result.is_err() {
            return None;
        }

        if data_type == REG_SZ || data_type == REG_EXPAND_SZ {
            // 将字节转换为 u16 数组 (UTF-16 LE)
            let wide_data: Vec<u16> = (0..buf_len as usize / 2)
                .map(|i| u16::from_le_bytes([buf[i * 2], buf[i * 2 + 1]]))
                .collect();
            // 去除尾部 null
            let trimmed: Vec<u16> = wide_data
                .split(|&c| c == 0)
                .next()
                .unwrap_or(&wide_data)
                .to_vec();
            OsString::from_wide(&trimmed)
                .to_string_lossy()
                .to_string()
                .into()
        } else {
            None
        }
    }

    /// 非 Windows 平台: 注册表扫描返回空结果.
    #[cfg(not(windows))]
    fn scan_registry_key(
        _hive: &str,
        _subkey: &str,
        _batch: &mut Vec<AppEntry>,
    ) -> std::io::Result<()> {
        Ok(())
    }

    /// 非 Windows 平台: 注册表字符串读取返回 None.
    #[cfg(not(windows))]
    fn read_registry_string(_hkey: (), _name: &str) -> Option<String> {
        None
    }

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
                let (pinyin_initials, pinyin_full) = pinyin_of(display_name);
                batch.push(AppEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: display_name.to_string(),
                    name_lower: display_name.to_lowercase(),
                    path: path.clone(),
                    icon_path: None,
                    category: "System".to_string(),
                    last_launched: None,
                    launch_count: 0,
                    alias: None,
                    is_special_shortcut: false,
                    special_command: None,
                    special_args: None,
                    pinyin_initials,
                    pinyin_full,
                    version: None,
                    file_types: Vec::new(),
                });
                continue;
            }
            // 再在 syswow64 中查找
            let path = syswow64.join(exe_name);
            if path.exists() {
                let (pinyin_initials, pinyin_full) = pinyin_of(display_name);
                batch.push(AppEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: display_name.to_string(),
                    name_lower: display_name.to_lowercase(),
                    path: path.clone(),
                    icon_path: None,
                    category: "System".to_string(),
                    last_launched: None,
                    launch_count: 0,
                    alias: None,
                    is_special_shortcut: false,
                    special_command: None,
                    special_args: None,
                    pinyin_initials,
                    pinyin_full,
                    version: None,
                    file_types: Vec::new(),
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
                cache_write.entry(key).or_insert(entry);
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
        // exact: "chrome" == "chrome" (already lowercase) → APP_SCORE_EXACT
        let exact = score_app_match(
            "chrome",
            "chrome",
            r"c:\program files\chrome\chrome.exe",
            0,
            "",
            "",
        );
        // prefix: "chrome" 是 "chrome browser" 的前缀 → APP_SCORE_PREFIX
        let prefix = score_app_match(
            "chrome",
            "chrome browser",
            r"c:\apps\chrome\browser.exe",
            0,
            "",
            "",
        );
        // substr: "chrome" 是 "google chrome" 的子串 (但不是前缀) → APP_SCORE_SUBSTR
        let substr = score_app_match(
            "chrome",
            "google chrome",
            r"c:\program files\google\chrome.exe",
            0,
            "",
            "",
        );
        let _fuzzy = score_app_match("chrome", "chrm", r"c:\apps\chrm\chrm.exe", 0, "", "");
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

    /// 多 token 搜索: 拆分后每段独立加分, 加 launch_count 权重 (对数缩放).
    #[test]
    fn score_multi_term_and_launch_count() {
        let s0 = score_app_match(
            "vs code",
            "visual studio code",
            r"c:\program files\vs\code.exe",
            0,
            "",
            "",
        );
        let s_high = score_app_match(
            "vs code",
            "visual studio code",
            r"c:\program files\vs\code.exe",
            100,
            "",
            "",
        );
        // launch_count=100 的对数加权: ln(101) * 0.5 ≈ 2.307
        let expected_diff = (101.0_f32).ln() * search_cfg::APP_LAUNCH_COUNT_WEIGHT;
        assert!(
            (s_high - s0 - expected_diff).abs() < 0.01,
            "launch_count 加权差值异常: s0={s0} s_high={s_high} expected_diff={expected_diff}"
        );
        // 多 token: "vs" 和 "code" 各加一次 FUZZY (命中 name)
        assert!(s0 > 0.0);
    }

    /// 不匹配返回 0 (而非负数, 调用方据此过滤).
    /// 注意: 即使名称不匹配, launch_count 仍会贡献分数 (对数缩放).
    #[test]
    fn score_no_match_returns_zero() {
        let s = score_app_match("zzz", "chrome", r"c:\chrome\chrome.exe", 0, "", "");
        // 没任何匹配 + launch_count=0 → 0
        assert_eq!(s, 0.0);
    }

    /// launch_count 仅在名称有匹配时才加权, 无匹配时返回 0 (不污染结果).
    #[test]
    fn score_launch_count_only_with_name_match() {
        // 无名称匹配: launch_count 不应贡献分数, 返回 0
        let s_0 = score_app_match("zzz", "chrome", r"c:\chrome\chrome.exe", 0, "", "");
        let s_100 = score_app_match("zzz", "chrome", r"c:\chrome\chrome.exe", 100, "", "");
        assert_eq!(s_0, 0.0, "无匹配时 launch_count=0 应返回 0");
        assert_eq!(
            s_100, 0.0,
            "无匹配时 launch_count=100 也应返回 0 (不污染结果)"
        );

        // 有名称匹配: launch_count 应额外加权
        let s_match_0 = score_app_match("chrome", "chrome", r"c:\chrome\chrome.exe", 0, "", "");
        let s_match_100 = score_app_match("chrome", "chrome", r"c:\chrome\chrome.exe", 100, "", "");
        let expected_bonus = (101.0_f32).ln() * search_cfg::APP_LAUNCH_COUNT_WEIGHT;
        assert!(
            (s_match_100 - s_match_0 - expected_bonus).abs() < 0.01,
            "有匹配时 launch_count=100 应额外加 {expected_bonus}: got {s_match_0} vs {s_match_100}"
        );
        assert!(s_match_100 > s_match_0, "高频应用在有匹配时应排更前");
    }

    /// path 命中 token 应加 APP_SCORE_TOKEN.
    #[test]
    fn score_path_token_match() {
        let s_path_hit = score_app_match(
            "office",
            "myapp",
            r"c:\program files\microsoft office\office16\outlook.exe",
            0,
            "",
            "",
        );
        // name 不含 "office" 但 path 命中 → 0 + APP_SCORE_TOKEN
        assert!(
            (s_path_hit - search_cfg::APP_SCORE_TOKEN).abs() < 0.01,
            "path token 命中应得 APP_SCORE_TOKEN, got {s_path_hit}"
        );
    }

    /// 拼音首字母命中应加 PINYIN_SCORE_INITIALS.
    #[test]
    fn score_pinyin_initials_match() {
        // "wx" 命中 "微信" (initials = "wx")
        // name_lower 参数需要传入小写: "微信" 的小写仍是 "微信"
        let s = score_app_match("wx", "微信", r"c:\apps\wechat.exe", 0, "wx", "weixin");
        assert!(
            (s - search_cfg::PINYIN_SCORE_INITIALS).abs() < 0.01,
            "拼音首字母 'wx' 命中 '微信' 应得 PINYIN_SCORE_INITIALS, got {s}"
        );
    }

    /// 拼音全拼命中应加 PINYIN_SCORE_FULL.
    #[test]
    fn score_pinyin_full_match() {
        // "weixin" 命中 "微信" 的全拼 "weixin"
        let s = score_app_match("weixin", "微信", r"c:\apps\wechat.exe", 0, "wx", "weixin");
        assert!(
            (s - search_cfg::PINYIN_SCORE_FULL).abs() < 0.01,
            "拼音全拼 'weixin' 命中 '微信' 应得 PINYIN_SCORE_FULL, got {s}"
        );
    }

    /// 拼音首字母 < 全拼 ≤ 子串: 拼音辅助匹配不应超过名称子串.
    #[test]
    fn score_pinyin_ordering() {
        let initials = score_app_match("wx", "微信", "", 0, "wx", "weixin");
        let full = score_app_match("weixin", "微信", "", 0, "wx", "weixin");
        let substr = score_app_match("信", "微信", "", 0, "wx", "weixin");
        // initials (30) < full (50) == substr (50)
        assert!(
            initials <= full && full <= substr,
            "拼音首字母({initials}) <= 全拼({full}) <= 子串({substr})"
        );
    }

    /// 无拼音数据时 (纯英文应用), 拼音参数为空串, 不影响原有评分.
    #[test]
    fn score_ascii_app_unchanged_with_empty_pinyin() {
        let with_pinyin = score_app_match("chrome", "chrome", r"c:\chrome\chrome.exe", 0, "", "");
        let without = score_app_match("chrome", "chrome", r"c:\chrome\chrome.exe", 0, "", "");
        assert_eq!(with_pinyin, without, "空拼音不应改变纯英文应用的评分");
        assert!(with_pinyin >= search_cfg::APP_SCORE_EXACT);
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
