pub fn is_absolute(path: &str) -> bool {
    std::path::Path::new(path).is_absolute()
}

pub fn normalize(path: &str) -> std::path::PathBuf {
    let path = path.replace('\\', "/");
    std::path::PathBuf::from(path)
}

pub fn file_name(path: &str) -> Option<&str> {
    std::path::Path::new(path).file_name()?.to_str()
}

pub fn extension(path: &str) -> Option<&str> {
    std::path::Path::new(path).extension()?.to_str()
}
