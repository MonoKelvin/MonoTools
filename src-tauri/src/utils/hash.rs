use crate::utils::path::fast_hash;

/// 使用 fast_hash 对字符串均匀分桶
pub fn bucket(s: &str, n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    (fast_hash(s) as usize) % n
}

/// 生成短 ID（前 8 位 hash + 时间戳后缀）
pub fn short_id(s: &str) -> String {
    let h = fast_hash(s);
    format!("{:x}", h & 0xffffffff)
}
