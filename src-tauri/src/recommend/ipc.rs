//! 推荐模块 IPC 命令
//!
//! 独立模块设计：所有 IPC 命令和状态自包含。
//! 通过 tauri::Builder.manage() 注册服务，通过 State 访问。

use crate::recommend::{FeedbackEvent, RecommendContext, RecommendItem, RecommendService};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

/// 设置推荐候选项目
#[tauri::command]
pub async fn recommend_set_items(
    service: State<'_, Arc<RecommendService>>,
    items: Vec<RecommendItem>,
) -> Result<(), String> {
    service.set_items(items).await;
    Ok(())
}

/// 记录一次应用启动
#[tauri::command]
pub async fn recommend_record_launch(
    service: State<'_, Arc<RecommendService>>,
    item_id: String,
) -> Result<(), String> {
    service.record_launch(&item_id).await;
    Ok(())
}

/// 获取推荐分数
#[tauri::command]
pub async fn recommend_get_scores(
    service: State<'_, Arc<RecommendService>>,
    context: RecommendContext,
) -> Result<Vec<(String, f32)>, String> {
    let scores = service.get_scores(&context).await;
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

/// 推荐状态响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendStatusResponse {
    pub enabled: bool,
    pub py_available: bool,
    pub item_count: usize,
    pub total_recommendations: u64,
    pub total_feedbacks: u64,
}

/// 获取推荐状态
#[tauri::command]
pub async fn recommend_get_status(
    service: State<'_, Arc<RecommendService>>,
) -> Result<RecommendStatusResponse, String> {
    let status = service.status().await;
    let item_count = service.item_count().await;
    let has_py = service.has_py_engine().await;

    Ok(RecommendStatusResponse {
        enabled: status.enabled,
        py_available: has_py,
        item_count,
        total_recommendations: status.total_recommendations,
        total_feedbacks: status.total_feedbacks,
    })
}

/// 获取窗口监控状态 (当前激活应用 + 最近应用历史).
#[tauri::command]
pub async fn get_window_monitor_state(
    service: State<'_, Arc<RecommendService>>,
) -> Result<serde_json::Value, String> {
    let snapshot = service.window_monitor_state().await;
    Ok(serde_json::json!({
        "activeAppPath": snapshot.active_app_path,
        "activeAppTitle": snapshot.active_app_title,
        "recentApps": snapshot.recent_apps.iter().map(|a| serde_json::json!({
            "path": &a.path,
            "title": &a.title,
        })).collect::<Vec<_>>(),
    }))
}
