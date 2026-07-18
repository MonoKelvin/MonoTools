//! 应用搜索引擎模块
//!
//! 从开始菜单、桌面、注册表扫描应用程序。

pub mod engine;
pub mod trie;

pub use engine::AppSearchEngine;
pub use trie::Trie;
