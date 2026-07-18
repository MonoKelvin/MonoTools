//! Trie（前缀树）索引
//!
//! 为 AppSearchEngine 提供 O(k) 前缀匹配能力，k 为查询串长度。
//! 每个节点维护一个 `indices` 列表，记录所有经过该节点的字符串索引，
//! 这样 `search(prefix)` 只需走到 prefix 末端节点，直接返回其 indices。
//!
//! 索引策略：对每个 AppEntry 插入三条记录：
//!   1. 名称小写（例: "chrome"）
//!   2. 拼音首字母小写（例: "wj" → "微信"）
//!   3. 拼音全拼小写（例: "weixin" → "微信"）
//! 搜索时合并三路结果并去重。

use std::collections::HashMap;

/// Trie 树的节点。
#[derive(Debug, Default, Clone)]
pub struct TrieNode {
    children: HashMap<char, TrieNode>,
    /// 所有经过此节点的字符串索引（指向 entries 向量中的位置）。
    indices: Vec<usize>,
}

impl TrieNode {
    fn new() -> Self {
        Self {
            children: HashMap::new(),
            indices: Vec::new(),
        }
    }
}

/// 前缀树。线程安全由外部 `RwLock` 保证，本结构体本身不内部加锁。
#[derive(Debug, Default)]
pub struct Trie {
    root: TrieNode,
}

impl Trie {
    /// 创建空 Trie。
    pub fn new() -> Self {
        Self::default()
    }

    /// 将 `text` 插入 Trie，并关联索引 `index`。
    ///
    /// `text` 应为已转为小写的字符串（调用方负责 to_lowercase）。
    /// 沿途每个节点都会追加 `index`，因此前缀查询可直接拿到所有命中。
    pub fn insert(&mut self, text: &str, index: usize) {
        let mut node = &mut self.root;
        node.indices.push(index);
        for ch in text.chars() {
            node = node.children.entry(ch).or_insert_with(TrieNode::new);
            node.indices.push(index);
        }
    }

    /// 查询所有以 `prefix` 开头的字符串索引。
    ///
    /// 返回的索引可能包含重复（同一 entry 的多条拼音记录都命中时），
    /// 调用方应自行去重。
    pub fn search(&self, prefix: &str) -> &[usize] {
        let mut node = &self.root;
        for ch in prefix.chars() {
            match node.children.get(&ch) {
                Some(child) => node = child,
                None => return &[],
            }
        }
        &node.indices
    }

    /// 返回 Trie 中索引的总插入次数（非去重条目数，仅用于调试/日志）。
    pub fn len(&self) -> usize {
        fn count_nodes(node: &TrieNode) -> usize {
            let mut total = node.indices.len();
            for child in node.children.values() {
                total += count_nodes(child);
            }
            total
        }
        count_nodes(&self.root)
    }

    /// 是否为空（未插入任何字符串）。
    pub fn is_empty(&self) -> bool {
        self.root.indices.is_empty()
    }

    /// 清空 Trie。
    pub fn clear(&mut self) {
        self.root = TrieNode::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_search_single() {
        let mut trie = Trie::new();
        trie.insert("chrome", 0);
        assert_eq!(trie.search("chr"), &[0]);
        assert_eq!(trie.search("chrome"), &[0]);
        assert_eq!(trie.search("chromex"), &Vec::<usize>::new());
    }

    #[test]
    fn insert_and_search_multiple() {
        let mut trie = Trie::new();
        trie.insert("chrome", 0);
        trie.insert("chromium", 1);
        trie.insert("firefox", 2);

        let r = trie.search("chr");
        assert!(r.contains(&0));
        assert!(r.contains(&1));
        assert!(!r.contains(&2));

        assert_eq!(trie.search("fire"), &[2]);
        assert_eq!(trie.search("xyz"), &Vec::<usize>::new());
    }

    #[test]
    fn search_returns_all_matching_indices() {
        let mut trie = Trie::new();
        // 同一条文本插入多次（模拟同名不同路径的应用）
        trie.insert("wechat", 0);
        trie.insert("wechat", 1);
        let r = trie.search("we");
        assert!(r.contains(&0));
        assert!(r.contains(&1));
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn empty_prefix_returns_root_indices() {
        let mut trie = Trie::new();
        trie.insert("abc", 0);
        trie.insert("def", 1);
        // 空前缀应返回所有索引（因为 root.indices 包含全部）
        let r = trie.search("");
        assert!(r.contains(&0));
        assert!(r.contains(&1));
    }

    #[test]
    fn unicode_and_chinese_chars() {
        let mut trie = Trie::new();
        trie.insert("微信", 0);
        trie.insert("微博", 1);
        trie.insert("weibo", 2);

        assert_eq!(trie.search("微"), &[0, 1]);
        assert_eq!(trie.search("微信"), &[0]);
        assert_eq!(trie.search("weibo"), &[2]);
        assert_eq!(trie.search("微x"), &Vec::<usize>::new());
    }

    #[test]
    fn clear_removes_all() {
        let mut trie = Trie::new();
        trie.insert("abc", 0);
        trie.clear();
        assert_eq!(trie.search("a"), &Vec::<usize>::new());
        assert!(trie.is_empty());
    }

    #[test]
    fn case_sensitive_by_design() {
        // Trie 本身不做大小写转换，调用方负责 to_lowercase。
        // 这里验证 Trie 是大小写敏感的（符合设计预期）。
        let mut trie = Trie::new();
        trie.insert("Chrome", 0);
        assert_eq!(trie.search("chrome"), &Vec::<usize>::new());
        assert_eq!(trie.search("Chrome"), &[0]);
    }
}
