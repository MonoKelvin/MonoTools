use std::path::{Path, PathBuf};
use fuzzy_matcher::FuzzyMatcher;

/// 拼接路径段
pub fn join(base: &Path, segments: &[&str]) -> PathBuf {
    let mut p = base.to_path_buf();
    for s in segments {
        p.push(s);
    }
    p
}

/// 判断是否可执行文件
pub fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    path.extension()
        .map(|e| {
            let e = e.to_string_lossy().to_lowercase();
            e == "exe" || e == "bat" || e == "cmd" || e == "lnk" || e == "url"
        })
        .unwrap_or(false)
}

/// 模糊匹配：是否 query 是 name 的子串（不区分大小写）
pub fn fuzzy_match(query: &str, name: &str) -> bool {
    let q = query.to_lowercase();
    let n = name.to_lowercase();
    fuzzy_matcher::skim::SkimMatcherV2::default()
        .fuzzy_match(&n, &q)
        .is_some()
}

/// 计算字符串 hash（FNV-1a）
pub fn fast_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}
