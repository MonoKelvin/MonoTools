//! Python 推荐引擎 - 通过 pybridge 调用 Python 侧推荐服务
//!
//! 仅在 feature = "recommend-py" 时编译。

use async_trait::async_trait;

use crate::pybridge::PyBridge;

use super::engine::RecommendEngine;
use super::types::{FeedbackEvent, RecommendContext, RecommendItem};

/// Python 推荐引擎
pub struct PyRecommendEngine {
    bridge: std::sync::Arc<PyBridge>,
}

impl PyRecommendEngine {
    /// 创建新的 Python 推荐引擎
    pub fn new(bridge: std::sync::Arc<PyBridge>) -> Self {
        Self { bridge }
    }

    /// 初始化 Python 侧推荐服务 (传入候选应用)
    pub async fn initialize(&self, items: &[RecommendItem]) -> anyhow::Result<()> {
        let params = serde_json::json!({
            "items": items,
        });

        let result = self
            .bridge
            .call("recommend.initialize", params)
            .await
            .map_err(|e| anyhow::anyhow!("初始化推荐服务失败: {}", e))?;

        if result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(())
        } else {
            Err(anyhow::anyhow!("初始化失败: {:?}", result))
        }
    }
}

#[async_trait]
impl RecommendEngine for PyRecommendEngine {
    fn name(&self) -> &str {
        "py_recommend_engine"
    }

    async fn get_scores(
        &self,
        items: &[RecommendItem],
        context: &RecommendContext,
    ) -> anyhow::Result<Vec<(String, f32)>> {
        let params = serde_json::json!({
            "items": items,
            "context": context,
        });

        let result = self
            .bridge
            .call("recommend.get_scores", params)
            .await
            .map_err(|e| anyhow::anyhow!("获取推荐分数失败: {}", e))?;

        let scores = result
            .get("scores")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("响应格式错误: 缺少 scores 数组"))?;

        let mut result_vec = Vec::with_capacity(scores.len());
        for score_obj in scores {
            let id = score_obj
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let score = score_obj
                .get("score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;
            result_vec.push((id, score));
        }

        Ok(result_vec)
    }

    async fn report_feedback(&self, feedback: &FeedbackEvent) -> anyhow::Result<()> {
        let params = serde_json::json!({
            "feedback": feedback,
        });

        let _ = self
            .bridge
            .call("recommend.report_feedback", params)
            .await
            .map_err(|e| anyhow::anyhow!("上报反馈失败: {}", e))?;

        Ok(())
    }
}
