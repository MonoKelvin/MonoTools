use std::path::PathBuf;

fn base_dir() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("tests").join("rust")
}

pub fn config_dir(module: &str) -> PathBuf {
    base_dir().join("config").join(module)
}

pub fn data_dir(module: &str) -> PathBuf {
    base_dir().join("data").join(module)
}

pub fn output_dir(module: &str) -> PathBuf {
    base_dir().join("output").join(module)
}

pub fn config_path(module: &str, filename: &str) -> PathBuf {
    config_dir(module).join(filename)
}

pub fn data_path(module: &str, filename: &str) -> PathBuf {
    data_dir(module).join(filename)
}

pub fn output_path(module: &str, filename: &str) -> PathBuf {
    output_dir(module).join(filename)
}

pub fn ensure_dir(path: &PathBuf) {
    if !path.exists() {
        let _ = std::fs::create_dir_all(path);
    }
}