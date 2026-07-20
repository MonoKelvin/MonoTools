//! 规则引擎 - 纯 Rust 实现的保底推荐引擎
//!
//! 基于规则的推荐算法，不依赖任何外部服务。
//! Python 引擎不可用时自动降级到这里。

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;

use super::engine::RecommendEngine;
use super::types::{FeedbackEvent, FeedbackType, RecommendContext, RecommendItem, RuleWeights};

/// 规则引擎
pub struct RuleEngine {
    weights: RuleWeights,
    stats: Mutex<RuleEngineStats>,
}

#[derive(Default)]
struct RuleEngineStats {
    click_counts: HashMap<String, u64>,
}

impl RuleEngine {
    /// 创建新的规则引擎
    pub fn new(weights: RuleWeights) -> Self {
        Self {
            weights,
            stats: Mutex::new(RuleEngineStats::default()),
        }
    }

    /// 计算单个项目的规则分数
    fn score_item(&self, item: &RecommendItem, context: &RecommendContext) -> (f32, Vec<String>) {
        let mut score = 0.0;
        let mut reasons = Vec::new();

        // 1. 启动次数 (对数缩放)
        let count_factor = ((item.launch_count as f32) + 1.0).ln();
        score += count_factor * self.weights.launch_count;
        if item.launch_count > 0 {
            reasons.push(format!("启动 {} 次", item.launch_count));
        }

        // 2. 名称相似度 (与查询词或前台应用标题)
        let query_str = if !context.query.is_empty() {
            context.query.to_lowercase()
        } else {
            context.foreground_app_title.to_lowercase()
        };
        if !query_str.is_empty() {
            let title_lower = item.title.to_lowercase();
            if title_lower.contains(&query_str) {
                score += self.weights.name_similarity;
                reasons.push("名称匹配".to_string());
            }
        }

        // 3. 前台应用类别匹配
        if let Some(fg_cat) = &context.foreground_category {
            if item.tags.iter().any(|t| t == fg_cat) {
                score += self.weights.foreground_category;
                reasons.push("同类应用".to_string());
            } else {
                // 弱相关类别也轻微加分
                let related = related_categories(fg_cat);
                if item.tags.iter().any(|t| related.contains(&t.as_str())) {
                    score += self.weights.foreground_category * 0.3;
                    reasons.push("相关类别".to_string());
                }
            }
        }

        // 4. 时间衰减 (最近使用的加分)
        if let Some(last_launched) = item.last_launched {
            let now = chrono::Utc::now().timestamp();
            let hours_since = ((now - last_launched) as f32) / 3600.0;
            let decay = (1.0 - self.weights.time_decay_per_hour).powf(hours_since);
            score *= 0.5 + 0.5 * decay.max(0.0);
        }

        (score, reasons)
    }

    /// 记录反馈 (更新统计)
    pub fn record_feedback(&self, feedback: &FeedbackEvent) {
        match feedback.feedback_type {
            FeedbackType::Click | FeedbackType::Pin => {
                let mut stats = self.stats.lock().unwrap_or_else(|e| e.into_inner());
                *stats
                    .click_counts
                    .entry(feedback.item_id.clone())
                    .or_insert(0) += 1;
            }
            _ => {}
        }
    }
}

#[async_trait]
impl RecommendEngine for RuleEngine {
    fn name(&self) -> &str {
        "rule_engine"
    }

    async fn get_scores(
        &self,
        items: &[RecommendItem],
        context: &RecommendContext,
    ) -> anyhow::Result<Vec<(String, f32)>> {
        let mut results = Vec::with_capacity(items.len());

        for item in items {
            let (score, _reasons) = self.score_item(item, context);
            results.push((item.id.clone(), score));
        }

        Ok(results)
    }

    async fn report_feedback(&self, feedback: &FeedbackEvent) -> anyhow::Result<()> {
        self.record_feedback(feedback);
        Ok(())
    }
}

/// 相关类别映射 (简化版，与前端 APP_CATEGORIES/RECOMMENDATION_MAP 对应)
fn related_categories(category: &str) -> Vec<&'static str> {
    match category {
        "dev" => vec!["terminal", "vcs", "browser", "filemanager"],
        "terminal" => vec!["dev", "vcs", "filemanager"],
        "vcs" => vec!["dev", "terminal"],
        "communication" => vec!["browser", "image", "office"],
        "browser" => vec!["communication", "download", "office"],
        "media" => vec!["image", "download"],
        "image" => vec!["media", "office"],
        "office" => vec!["filemanager", "browser", "communication"],
        "filemanager" => vec!["archive", "download", "office"],
        "download" => vec!["archive", "filemanager"],
        "archive" => vec!["filemanager", "download"],
        _ => vec![],
    }
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
    async fn test_rule_engine_basic() {
        let engine = RuleEngine::new(RuleWeights::default());
        let items = vec![
            make_item("1", "VS Code", &["dev"], 100),
            make_item("2", "Chrome", &["browser"], 50),
        ];
        let context = RecommendContext::default();

        let scores = engine.get_scores(&items, &context).await.unwrap();
        assert_eq!(scores.len(), 2);
        // 启动次数多的分数高
        assert!(scores[0].1 > scores[1].1);
    }

    #[tokio::test]
    async fn test_rule_engine_foreground_category() {
        let engine = RuleEngine::new(RuleWeights::default());
        let items = vec![
            make_item("1", "VS Code", &["dev"], 10),
            make_item("2", "Chrome", &["browser"], 10),
        ];
        let mut context = RecommendContext::default();
        context.foreground_category = Some("dev".to_string());

        let scores = engine.get_scores(&items, &context).await.unwrap();
        // 同类别的应该分数更高
        let score_dev = scores.iter().find(|(id, _)| id == "1").unwrap().1;
        let score_browser = scores.iter().find(|(id, _)| id == "2").unwrap().1;
        assert!(score_dev > score_browser);
    }

    #[test]
    fn test_related_categories() {
        let related = related_categories("dev");
        assert!(related.contains(&"terminal"));
        assert!(related.contains(&"browser"));
        assert!(!related.contains(&"media"));

        let related = related_categories("unknown");
        assert!(related.is_empty());
    }

    #[tokio::test]
    async fn test_name_similarity() {
        let engine = RuleEngine::new(RuleWeights::default());
        let items = vec![
            make_item("1", "Visual Studio Code", &["dev"], 10),
            make_item("2", "Google Chrome", &["browser"], 10),
        ];
        let mut context = RecommendContext::default();
        context.query = "code".to_string();

        let scores = engine.get_scores(&items, &context).await.unwrap();
        let score_code = scores.iter().find(|(id, _)| id == "1").unwrap().1;
        let score_chrome = scores.iter().find(|(id, _)| id == "2").unwrap().1;
        assert!(score_code > score_chrome);
    }

    #[tokio::test]
    async fn test_time_decay() {
        let engine = RuleEngine::new(RuleWeights::default());
        let now = chrono::Utc::now().timestamp();

        let mut item_recent = make_item("1", "Recent App", &["dev"], 10);
        item_recent.last_launched = Some(now - 3600); // 1 小时前

        let mut item_old = make_item("2", "Old App", &["dev"], 10);
        item_old.last_launched = Some(now - 7 * 24 * 3600); // 7 天前

        let items = vec![item_recent, item_old];
        let context = RecommendContext::default();

        let scores = engine.get_scores(&items, &context).await.unwrap();
        let score_recent = scores.iter().find(|(id, _)| id == "1").unwrap().1;
        let score_old = scores.iter().find(|(id, _)| id == "2").unwrap().1;
        assert!(score_recent > score_old);
    }

    #[test]
    fn test_record_feedback_click() {
        let engine = RuleEngine::new(RuleWeights::default());

        let feedback = FeedbackEvent {
            item_id: "app1".to_string(),
            feedback_type: FeedbackType::Click,
            position: None,
            context: RecommendContext::default(),
            timestamp: 0,
        };

        engine.record_feedback(&feedback);
        engine.record_feedback(&feedback);

        let stats = engine.stats.lock().unwrap();
        assert_eq!(*stats.click_counts.get("app1").unwrap(), 2);
    }

    #[test]
    fn test_record_feedback_pin() {
        let engine = RuleEngine::new(RuleWeights::default());

        let feedback = FeedbackEvent {
            item_id: "app1".to_string(),
            feedback_type: FeedbackType::Pin,
            position: None,
            context: RecommendContext::default(),
            timestamp: 0,
        };

        engine.record_feedback(&feedback);

        let stats = engine.stats.lock().unwrap();
        assert_eq!(*stats.click_counts.get("app1").unwrap(), 1);
    }

    #[test]
    fn test_record_feedback_dislike() {
        let engine = RuleEngine::new(RuleWeights::default());

        let feedback = FeedbackEvent {
            item_id: "app1".to_string(),
            feedback_type: FeedbackType::Dislike,
            position: None,
            context: RecommendContext::default(),
            timestamp: 0,
        };

        engine.record_feedback(&feedback);

        let stats = engine.stats.lock().unwrap();
        assert!(stats.click_counts.get("app1").is_none());
    }

    #[tokio::test]
    async fn test_empty_items() {
        let engine = RuleEngine::new(RuleWeights::default());
        let items: Vec<RecommendItem> = vec![];
        let context = RecommendContext::default();

        let scores = engine.get_scores(&items, &context).await.unwrap();
        assert!(scores.is_empty());
    }

    #[tokio::test]
    async fn test_related_category_bonus() {
        let engine = RuleEngine::new(RuleWeights::default());
        let items = vec![
            make_item("1", "Terminal", &["terminal"], 10),
            make_item("2", "Spotify", &["media"], 10),
        ];
        let mut context = RecommendContext::default();
        context.foreground_category = Some("dev".to_string());

        let scores = engine.get_scores(&items, &context).await.unwrap();
        // terminal 是 dev 的相关类别，应该比 media 分数高
        let score_terminal = scores.iter().find(|(id, _)| id == "1").unwrap().1;
        let score_media = scores.iter().find(|(id, _)| id == "2").unwrap().1;
        assert!(score_terminal > score_media);
    }

    #[tokio::test]
    async fn test_report_feedback() {
        let engine = RuleEngine::new(RuleWeights::default());

        let feedback = FeedbackEvent {
            item_id: "app1".to_string(),
            feedback_type: FeedbackType::Click,
            position: Some(0),
            context: RecommendContext::default(),
            timestamp: 12345,
        };

        let result = engine.report_feedback(&feedback).await;
        assert!(result.is_ok());

        let stats = engine.stats.lock().unwrap();
        assert_eq!(*stats.click_counts.get("app1").unwrap(), 1);
    }
}
