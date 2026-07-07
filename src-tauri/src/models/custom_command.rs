use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomCommand {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub keyword: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub icon: Option<String>,
    pub category: String,
    pub enabled: bool,
    pub run_as_admin: bool,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
}
