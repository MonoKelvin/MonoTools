//! Recommend - 智能推荐模块
//!
//! 完全独立的推荐模块，不依赖任何业务模块。
//! 删除本目录或禁用 feature 后不影响其他功能。
//!
//! # 架构
//!
//! ```text
//!                 ┌───────────────────────────┐
//!                 │      RecommendService     │
//!                 │     (对外统一入口)         │
//!                 └─────────────┬─────────────┘
//!                               │
//!          ┌────────────────────┼────────────────────┐
//!          │                    │                    │
//! ┌────────▼────────┐  ┌────────▼─────────┐  ┌──────▼───────┐
//! │  Items Store    │  │  Usage Stats     │  │  Engines     │
//! │ (候选项目数据)   │  │ (使用统计)        │  │ (评分引擎)    │
//! └─────────────────┘  └──────────────────┘  └──────┬───────┘
//!                                                    │
//!                                          ┌─────────┴─────────┐
//!                                          │                   │
//!                                    ┌─────▼─────┐       ┌─────▼──────┐
//!                                    │ RuleEngine │       │ PyEngine   │
//!                                    │ (保底排序)  │       │ (Python增强)│
//!                                    └───────────┘       └────────────┘
//! ```
//!
//! # Feature Flag
//!
//! - `recommend` - 启用推荐模块（默认启用）
//! - `recommend-py` - 启用 Python 增强推荐（自动检测，失败降级）
//!
//! # 核心 API
//!
//! ```rust,ignore
//! use recommend::RecommendService;
//!
//! let service = RecommendService::new(config);
//! service.set_items(items).await;      // 设置候选项目
//! let scores = service.get_scores(&context).await;  // 获取推荐
//! service.record_launch("app_id").await;  // 记录启动
//! ```

pub mod engine;
pub mod types;
pub mod window_monitor;

#[cfg(feature = "recommend-py")]
pub mod py_engine;

pub mod ipc;

pub use engine::RecommendEngine;
pub use types::{
    FeedbackEvent, FeedbackType, RecommendConfig, RecommendContext, RecommendItem, RecommendStatus,
};
pub use window_monitor::{ActiveAppEntry, WindowMonitorState};

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use window_monitor::WindowMonitor;

/// 推荐服务 - 对外统一入口
///
/// 完全独立，不依赖任何业务模块。
/// 所有数据通过 API 注入，所有状态内部维护。
pub struct RecommendService {
    config: RecommendConfig,

    /// 候选项目（外部设置）
    items: RwLock<Vec<RecommendItem>>,

    /// 使用统计（内部维护）
    usage_stats: RwLock<HashMap<String, UsageStat>>,

    /// 窗口监控（可选，默认开启）
    window_monitor: Arc<WindowMonitor>,

    /// Python 引擎（可选，自动检测可用性）
    #[cfg(feature = "recommend-py")]
    py_engine: RwLock<Option<Arc<py_engine::PyRecommendEngine>>>,

    /// 运行状态
    status: RwLock<RecommendStatus>,
}

/// 使用统计记录
#[derive(Debug, Clone, Default)]
struct UsageStat {
    launch_count: u32,
    last_launched: Option<i64>,
}

impl RecommendService {
    /// 创建推荐服务
    pub fn new(config: RecommendConfig) -> Self {
        let enabled = config.enabled;
        Self {
            config,
            items: RwLock::new(Vec::new()),
            usage_stats: RwLock::new(HashMap::new()),
            window_monitor: Arc::new(WindowMonitor::new()),
            #[cfg(feature = "recommend-py")]
            py_engine: RwLock::new(None),
            status: RwLock::new(RecommendStatus {
                enabled,
                ..Default::default()
            }),
        }
    }

    /// 获取配置
    pub fn config(&self) -> &RecommendConfig {
        &self.config
    }

    /// 设置/替换所有候选项目
    pub async fn set_items(&self, items: Vec<RecommendItem>) {
        let mut items_guard = self.items.write().await;
        *items_guard = items;
        log::info!("[recommend] 候选项目已更新，共 {} 个", items_guard.len());
    }

    /// 获取当前候选项目数量
    pub async fn item_count(&self) -> usize {
        self.items.read().await.len()
    }

    /// 记录一次启动（更新使用统计）
    pub async fn record_launch(&self, item_id: &str) {
        let mut stats = self.usage_stats.write().await;
        let stat = stats.entry(item_id.to_string()).or_default();
        stat.launch_count += 1;
        stat.last_launched = Some(chrono::Utc::now().timestamp());

        let mut status = self.status.write().await;
        status.total_feedbacks += 1;
    }

    /// 更新前台应用信息（窗口监控调用）
    pub async fn update_foreground(&self, path: String, title: String) {
        self.window_monitor.update(path, title).await;
    }

    /// 获取窗口监控状态快照
    pub async fn window_monitor_state(&self) -> WindowMonitorState {
        self.window_monitor.snapshot().await
    }

    /// 获取窗口监控器引用（内部使用）
    pub(crate) fn window_monitor(&self) -> &Arc<WindowMonitor> {
        &self.window_monitor
    }

    /// 获取推荐分数
    ///
    /// 优先使用 Python 引擎，失败或不可用时降级到简单排序。
    pub async fn get_scores(&self, context: &RecommendContext) -> Vec<(String, f32)> {
        if !self.config.enabled {
            return Vec::new();
        }

        let items = self.items.read().await;
        if items.is_empty() {
            return Vec::new();
        }

        // 合并使用统计到 items
        let stats = self.usage_stats.read().await;
        let enriched_items: Vec<RecommendItem> = items
            .iter()
            .map(|item| {
                let mut enriched = item.clone();
                if let Some(stat) = stats.get(&item.id) {
                    enriched.launch_count = stat.launch_count;
                    enriched.last_launched = stat.last_launched;
                }
                enriched
            })
            .collect();

        drop(stats);
        drop(items);

        // 尝试 Python 引擎
        #[cfg(feature = "recommend-py")]
        {
            let py_guard = self.py_engine.read().await;
            if let Some(py_engine) = py_guard.as_ref() {
                match py_engine.get_scores(&enriched_items, context).await {
                    Ok(scores) => {
                        let result = self.post_process(scores);
                        self.incr_recommend_count().await;
                        return result;
                    }
                    Err(e) => {
                        log::warn!("[recommend] Python 引擎失败，降级: {}", e);
                    }
                }
            }
        }

        // 降级：简单排序（启动次数 + 最近使用）
        let scores = simple_score(&enriched_items);
        let result = self.post_process(scores);
        self.incr_recommend_count().await;
        result
    }

    /// 上报用户反馈
    pub async fn report_feedback(&self, feedback: FeedbackEvent) {
        let mut status = self.status.write().await;
        status.total_feedbacks += 1;
        drop(status);

        // 点击反馈 = 记录启动
        if matches!(
            feedback.feedback_type,
            FeedbackType::Click | FeedbackType::Pin
        ) {
            self.record_launch(&feedback.item_id).await;
        }

        // Python 引擎也上报
        #[cfg(feature = "recommend-py")]
        {
            let py_guard = self.py_engine.read().await;
            if let Some(py_engine) = py_guard.as_ref() {
                if let Err(e) = py_engine.report_feedback(&feedback).await {
                    log::warn!("[recommend] 上报反馈到 Python 失败: {}", e);
                }
            }
        }
    }

    /// 获取当前状态
    pub async fn status(&self) -> RecommendStatus {
        let mut status = self.status.read().await.clone();

        #[cfg(feature = "recommend-py")]
        {
            let py_guard = self.py_engine.read().await;
            status.py_engine_available = py_guard.is_some();
        }

        status
    }

    /// 检查 Python 引擎是否可用
    pub async fn has_py_engine(&self) -> bool {
        #[cfg(feature = "recommend-py")]
        {
            self.py_engine.read().await.is_some()
        }
        #[cfg(not(feature = "recommend-py"))]
        {
            false
        }
    }

    /// 设置 Python 引擎（内部使用，初始化时调用）
    #[cfg(feature = "recommend-py")]
    pub async fn set_py_engine(&self, engine: py_engine::PyRecommendEngine) {
        let mut py_guard = self.py_engine.write().await;
        *py_guard = Some(Arc::new(engine));
        log::info!("[recommend] Python 引擎已就绪");
    }

    // --- 内部方法 ---

    fn post_process(&self, mut scores: Vec<(String, f32)>) -> Vec<(String, f32)> {
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(self.config.max_results);
        scores
    }

    async fn incr_recommend_count(&self) {
        let mut status = self.status.write().await;
        status.total_recommendations += 1;
    }
}

/// 简单评分：启动次数（对数缩放） + 最近使用时间衰减
fn simple_score(items: &[RecommendItem]) -> Vec<(String, f32)> {
    let now = chrono::Utc::now().timestamp();

    items
        .iter()
        .map(|item| {
            let mut score = 0.0;

            // 启动次数（对数缩放，避免高频应用垄断）
            score += ((item.launch_count as f32) + 1.0).ln() * 2.0;

            // 最近使用时间衰减（越近分越高）
            if let Some(last) = item.last_launched {
                let hours = ((now - last).max(0) as f32) / 3600.0;
                let decay = (-hours * 0.05).exp();
                score += decay * 5.0;
            }

            (item.id.clone(), score)
        })
        .collect()
}

// ============================================================
// 模块初始化入口
// ============================================================

/// 初始化推荐模块 - 基础版本（无 Python）
///
/// 只注册基础服务，不启动任何额外功能。
pub fn init<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    use tauri::Manager;
    let config = RecommendConfig::default();
    let service = Arc::new(RecommendService::new(config));
    app.manage(service);
    log::info!("[recommend] 模块初始化完成");
}

/// 推荐模块初始化配置
///
/// 所有外部依赖通过此配置注入，遵循依赖倒置原则。
pub struct RecommendInitConfig {
    /// 是否启用窗口监控（默认 true）
    pub enable_window_monitor: bool,
}

impl Default for RecommendInitConfig {
    fn default() -> Self {
        Self {
            enable_window_monitor: true,
        }
    }
}

/// 完整初始化推荐模块（推荐使用）
///
/// 包含：
/// - 基础服务注册
/// - Python 引擎自动检测与初始化（如果启用了 recommend-py feature）
/// - 窗口监控（可选，默认开启）
///
/// # 注意
/// Python 引擎初始化是异步的，启动失败会自动降级，不影响使用。
pub fn init_full<R: tauri::Runtime + 'static>(
    app: &tauri::AppHandle<R>,
    config: RecommendInitConfig,
) {
    use tauri::Manager;

    // 先注册基础服务
    init(app);

    log::info!("[recommend] 完整初始化开始");

    // 获取已注册的服务
    let service = app.state::<Arc<RecommendService>>().inner().clone();

    // 窗口监控
    if config.enable_window_monitor {
        let monitor = service.window_monitor().clone();
        window_monitor::start_window_monitor(app, service, monitor);
        log::info!("[recommend] 窗口监控已启动");
    }

    // Python 引擎（可选）
    #[cfg(feature = "recommend-py")]
    {
        let app_handle = app.clone();
        tauri::async_runtime::spawn(async move {
            init_python_engine(&app_handle).await;
        });
    }
}

// ============================================================
// Python 引擎初始化（可选）
// ============================================================

#[cfg(feature = "recommend-py")]
async fn init_python_engine<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    use tauri::Manager;

    log::info!("[recommend-py] 正在初始化 Python 引擎...");

    // 查找 Python 解释器
    let python_path = find_python_executable();
    log::info!("[recommend-py] Python 解释器: {}", python_path);

    // 解析脚本路径
    let script_path = resolve_python_script_path(app);
    log::info!("[recommend-py] 脚本路径: {}", script_path);

    // 启动 PyBridge
    let config = crate::pybridge::PyBridgeConfig {
        enabled: true,
        python_path,
        script_path,
        startup_timeout_ms: 10000,
        ..Default::default()
    };

    crate::pybridge::init(app, config);

    let bridge = match app
        .try_state::<std::sync::Arc<crate::pybridge::PyBridge>>()
        .map(|s| s.inner().clone())
    {
        Some(b) => b,
        None => {
            log::warn!("[recommend-py] PyBridge 未注册，跳过 Python 引擎");
            return;
        }
    };

    match bridge.start().await {
        Ok(_) => {
            log::info!("[recommend-py] PyBridge 启动成功");

            // 等待 items 设置（最多等 30 秒）
            let service = app
                .state::<std::sync::Arc<RecommendService>>()
                .inner()
                .clone();
            let mut waited = 0;
            while service.item_count().await < 5 && waited < 300 {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                waited += 1;
            }

            let item_count = service.item_count().await;
            if item_count == 0 {
                log::warn!("[recommend-py] 没有候选项目，Python 引擎暂不初始化");
                return;
            }

            log::info!("[recommend-py] 候选项目就绪，共 {} 个", item_count);

            // 从 service 获取 items + stats 来初始化 Python 引擎
            let items = {
                let items_guard = service.items.read().await;
                let stats_guard = service.usage_stats.read().await;
                items_guard
                    .iter()
                    .map(|item| {
                        let mut enriched = item.clone();
                        if let Some(stat) = stats_guard.get(&item.id) {
                            enriched.launch_count = stat.launch_count;
                            enriched.last_launched = stat.last_launched;
                        }
                        enriched
                    })
                    .collect::<Vec<_>>()
            };

            let py_engine = crate::recommend::py_engine::PyRecommendEngine::new(bridge.clone());
            match py_engine.initialize(&items).await {
                Ok(_) => {
                    service.set_py_engine(py_engine).await;
                    log::info!("[recommend-py] ✅ Python 推荐引擎已就绪");
                }
                Err(e) => {
                    log::warn!("[recommend-py] Python 引擎初始化失败，降级: {}", e);
                }
            }
        }
        Err(e) => {
            log::warn!("[recommend-py] PyBridge 启动失败，降级: {}", e);
        }
    }
}

#[cfg(feature = "recommend-py")]
fn resolve_python_script_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> String {
    use std::path::PathBuf;
    use tauri::Manager;

    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(path) = app.path().resolve(
        "python/pybridge/server.py",
        tauri::path::BaseDirectory::Resource,
    ) {
        candidates.push(path);
    }

    if let Ok(exe_path) = std::env::current_exe() {
        let mut p = exe_path.clone();
        for _ in 0..4 {
            p.pop();
        }
        candidates.push(p.join("python/pybridge/server.py"));
    }

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("python/pybridge/server.py"));
    }

    candidates.push(PathBuf::from("python/pybridge/server.py"));

    for path in &candidates {
        if path.exists() {
            return path.to_string_lossy().to_string();
        }
    }

    log::warn!("[recommend-py] 未找到 server.py");
    candidates[0].to_string_lossy().to_string()
}

#[cfg(feature = "recommend-py")]
fn find_python_executable() -> String {
    let candidates = if cfg!(windows) {
        vec!["python", "py", "python3"]
    } else {
        vec!["python3", "python", "py"]
    };

    for cmd in &candidates {
        if let Ok(output) = std::process::Command::new(cmd).arg("--version").output() {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                log::info!("[recommend-py] 找到 Python: {} ({})", cmd, version);
                return cmd.to_string();
            }
        }
    }

    log::warn!("[recommend-py] 未找到 Python 解释器，使用默认 'python'");
    "python".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(id: &str, title: &str, launch_count: u32) -> RecommendItem {
        RecommendItem {
            id: id.to_string(),
            title: title.to_string(),
            subtitle: String::new(),
            category: "apps".to_string(),
            launch_count,
            last_launched: None,
            tags: Vec::new(),
        }
    }

    #[tokio::test]
    async fn test_service_basic() {
        let config = RecommendConfig::default();
        let service = RecommendService::new(config);
        assert_eq!(service.config().enabled, true);
        assert_eq!(service.has_py_engine().await, false);
        assert_eq!(service.item_count().await, 0);
    }

    #[tokio::test]
    async fn test_set_items() {
        let config = RecommendConfig::default();
        let service = RecommendService::new(config);

        let items = vec![make_item("1", "A", 10), make_item("2", "B", 5)];
        service.set_items(items).await;
        assert_eq!(service.item_count().await, 2);
    }

    #[tokio::test]
    async fn test_disabled() {
        let mut config = RecommendConfig::default();
        config.enabled = false;
        let service = RecommendService::new(config);

        let items = vec![make_item("1", "Test", 10)];
        service.set_items(items).await;

        let context = RecommendContext::default();
        let scores = service.get_scores(&context).await;
        assert!(scores.is_empty());
    }

    #[tokio::test]
    async fn test_get_scores_sorted() {
        let config = RecommendConfig::default();
        let service = RecommendService::new(config);

        let items = vec![
            make_item("1", "Low", 1),
            make_item("2", "High", 100),
            make_item("3", "Medium", 50),
        ];
        service.set_items(items).await;

        let context = RecommendContext::default();
        let scores = service.get_scores(&context).await;

        assert_eq!(scores.len(), 3);
        assert!(scores[0].1 >= scores[1].1);
        assert!(scores[1].1 >= scores[2].1);
    }

    #[tokio::test]
    async fn test_max_results() {
        let mut config = RecommendConfig::default();
        config.max_results = 2;
        let service = RecommendService::new(config);

        let items = vec![
            make_item("1", "A", 100),
            make_item("2", "B", 50),
            make_item("3", "C", 10),
        ];
        service.set_items(items).await;

        let context = RecommendContext::default();
        let scores = service.get_scores(&context).await;
        assert_eq!(scores.len(), 2);
    }

    #[tokio::test]
    async fn test_empty_items() {
        let config = RecommendConfig::default();
        let service = RecommendService::new(config);

        let context = RecommendContext::default();
        let scores = service.get_scores(&context).await;
        assert!(scores.is_empty());
    }

    #[tokio::test]
    async fn test_record_launch() {
        let config = RecommendConfig::default();
        let service = RecommendService::new(config);

        let items = vec![make_item("1", "Test", 0)];
        service.set_items(items).await;

        service.record_launch("1").await;
        service.record_launch("1").await;

        let context = RecommendContext::default();
        let scores = service.get_scores(&context).await;
        assert_eq!(scores[0].0, "1");
        assert!(scores[0].1 > 0.0);
    }

    #[tokio::test]
    async fn test_status() {
        let config = RecommendConfig::default();
        let service = RecommendService::new(config);

        let status = service.status().await;
        assert_eq!(status.enabled, true);
        assert_eq!(status.py_engine_available, false);
        assert_eq!(status.total_recommendations, 0);
        assert_eq!(status.total_feedbacks, 0);
    }

    #[tokio::test]
    async fn test_report_feedback() {
        let config = RecommendConfig::default();
        let service = RecommendService::new(config);

        let items = vec![make_item("1", "Test", 0)];
        service.set_items(items).await;

        let feedback = FeedbackEvent {
            item_id: "1".to_string(),
            feedback_type: FeedbackType::Click,
            position: Some(0),
            context: RecommendContext::default(),
            timestamp: 12345,
        };

        service.report_feedback(feedback).await;

        let status = service.status().await;
        assert_eq!(status.total_feedbacks, 1);
    }
}
