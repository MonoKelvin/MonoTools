use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use serde_json::Value;
use crate::models::plugin::PluginManifest;

pub struct PluginLoader {
    plugins_dir: std::path::PathBuf,
}

impl PluginLoader {
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self { plugins_dir }
    }

    /// 扫描插件目录
    pub fn scan_plugins(&self) -> Result<Vec<PluginManifest>> {
        let mut manifests = Vec::new();

        if !self.plugins_dir.exists() {
            std::fs::create_dir_all(&self.plugins_dir)?;
            return Ok(manifests);
        }

        for entry in std::fs::read_dir(&self.plugins_dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            // 检查是否有 plugin.json
            let manifest_path = path.join("plugin.json");
            if !manifest_path.exists() {
                continue;
            }

            // 解析 plugin.json
            match self.parse_manifest(&manifest_path) {
                Ok(manifest) => {
                    manifests.push(manifest);
                }
                Err(e) => {
                    tracing::warn!("Failed to parse plugin manifest at {:?}: {}", manifest_path, e);
                }
            }
        }

        Ok(manifests)
    }

    /// 解析 plugin.json
    pub fn parse_manifest(&self, path: &Path) -> Result<PluginManifest> {
        let content = std::fs::read_to_string(path)
            .context(format!("Failed to read plugin manifest: {:?}", path))?;

        let mut manifest: PluginManifest = serde_json::from_str(&content)
            .context(format!("Failed to parse plugin manifest JSON: {:?}", path))?;

        // 标记为非内置插件
        manifest.is_builtin = false;

        // 验证必填字段
        if manifest.id.is_empty() {
            anyhow::bail!("Plugin ID is required");
        }

        if manifest.name.is_empty() {
            anyhow::bail!("Plugin name is required");
        }

        if manifest.version.is_empty() {
            anyhow::bail!("Plugin version is required");
        }

        if manifest.plugin_type.is_empty() {
            anyhow::bail!("Plugin type is required");
        }

        // 验证插件类型
        let valid_types = ["theme", "provider", "command", "view", "integration", "hybrid"];
        if !valid_types.contains(&manifest.plugin_type.as_str()) {
            anyhow::bail!("Invalid plugin type: {}", manifest.plugin_type);
        }

        Ok(manifest)
    }

    /// 加载单个插件
    pub fn load_plugin(&self, manifest: &PluginManifest) -> Result<LoadedPlugin> {
        let plugin_dir = self.plugins_dir.join(&manifest.id);

        if !plugin_dir.exists() {
            anyhow::bail!("Plugin directory not found: {:?}", plugin_dir);
        }

        // 加载前端代码
        let frontend = if let Some(frontend_path) = &manifest.entry.frontend {
            let full_path = plugin_dir.join(frontend_path);
            if full_path.exists() {
                Some(self.load_frontend(&full_path)?)
            } else {
                None
            }
        } else {
            None
        };

        // 加载后端代码（WASM 或 sidecar）
        let backend = if let Some(backend_path) = &manifest.entry.backend {
            let full_path = plugin_dir.join(backend_path);
            if full_path.exists() {
                Some(self.load_backend(&full_path)?)
            } else {
                None
            }
        } else {
            None
        };

        Ok(LoadedPlugin {
            manifest: manifest.clone(),
            frontend,
            backend,
        })
    }

    /// 加载前端代码
    fn load_frontend(&self, path: &Path) -> Result<FrontendCode> {
        let content = std::fs::read_to_string(path)?;
        Ok(FrontendCode {
            path: path.to_path_buf(),
            content,
        })
    }

    /// 加载后端代码
    fn load_backend(&self, path: &Path) -> Result<BackendCode> {
        if path.extension().and_then(|s| s.to_str()) == Some("wasm") {
            let bytes = std::fs::read(path)?;
            Ok(BackendCode::Wasm { bytes })
        } else if path.extension().and_then(|s| s.to_str()) == Some("exe") {
            Ok(BackendCode::Sidecar { path: path.to_path_buf() })
        } else {
            anyhow::bail!("Unsupported backend type: {:?}", path);
        }
    }
}

/// 加载的插件
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub frontend: Option<FrontendCode>,
    pub backend: Option<BackendCode>,
}

/// 前端代码
pub struct FrontendCode {
    pub path: std::path::PathBuf,
    pub content: String,
}

/// 后端代码
pub enum BackendCode {
    Wasm { bytes: Vec<u8> },
    Sidecar { path: std::path::PathBuf },
}

impl Default for PluginLoader {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .expect("Failed to get data directory");
        Self::new(data_dir.join("MonoTools").join("plugins"))
    }
}
