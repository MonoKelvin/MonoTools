//! 服务注册表 - 管理注册到 Python 侧的服务

use std::collections::HashMap;
use std::sync::Mutex;

use super::types::ServiceHandle;

/// 服务注册表
///
/// 用于记录所有可用的 Python 服务，业务模块通过注册自己的服务信息
/// 来声明 Python 侧有对应服务可用。
pub struct ServiceRegistry {
    services: Mutex<HashMap<String, ServiceHandle>>,
}

impl ServiceRegistry {
    /// 创建新的注册表
    pub fn new() -> Self {
        Self {
            services: Mutex::new(HashMap::new()),
        }
    }

    /// 注册一个服务
    pub fn register(&self, service: ServiceHandle) {
        let mut services = self
            .services
            .lock()
            .expect("service registry lock poisoned");
        let name = service.name.clone();
        log::info!("[pybridge] 服务已注册: {}", name);
        services.insert(name, service);
    }

    /// 注销一个服务
    pub fn unregister(&self, name: &str) {
        let mut services = self
            .services
            .lock()
            .expect("service registry lock poisoned");
        services.remove(name);
        log::info!("[pybridge] 服务已注销: {}", name);
    }

    /// 检查服务是否已注册
    pub fn has_service(&self, name: &str) -> bool {
        let services = self
            .services
            .lock()
            .expect("service registry lock poisoned");
        services.contains_key(name)
    }

    /// 获取服务信息
    pub fn get_service(&self, name: &str) -> Option<ServiceHandle> {
        let services = self
            .services
            .lock()
            .expect("service registry lock poisoned");
        services.get(name).cloned()
    }

    /// 列出所有已注册的服务
    pub fn list_services(&self) -> Vec<ServiceHandle> {
        let services = self
            .services
            .lock()
            .expect("service registry lock poisoned");
        services.values().cloned().collect()
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get() {
        let registry = ServiceRegistry::new();
        assert!(!registry.has_service("test"));

        registry.register(ServiceHandle {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test service".to_string(),
        });

        assert!(registry.has_service("test"));
        let svc = registry.get_service("test").unwrap();
        assert_eq!(svc.name, "test");
        assert_eq!(svc.version, "1.0.0");
    }

    #[test]
    fn test_unregister() {
        let registry = ServiceRegistry::new();
        registry.register(ServiceHandle {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: "".to_string(),
        });
        assert!(registry.has_service("test"));

        registry.unregister("test");
        assert!(!registry.has_service("test"));
    }

    #[test]
    fn test_list_services() {
        let registry = ServiceRegistry::new();
        registry.register(ServiceHandle {
            name: "a".to_string(),
            version: "1.0".to_string(),
            description: "".to_string(),
        });
        registry.register(ServiceHandle {
            name: "b".to_string(),
            version: "2.0".to_string(),
            description: "".to_string(),
        });

        let list = registry.list_services();
        assert_eq!(list.len(), 2);
    }
}
