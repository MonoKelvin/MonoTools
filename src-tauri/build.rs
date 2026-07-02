fn main() {
    // 空的 build script，用于设置 OUT_DIR 环境变量
    println!("cargo:rerun-if-changed=src/main.rs");
}
