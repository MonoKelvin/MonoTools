//! 拼音工具 —— 将中文字符串转换为拼音首字母和完整拼音。
//!
//! 使用 `pinyin` crate (v0.11, default-features = false), 内置拼音字典,
//! 无需外部数据文件, 编译体积增加 ~200KB。

/// 单个字符串的拼音信息。
#[derive(Debug, Clone, Default)]
pub struct PinyinInfo {
    /// 拼音首字母 (小写, 无分隔). 例: "微信" → "wx".
    pub initials: Option<String>,
    /// 完整拼音 (小写, 无空格无声调). 例: "微信" → "weixin".
    pub full: Option<String>,
}

/// 将字符串转换为拼音信息。
///
/// 策略:
/// - 只处理中文字符 (CJK Unified Ideographs, U+4E00..=U+9FFF);
///   ASCII / 数字 / 其他字符原样跳过 (不参与 initials / full 构建).
/// - 若字符串中**不含任何中文字符**, 返回 `PinyinInfo { initials: None, full: None }`,
///   调用方无需存储, 节省缓存空间。
/// - 多音字取常用读音 (`pinyin::to_pinyin` 默认行为)。
pub fn to_pinyin(s: &str) -> PinyinInfo {
    use pinyin::ToPinyin;

    let mut has_chinese = false;
    let mut initials = String::new();
    let mut full = String::new();

    for ch in s.chars() {
        // 只处理 CJK 统一汉字
        if !('\u{4e00}'..='\u{9fff}').contains(&ch) {
            continue;
        }
        has_chinese = true;
        if let Some(py) = ch.to_pinyin() {
            let plain = py.plain(); // 无声调, 例: "wei"
            initials.push(plain.chars().next().unwrap_or(ch));
            full.push_str(plain);
        }
    }

    if !has_chinese {
        return PinyinInfo::default();
    }

    PinyinInfo {
        initials: Some(initials.to_lowercase()),
        full: Some(full.to_lowercase()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinyin_weixin() {
        let info = to_pinyin("微信");
        assert_eq!(info.initials, Some("wx".to_string()));
        assert_eq!(info.full, Some("weixin".to_string()));
    }

    #[test]
    fn pinyin_ascii_returns_none() {
        let info = to_pinyin("Chrome");
        assert!(info.initials.is_none());
        assert!(info.full.is_none());
    }

    #[test]
    fn pinyin_mixed() {
        let info = to_pinyin("微信Chrome");
        assert_eq!(info.initials, Some("wx".to_string()));
        assert_eq!(info.full, Some("weixin".to_string()));
    }

    #[test]
    fn pinyin_single_char() {
        let info = to_pinyin("中");
        assert_eq!(info.initials, Some("z".to_string())); // zhong → z
        assert_eq!(info.full, Some("zhong".to_string()));
    }
}
