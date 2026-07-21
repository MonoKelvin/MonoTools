//! 推荐模块类型定义

use serde::{Deserialize, Serialize};

/// 推荐配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendConfig {
    /// 是否启用推荐功能
    pub enabled: bool,

    /// 最多返回多少个推荐结果
    pub max_results: usize,
}

impl Default for RecommendConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_results: 20,
        }
    }
}

/// 推荐项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendItem {
    /// 项目唯一 ID
    pub id: String,
    /// 标题/名称
    pub title: String,
    /// 副标题/路径
    pub subtitle: String,
    /// 类别 (apps, files, commands 等)
    pub category: String,
    /// 启动次数
    pub launch_count: u32,
    /// 上次启动时间戳 (秒)
    pub last_launched: Option<i64>,
    /// 标签/类别关键字
    pub tags: Vec<String>,
}

/// 推荐上下文
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecommendContext {
    /// 当前前台应用路径
    pub foreground_app_path: String,
    /// 当前前台应用标题
    pub foreground_app_title: String,
    /// 查询词 (如果有的话)
    pub query: String,
}

/// 用户反馈事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEvent {
    /// 项目 ID
    pub item_id: String,
    /// 反馈类型
    pub feedback_type: FeedbackType,
    /// 推荐时的位置
    pub position: Option<usize>,
    /// 反馈时的上下文
    pub context: RecommendContext,
    /// 时间戳 (秒)
    pub timestamp: i64,
}

/// 反馈类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackType {
    /// 用户点击/启动了 (正反馈)
    Click,
    /// 用户忽略了 (负反馈/未点击)
    Ignore,
    /// 用户显式不感兴趣
    Dislike,
    /// 用户固定了 (强正反馈)
    Pin,
}

/// 推荐状态
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecommendStatus {
    /// 是否启用
    pub enabled: bool,
    /// Python 引擎是否可用
    pub py_engine_available: bool,
    /// 累计推荐次数
    pub total_recommendations: u64,
    /// 累计反馈次数
    pub total_feedbacks: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = RecommendConfig::default();
        assert_eq!(config.enabled, true);
        assert_eq!(config.max_results, 20);
    }

    #[test]
    fn test_recommend_item() {
        let item = RecommendItem {
            id: "1".to_string(),
            title: "Test App".to_string(),
            subtitle: "test.exe".to_string(),
            category: "apps".to_string(),
            launch_count: 10,
            last_launched: Some(1234567890),
            tags: vec!["dev".to_string()],
        };
        assert_eq!(item.id, "1");
        assert_eq!(item.tags.len(), 1);
    }

    #[test]
    fn test_context_default() {
        let ctx = RecommendContext::default();
        assert_eq!(ctx.query, "");
        assert!(ctx.foreground_app_path.is_empty());
    }

    #[test]
    fn test_feedback_type_serialization() {
        let json_click = serde_json::to_string(&FeedbackType::Click).unwrap();
        assert_eq!(json_click, "\"click\"");

        let json_pin = serde_json::to_string(&FeedbackType::Pin).unwrap();
        assert_eq!(json_pin, "\"pin\"");

        let parsed: FeedbackType = serde_json::from_str("\"dislike\"").unwrap();
        assert_eq!(parsed, FeedbackType::Dislike);
    }

    #[test]
    fn test_recommend_status_default() {
        let status = RecommendStatus::default();
        assert_eq!(status.enabled, false);
        assert_eq!(status.py_engine_available, false);
        assert_eq!(status.total_recommendations, 0);
        assert_eq!(status.total_feedbacks, 0);
    }

    #[test]
    fn test_feedback_event() {
        let event = FeedbackEvent {
            item_id: "app1".to_string(),
            feedback_type: FeedbackType::Click,
            position: Some(0),
            context: RecommendContext::default(),
            timestamp: 12345,
        };
        assert_eq!(event.item_id, "app1");
        assert_eq!(event.feedback_type, FeedbackType::Click);
        assert_eq!(event.position, Some(0));
    }
}
