use std::path::PathBuf;

/// 仓库根目录：`<repo>/`（所有测试输出和共享库都集中在这里）
fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR 指向 src-tauri/。仓库根目录 = parent of that.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(manifest_dir))
}

/// 测试根目录：`<repo>/tests/`
pub fn tests_root() -> PathBuf {
    repo_root().join("tests")
}

/// 数据目录：`<repo>/tests/data/`
fn data_root() -> PathBuf {
    tests_root().join("data")
}

/// 输出目录：`<repo>/tests/output/`
fn output_root() -> PathBuf {
    tests_root().join("output")
}

pub fn data_dir(module: &str) -> PathBuf {
    data_root().join(module)
}

pub fn output_dir(module: &str) -> PathBuf {
    output_root().join(module)
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

/// 为模块生成时间戳文件路径：`<repo>/tests/output/<module>/<base>_<timestamp>.<ext>`
pub fn timestamped_output_path(module: &str, base: &str, ext: &str) -> PathBuf {
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    output_path(module, &format!("{}_{}.{}", base, ts, ext))
}
