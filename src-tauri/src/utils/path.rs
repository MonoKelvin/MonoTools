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
///
/// Windows 平台上的可执行程序格式：
/// - .exe: 标准可执行文件
/// - .bat: 批处理脚本
/// - .cmd: 命令脚本
/// - .msi: Windows Installer 安装包
/// - .com: DOS 可执行文件
/// - .scr: 屏幕保护程序
/// - .lnk: 快捷方式（需要进一步检查目标）
///
/// 注意: 不包含 .url —— .url 是 Internet Shortcut (网址快捷方式),
/// 不是应用程序, 不应出现在应用搜索结果中.
pub fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    path.extension()
        .map(|e| {
            let e = e.to_string_lossy().to_lowercase();
            matches!(
                e.as_str(),
                "exe" | "bat" | "cmd" | "msi" | "com" | "scr" | "lnk"
            )
        })
        .unwrap_or(false)
}

/// 判断路径是否指向真正的可执行程序（用于验证快捷方式的目标）
///
/// 与 is_executable 不同的是，此函数不包含 .lnk，
/// 因为我们要检查的是快捷方式指向的最终目标是否是可执行程序。
pub fn is_true_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    path.extension()
        .map(|e| {
            let e = e.to_string_lossy().to_lowercase();
            matches!(
                e.as_str(),
                "exe" | "bat" | "cmd" | "msi" | "com" | "scr"
            )
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
