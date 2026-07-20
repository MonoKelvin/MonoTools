//! Recommend - 智能推荐模块
//!
//! 提供基于上下文的应用推荐能力。
//! 设计为完全独立：删除本目录或禁用 feature 后不影响其他功能。
//!
//! # 架构
//!
//! ```text
//!                   ┌───────────────────────┐
//!                   │   RecommendEngine     │
//!                   │      (trait)          │
//!                   └───────────┬───────────┘
//!                               │
//!              ┌────────────────┴────────────────┐
//!              │                                 │
//!    ┌─────────▼─────────┐            ┌──────────▼─────────┐
//!    │  RuleEngine       │            │  PyRecommendEngine │
//!    │  (保底，纯 Rust)    │            │  (Python 增强)     │
//!    └───────────────────┘            └────────────────────┘
//!              │                                 │
//!              └────────────────┬────────────────┘
//!                               │
//!                    ┌──────────▼──────────┐
//!                    │  RecommendService   │
//!                    │  (对外统一入口)      │
//!                    └─────────────────────┘
//! ```
//!
//! # Feature Flag
//!
//! - `recommend` - 启用推荐模块基础功能 (规则引擎)
//! - `recommend-py` - 启用 Python 增强推荐 (依赖 pybridge)
//!
//! # 用法
//!
//! ```rust,ignore
//! use recommend::RecommendService;
//!
//! let service = RecommendService::new(config);
//! let scores = service.get_recommend_scores(&items, &context).await;
//! ```

pub mod types;
pub mod engine;
pub mod rule_engine;

#[cfg(feature = "recommend-py")]
pub mod py_engine;

pub mod ipc;

pub use types::{
    RecommendConfig, RecommendContext, RecommendItem, RecommendResult, FeedbackEvent,
    FeedbackType, RecommendStatus,
};
pub use engine::RecommendEngine;
pub use rule_engine::RuleEngine;

use std::sync::Arc;
use tokio::sync::RwLock;

/// 推荐服务 - 对外统一入口
///
/// 根据配置和可用的引擎，自动选择最佳推荐策略。
/// Python 引擎不可用时自动降级到规则引擎。
pub struct RecommendService {
    config: RecommendConfig,
    rule_engine: Arc<RuleEngine>,

    #[cfg(feature = "recommend-py")]
    py_engine: Option<Arc<py_engine::PyRecommendEngine>>,

    status: Arc<RwLock<RecommendStatus>>,
}

impl RecommendService {
    /// 创建推荐服务
    pub fn new(config: RecommendConfig) -> Self {
        let rule_engine = Arc::new(RuleEngine::new(config.rule_weights.clone()));

        Self {
            config,
            rule_engine,
            #[cfg(feature = "recommend-py")]
            py_engine: None,
            status: Arc::new(RwLock::new(RecommendStatus::default())),
        }
    }

    /// 获取配置
    pub fn config(&self) -> &RecommendConfig {
        &self.config
    }

    /// 获取当前状态
    pub async fn status(&self) -> RecommendStatus {
        self.status.read().await.clone()
    }

    /// 设置 Python 推荐引擎（需要 pybridge feature）
    #[cfg(feature = "recommend-py")]
    pub fn set_py_engine(&mut self, engine: py_engine::PyRecommendEngine) {
        self.py_engine = Some(Arc::new(engine));
    }

    /// 检查 Python 引擎是否可用
    pub fn has_py_engine(&self) -> bool {
        #[cfg(feature = "recommend-py")]
        {
            self.py_engine.is_some()
        }
        #[cfg(not(feature = "recommend-py"))]
        {
            false
        }
    }

    /// 获取推荐分数
    ///
    /// 优先使用 Python 引擎，失败或不可用时降级到规则引擎。
    /// 返回按分数降序排列的结果，数量不超过 max_results。
    pub async fn get_scores(
        &self,
        items: &[RecommendItem],
        context: &RecommendContext,
    ) -> Vec<(String, f32)> {
        if !self.config.enabled {
            return Vec::new();
        }

        // 尝试 Python 引擎
        #[cfg(feature = "recommend-py")]
        {
            if let Some(py_engine) = &self.py_engine {
                match py_engine.get_scores(items, context).await {
                    Ok(scores) => {
                        let result = self.post_process(scores);
                        self.update_stats(result.len() as u64).await;
                        return result;
                    }
                    Err(e) => {
                        log::warn!("[recommend] Python 推荐失败，降级到规则引擎: {}", e);
                    }
                }
            }
        }

        // 降级到规则引擎
        let scores = match self.rule_engine.get_scores(items, context).await {
            Ok(scores) => scores,
            Err(e) => {
                log::error!("[recommend] 规则引擎也失败了: {}", e);
                Vec::new()
            }
        };

        let result = self.post_process(scores);
        self.update_stats(result.len() as u64).await;
        result
    }

    /// 后处理：排序 + 截断
    fn post_process(&self, mut scores: Vec<(String, f32)>) -> Vec<(String, f32)> {
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(self.config.max_results);
        scores
    }

    /// 更新统计数据
    async fn update_stats(&self, result_count: u64) {
        let mut status = self.status.write().await;
        status.total_recommendations += 1;
        let _ = result_count;
    }

    /// 上报用户反馈
    pub async fn report_feedback(&self, feedback: FeedbackEvent) {
        // 更新反馈统计
        {
            let mut status = self.status.write().await;
            status.total_feedbacks += 1;
        }

        // 规则引擎也可以从反馈中学习（简单统计）
        self.rule_engine.record_feedback(&feedback);

        // Python 引擎的在线学习
        #[cfg(feature = "recommend-py")]
        {
            if let Some(py_engine) = &self.py_engine {
                if let Err(e) = py_engine.report_feedback(&feedback).await {
                    log::warn!("[recommend] 上报反馈到 Python 失败: {}", e);
                }
            }
        }
    }
}

/// 初始化推荐模块 - 独立模块注册入口
///
/// 在 Tauri 的 setup 阶段调用，负责：
/// 1. 创建 RecommendService 实例
/// 2. 通过 app.manage() 注册服务状态
///
/// # 独立性
/// 删除本模块后，只需移除调用此函数的代码即可。
pub fn init<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    use tauri::Manager;
    let config = RecommendConfig::default();
    let service = Arc::new(RecommendService::new(config));
    app.manage(service);
    log::info!("[recommend] 模块初始化完成");
}

/// 注册推荐服务到 PyBridge（如果可用）
///
/// 独立模块的注册入口，被外部调用后才会接入系统。
/// 删除本模块后，调用方只需移除这一行即可。
#[cfg(feature = "recommend-py")]
pub fn register_service(
    _bridge: &crate::pybridge::PyBridge,
) -> Result<(), crate::pybridge::types::PyBridgeError> {
    use crate::pybridge::types::ServiceHandle;

    _bridge.registry().register(ServiceHandle {
        name: "recommend".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "智能推荐服务".to_string(),
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(id: &str, title: &str, tags: &[&str], launch_count: u32) -> RecommendItem {
        RecommendItem {
            id: id.to_string(),
            title: title.to_string(),
            subtitle: String::new(),
            category: "apps".to_string(),
            launch_count,
            last_launched: None,
            tags: tags.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[tokio::test]
    async fn test_service_basic() {
        let config = RecommendConfig::default();
        let service = RecommendService::new(config);

        assert_eq!(service.config().enabled, true);
        assert_eq!(service.has_py_engine(), false);
    }

    #[tokio::test]
    async fn test_service_disabled() {
        let mut config = RecommendConfig::default();
        config.enabled = false;
        let service = RecommendService::new(config);

        let items = vec![make_item("1", "Test", &["dev"], 10)];
        let context = RecommendContext::default();
        let scores = service.get_scores(&items, &context).await;

        assert!(scores.is_empty());
    }

    #[tokio::test]
    async fn test_service_get_scores_sorted() {
        let config = RecommendConfig::default();
        let service = RecommendService::new(config);

        let items = vec![
            make_item("1", "Low Usage", &["dev"], 1),
            make_item("2", "High Usage", &["dev"], 100),
            make_item("3", "Medium Usage", &["dev"], 50),
        ];
        let context = RecommendContext::default();
        let scores = service.get_scores(&items, &context).await;

        assert_eq!(scores.len(), 3);
        // 验证分数是降序排列的
        assert!(scores[0].1 >= scores[1].1);
        assert!(scores[1].1 >= scores[2].1);
    }

    #[tokio::test]
    async fn test_service_max_results() {
        let mut config = RecommendConfig::default();
        config.max_results = 2;
        let service = RecommendService::new(config);

        let items = vec![
            make_item("1", "A", &["dev"], 100),
            make_item("2", "B", &["dev"], 50),
            make_item("3", "C", &["dev"], 10),
        ];
        let context = RecommendContext::default();
        let scores = service.get_scores(&items, &context).await;

        assert_eq!(scores.len(), 2);
    }

    #[tokio::test]
    async fn test_service_empty_items() {
        let config = RecommendConfig::default();
        let service = RecommendService::new(config);

        let items: Vec<RecommendItem> = vec![];
        let context = RecommendContext::default();
        let scores = service.get_scores(&items, &context).await;

        assert!(scores.is_empty());
    }

    #[tokio::test]
    async fn test_service_report_feedback() {
        let config = RecommendConfig::default();
        let service = RecommendService::new(config);

        let feedback = FeedbackEvent {
            item_id: "app1".to_string(),
            feedback_type: FeedbackType::Click,
            position: Some(0),
            context: RecommendContext::default(),
            timestamp: 12345,
        };

        service.report_feedback(feedback).await;

        let status = service.status().await;
        assert_eq!(status.total_feedbacks, 1);
    }

    #[tokio::test]
    async fn test_service_status() {
        let config = RecommendConfig::default();
        let service = RecommendService::new(config);

        let status = service.status().await;
        assert_eq!(status.total_recommendations, 0);
        assert_eq!(status.total_feedbacks, 0);
        assert_eq!(status.py_engine_available, false);
    }

    #[tokio::test]
    async fn test_service_recommendation_stats() {
        let config = RecommendConfig::default();
        let service = RecommendService::new(config);

        let items = vec![make_item("1", "Test", &["dev"], 10)];
        let context = RecommendContext::default();

        let _ = service.get_scores(&items, &context).await;
        let _ = service.get_scores(&items, &context).await;

        let status = service.status().await;
        assert_eq!(status.total_recommendations, 2);
    }

    #[tokio::test]
    async fn test_service_foreground_category() {
        let config = RecommendConfig::default();
        let service = RecommendService::new(config);

        let items = vec![
            make_item("1", "VS Code", &["dev"], 10),
            make_item("2", "Chrome", &["browser"], 10),
        ];
        let mut context = RecommendContext::default();
        context.foreground_category = Some("dev".to_string());

        let scores = service.get_scores(&items, &context).await;
        // 第一个应该是 dev 类别的
        assert_eq!(scores[0].0, "1");
    }

    #[test]
    fn test_service_config_access() {
        let config = RecommendConfig::default();
        let service = RecommendService::new(config.clone());
        assert_eq!(service.config().max_results, config.max_results);
    }
}
