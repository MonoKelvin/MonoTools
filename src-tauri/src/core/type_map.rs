//! 类型安全的动态类型映射容器
//!
//! 基于 `TypeId` + `Any` 实现，允许在一个集合中存储任意类型的值，
//! 并通过类型安全的方式获取。适用于依赖注入、上下文传递等场景。
//!
//! # 示例
//!
//! ```ignore
//! let mut map = TypeMap::new();
//! map.insert(42i32);
//! map.insert("hello".to_string());
//!
//! assert_eq!(map.get::<i32>(), Some(&42));
//! assert_eq!(map.get::<String>(), Some(&"hello".to_string()));
//! ```

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// 类型安全的动态映射容器
#[derive(Debug, Default)]
pub struct TypeMap {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl TypeMap {
    /// 创建一个空的 TypeMap
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// 插入一个值，按类型存储。如果该类型已存在则覆盖。
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) {
        self.map.insert(TypeId::of::<T>(), Box::new(value));
    }

    /// 获取指定类型的引用
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref::<T>())
    }

    /// 获取指定类型的可变引用
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.map
            .get_mut(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_mut::<T>())
    }

    /// 检查是否包含指定类型
    pub fn contains<T: 'static>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<T>())
    }

    /// 移除指定类型的值
    pub fn remove<T: 'static>(&mut self) -> Option<T> {
        self.map
            .remove(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast::<T>().ok().map(|b| *b))
    }

    /// 返回存储的类型数量
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut map = TypeMap::new();
        map.insert(42i32);
        map.insert("hello".to_string());
        map.insert(3.14f64);

        assert_eq!(map.get::<i32>(), Some(&42));
        assert_eq!(map.get::<String>(), Some(&"hello".to_string()));
        assert_eq!(map.get::<f64>(), Some(&3.14));
        assert_eq!(map.get::<bool>(), None);
    }

    #[test]
    fn test_overwrite() {
        let mut map = TypeMap::new();
        map.insert(42i32);
        map.insert(100i32);
        assert_eq!(map.get::<i32>(), Some(&100));
    }

    #[test]
    fn test_get_mut() {
        let mut map = TypeMap::new();
        map.insert(42i32);
        if let Some(v) = map.get_mut::<i32>() {
            *v = 100;
        }
        assert_eq!(map.get::<i32>(), Some(&100));
    }

    #[test]
    fn test_contains() {
        let mut map = TypeMap::new();
        assert!(!map.contains::<i32>());
        map.insert(42i32);
        assert!(map.contains::<i32>());
    }

    #[test]
    fn test_remove() {
        let mut map = TypeMap::new();
        map.insert(42i32);
        assert_eq!(map.remove::<i32>(), Some(42));
        assert_eq!(map.get::<i32>(), None);
    }

    #[test]
    fn test_arc() {
        use std::sync::Arc;
        let mut map = TypeMap::new();
        map.insert(Arc::new(42i32));
        assert_eq!(**map.get::<Arc<i32>>().unwrap(), 42);
    }
}
