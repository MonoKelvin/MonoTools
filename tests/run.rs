use command_registry_test::run_command_registry_tests;
use icon_extraction_frontend_test::run_frontend_api_test;
use icon_extraction_test::run_icon_extraction_tests;
use search_engine_test::run_search_engine_tests;

#[tokio::test]
async fn run_all_tests() {
    run_search_engine_tests().await;
    run_command_registry_tests().await;
    run_icon_extraction_tests().await;
    run_frontend_api_test().await;
}

#[path = "common/mod.rs"]
mod common;

#[path = "rust/features/search_engine/test.rs"]
mod search_engine_test;

#[path = "rust/features/command_registry/test.rs"]
mod command_registry_test;

#[path = "rust/features/icon_extraction/test.rs"]
mod icon_extraction_test;

#[path = "rust/features/icon_extraction/test_frontend_api.rs"]
mod icon_extraction_frontend_test;

// Phase 0 performance baseline suite. Run with:
//   cargo test --test monotools_it -- bench_search --nocapture
#[path = "rust/benchmarks/search_bench.rs"]
mod bench_search;
