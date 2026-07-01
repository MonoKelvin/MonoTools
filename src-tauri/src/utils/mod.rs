use std::path::Path;

/// 将 Windows 路径转换为 Unix 风格路径
pub fn normalize_path(path: &str) -> String {
    path.replace('\\', '/')
}

/// 判断是否为绝对路径
pub fn is_absolute_path(path: &str) -> bool {
    Path::new(path).is_absolute()
}

/// 计算字符串的 ASCII 和
pub fn calc_ascii_sum(s: &str) -> i32 {
    s.bytes().map(|b| b as i32).sum()
}

/// 检查字符串是否为 ASCII
pub fn is_ascii(s: &str) -> bool {
    s.is_ascii()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("C:\\Users\\test"), "C:/Users/test");
        assert_eq!(normalize_path("D:/Work/test"), "D:/Work/test");
    }

    #[test]
    fn test_calc_ascii_sum() {
        assert_eq!(calc_ascii_sum("abc"), 294);
    }
}
