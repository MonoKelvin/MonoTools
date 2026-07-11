use search_engine_test::run_search_engine_tests;
use command_registry_test::run_command_registry_tests;

#[tokio::test]
async fn run_all_tests() {
    run_search_engine_tests().await;
    run_command_registry_tests().await;
}

#[path = "common/mod.rs"]
mod common;

#[path = "rust/features/search_engine/test.rs"]
mod search_engine_test;

#[path = "rust/features/command_registry/test.rs"]
mod command_registry_test;
