use search_engine_test::run_search_engine_tests;

#[tokio::test]
async fn run_all_tests() {
    run_search_engine_tests().await;
}

#[path = "rust/common/mod.rs"]
mod common;

#[path = "rust/features/search_engine/test.rs"]
mod search_engine_test;
