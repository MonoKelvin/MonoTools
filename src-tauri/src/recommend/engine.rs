//! 推荐引擎 trait

use super::types::{FeedbackEvent, RecommendContext, RecommendItem};

use async_trait::async_trait;

/// 推荐引擎 trait
///
/// 所有推荐引擎都实现这个 trait，便于统一调度和降级。
#[async_trait]
pub trait RecommendEngine: Send + Sync {
    /// 引擎名称
    fn name(&self) -> &str;

    /// 计算推荐分数
    ///
    /// 返回 (item_id, score) 列表，不需要排序，调用方会排序。
    async fn get_scores(
        &self,
        items: &[RecommendItem],
        context: &RecommendContext,
    ) -> anyhow::Result<Vec<(String, f32)>>;

    /// 上报用户反馈 (可选实现，用于在线学习)
    async fn report_feedback(&self, _feedback: &FeedbackEvent) -> anyhow::Result<()> {
        Ok(())
    }
}
