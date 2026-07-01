use crate::models::plugin::PluginManifest;

/// 插件沙箱 - 权限检查
pub struct PluginSandbox;

impl PluginSandbox {
    /// 检查插件是否有权限执行操作
    pub fn check_permission(manifest: &PluginManifest, required_permission: &str) -> Result<()> {
        // 核心权限只有内置插件可以使用
        if required_permission.starts_with("core:") && !manifest.is_builtin {
            anyhow::bail!("Plugin '{}' does not have permission '{}' (core permissions are for builtin plugins only)", manifest.id, required_permission);
        }

        // 检查插件是否声明了该权限
        if !manifest.permissions.contains(&required_permission.to_string()) {
            anyhow::bail!("Plugin '{}' does not declare permission '{}'", manifest.id, required_permission);
        }

        Ok(())
    }

    /// 检查插件是否有任意一个所需权限
    pub fn check_any_permission(manifest: &PluginManifest, required: &[&str]) -> Result<()> {
        for perm in required {
            if manifest.permissions.contains(&perm.to_string()) {
                return Ok(());
            }
        }

        anyhow::bail!("Plugin '{}' does not have any of the required permissions: {:?}", manifest.id, required);
    }

    /// 验证插件权限集合
    pub fn validate_permissions(manifest: &PluginManifest) -> Result<()> {
        for perm in &manifest.permissions {
            // 检查权限格式
            if !Self::is_valid_permission_format(perm) {
                anyhow::bail!("Invalid permission format: {}", perm);
            }

            // 核心权限检查
            if perm.starts_with("core:") && !manifest.is_builtin {
                anyhow::bail!("Plugin '{}' cannot request core permission '{}'", manifest.id, perm);
            }
        }

        Ok(())
    }

    /// 检查权限格式是否合法
    fn is_valid_permission_format(permission: &str) -> bool {
        // 格式: namespace:action (例如: fs:read, clipboard:write)
        let parts: Vec<&str> = permission.split(':').collect();
        if parts.len() != 2 {
            return false;
        }

        let namespace = parts[0];
        let action = parts[1];

        // 检查命名空间
        let valid_namespaces = [
            "core", "fs", "ui", "system", "clipboard", "network", "storage", "plugin"
        ];

        if !valid_namespaces.contains(&namespace) {
            return false;
        }

        // action 不能为空
        !action.is_empty()
    }
}

impl Default for PluginSandbox {
    fn default() -> Self {
        Self
    }
}
