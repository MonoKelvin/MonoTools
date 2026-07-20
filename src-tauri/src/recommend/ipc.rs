//! 推荐模块 IPC 命令
//!
//! 独立模块设计：所有 IPC 命令和状态自包含，不依赖 AppState。
//! 通过 tauri::Builder.manage() 注册服务，通过 State 访问。

use crate::recommend::{FeedbackEvent, RecommendContext, RecommendItem, RecommendService};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

/// 获取推荐分数
///
/// 前端调用示例：
/// ```javascript
/// const scores = await invoke('recommend_get_scores', { items, context });
/// ```
#[tauri::command]
pub async fn recommend_get_scores(
    service: State<'_, Arc<RecommendService>>,
    items: Vec<RecommendItem>,
    context: RecommendContext,
) -> Result<Vec<(String, f32)>, String> {
    let scores = service.get_scores(&items, &context).await;
    Ok(scores)
}

/// 上报用户反馈
#[tauri::command]
pub async fn recommend_report_feedback(
    service: State<'_, Arc<RecommendService>>,
    feedback: FeedbackEvent,
) -> Result<(), String> {
    service.report_feedback(feedback).await;
    Ok(())
}

/// 获取推荐状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendStatusResponse {
    pub enabled: bool,
    pub py_available: bool,
    pub engine: String,
}

#[tauri::command]
pub async fn recommend_get_status(
    service: State<'_, Arc<RecommendService>>,
) -> Result<RecommendStatusResponse, String> {
    let status = service.status().await;
    Ok(RecommendStatusResponse {
        enabled: status.enabled,
        py_available: status.py_engine_available,
        engine: if service.has_py_engine() {
            "python".to_string()
        } else {
            "rule".to_string()
        },
    })
}
