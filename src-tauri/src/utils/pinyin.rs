use once_cell::sync::Lazy;
use std::collections::HashMap;

// 简化的拼音映射表（常用汉字）
// 在生产环境中，应该使用完整的拼音词典库
static PINYIN_MAP: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    // 常用字示例
    m.insert("安", "an");
    m.insert("百", "bai");
    m.insert("从", "cong");
    m.insert("的", "de");
    m.insert("额", "e");
    m.insert("发", "fa");
    m.insert("个", "ge");
    m.insert("好", "hao");
    m.insert("和", "he");
    m.insert("会", "hui");
    m.insert("机", "ji");
    m.insert("开", "kai");
    m.insert("了", "le");
    m.insert("吗", "ma");
    m.insert("你", "ni");
    m.insert("平", "ping");
    m.insert("器", "qi");
    m.insert("人", "ren");
    m.insert("三", "san");
    m.insert("通", "tong");
    m.insert("文", "wen");
    m.insert("下", "xia");
    m.insert("一", "yi");
    m.insert("在", "zai");
    m.insert("中", "zhong");
    m
});

/// 将中文文本转换为拼音（首字母）
pub fn to_pinyin_initials(text: &str) -> String {
    text.chars()
        .map(|c| {
            PINYIN_MAP
                .get(c.encode_utf16(&mut [0; 2]).as_ref())
                .and_then(|py| py.chars().next())
                .unwrap_or(c)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_pinyin_initials() {
        assert_eq!(to_pinyin_initials("你好"), "ni");
        assert_eq!(to_pinyin_initials("中国"), "zg");
    }
}
