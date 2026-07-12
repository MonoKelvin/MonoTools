//! command_registry 模块测试配置
use crate::common::paths::{data_path, output_path, ensure_dir};

pub struct CommandRegistryTestConfig {
    #[allow(dead_code)]
    pub timeout_ms: u64,
    #[allow(dead_code)]
    pub list_specs_valid: bool,
    #[allow(dead_code)]
    pub dispatch_id: String,
    #[allow(dead_code)]
    pub dispatch_args: Vec<String>,
}

impl Default for CommandRegistryTestConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 1000,
            list_specs_valid: true,
            dispatch_id: "search".into(),
            dispatch_args: vec!["hello".into()],
        }
    }
}

impl CommandRegistryTestConfig {
    #[allow(dead_code)]
    pub fn from_file(_path: &str) -> Self {
        Self::default()
    }
}

pub fn spec_module_dir(module: &str) -> std::path::PathBuf {
    output_path(module, "")
}

pub fn ensure_module_dir(module: &str) {
    ensure_dir(&spec_module_dir(module));
}

#[allow(dead_code)]
pub fn sample_log_dir(module: &str) -> std::path::PathBuf {
    output_path(module, "logs")
}

#[allow(dead_code)]
pub fn sample_data_dir(module: &str) -> std::path::PathBuf {
    data_path(module, "logs")
}
