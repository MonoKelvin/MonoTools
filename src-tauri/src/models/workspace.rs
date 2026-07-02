use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSnapshot {
    pub id: String,
    pub exe_path: String,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub window_title: String,
    pub window_rect: WindowRect,
    pub window_state: WindowState,
    pub launch_order: u32,
    pub launch_delay_ms: u64,
    pub require_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_restored_at: Option<DateTime<Utc>>,
    pub apps: Vec<AppSnapshot>,
    pub auto_start: bool,
}
