#[path = "rust/features/search_engine/test.rs"]
mod search_engine_test;

#[path = "rust/features/usn_journal/test.rs"]
mod usn_journal_test;

#[tokio::test]
async fn run_all_tests() {
    println!("========== 运行所有测试模块 ==========");
    
    search_engine_test::run_search_engine_tests().await;
    usn_journal_test::run_usn_journal_tests().await;
    
    println!("========== 所有测试模块已完成 ==========");
}
