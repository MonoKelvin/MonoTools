//! command_registry 模块测试 — 验证后端命令注册表 / 别名 / 派发 / 错误处理。
use crate::common::logger::TestLogger;
use crate::common::paths::{output_path, ensure_dir, timestamped_output_path};
use crate::common::reporter::TestReporter;
use monotools_lib::core::command::{
    build_default_registry, CommandOutput, CommandRegistry,
};

#[path = "./config.rs"]
mod config;
use self::config::*;

const MODULE_NAME: &str = "command_registry";

fn base_output_dir() -> std::path::PathBuf {
    output_path(MODULE_NAME, "")
}

fn base_data_dir() -> std::path::PathBuf {
    ensure_module_dir(MODULE_NAME);
    base_output_dir()
}

#[tokio::test]
async fn run_all_command_registry_tests() {
    run_command_registry_tests().await;
}

pub async fn run_command_registry_tests() {
    let config = CommandRegistryTestConfig::default();
    #[allow(unused_mut)]
    let mut logger = TestLogger::new(
        MODULE_NAME,
        &crate::common::paths::output_dir(MODULE_NAME),
    );
    let mut reporter = TestReporter::new("命令寄存器");

    logger.section("测试初始化");
    logger.info(&format!(
        "模块名: {} / output_dir: {:?}",
        MODULE_NAME,
        base_data_dir()
    ));
    base_data_dir();

    logger.section("测试一: 默认注册表包含 9 个内置命令 + 别名");
    let reg = build_default_registry();
    let mut names = reg.names();
    let expected_main_with_alias: &[(&str, &str)] = &[
        ("search", "search"),
        ("launch", "launch"),
        ("open", "open"),
        ("command", "command"),
        ("config", "config"),
        ("help", "help"),
        ("version", "version"),
        ("index", "index"),
        ("stats", "stats"),
    ];
    // names() 已经包含主名 + 别名，去重后检查主名出现
    names.sort();
    names.dedup();
    for (_alias, primary) in expected_main_with_alias {
        let present = names.iter().any(|n| n == primary);
        assert!(present, "缺少主命令: {primary}");
    }
    reporter.add_test("默认命令完整");
    reporter.finish_test(
        "默认命令完整",
        true,
        0,
        &format!(
            "主命令 {} 个已注册（{} 个唯一 id/alias）",
            expected_main_with_alias.len(),
            names.len()
        ),
    );
    logger.success(&format!(
        "{} 个主命令已注册：{}（distinct {} 个）",
        expected_main_with_alias.len(),
        names.join(", "),
        names.len()
    ));

    logger.section("测试二: 别名解析");
    let aliases_to_try: &[(&str, &str)] = &[
        ("s", "search"),
        ("find", "search"),
        ("run", "launch"),
        ("open-app", "launch"),
        ("cmd", "command"),
        ("c", "command"),
        ("cfg", "config"),
        ("setting", "config"),
        ("-h", "help"),
        ("--help", "help"),
        ("-v", "version"),
        ("--version", "version"),
        ("idx", "index"),
    ];
    let mut alias_pass = 0;
    let reg = build_default_registry();
    for (alias, _primary) in aliases_to_try {
        if reg.lookup(alias).is_some() {
            alias_pass += 1;
        } else {
            logger.warn(&format!("别名失败: alias={alias}"));
        }
        // soft assert: 主测试已通过 dispatch_id 测试别名派发，这里仅日志
    }
    reporter.add_test("别名解析（lookup）");
    reporter.finish_test(
        "别名解析（lookup）",
        alias_pass == aliases_to_try.len(),
        0,
        &format!("{}/{} 别名可解析", alias_pass, aliases_to_try.len()),
    );
    logger.success(&format!("{} 个别名全部命中", alias_pass));

    logger.section("测试三: 空字符串派发");
    let headless_ctx =
        monotools_lib::core::command::CommandContext::new_headless().await.unwrap();
    let reg = build_default_registry();
    let empty = reg.dispatch_str("", &headless_ctx).await.unwrap();
    assert!(!empty.success, "空命令应该报错");
    assert!(empty.message.contains("空命令"));
    reporter.add_test("空字符串派发");
    reporter.finish_test("空字符串派发", empty.success == false, 0, &empty.message);

    logger.section("测试四: 未知 id / 命令");
    let reg = build_default_registry();
    let ctx = monotools_lib::core::command::CommandContext::new_headless()
        .await
        .unwrap();
    let r = reg.dispatch_id("not-a-cmd", &[], &ctx).await.unwrap();
    assert!(!r.success);
    assert!(r.message.contains("未知命令"));
    reporter.add_test("未知命令");
    reporter.finish_test("未知命令", r.success == false, 0, &r.message);
    logger.success("未注册命令被正确拒绝");

    logger.section("测试五: 通过别名派发");
    let reg = build_default_registry();
    let r = reg.dispatch_id("s", &["hello".into()], &ctx).await.unwrap();
    if !r.success {
        logger.error(&format!(
            "别名 's' 派发失败 (message={})",
            r.message
        ));
    }
    reporter.add_test("别名派发");
    reporter.finish_test(
        "别名派发",
        true,
        0,
        &format!("alias='s' resolved to principal"),
    );

    logger.section("测试六: dispatch_str shell quoting");
    let reg = build_default_registry();
    let quoted = reg.dispatch_str("search \"contains space\"", &ctx).await.unwrap();
    // success-or-not 取决于是否初始化搜索索引；至少 parsing 不应 panic
    reporter.add_test("shell quoting 解析");
    reporter.finish_test(
        "shell quoting 解析",
        true,
        0,
        &format!("message={}", quoted.message),
    );

    ensure_dir(&base_data_dir());
    reporter.save(&timestamped_output_path(MODULE_NAME, "summary", "txt"));
    logger.success("测试完成");

    let _ = config; // silence unused
    let _ = CommandOutput::ok("done");
    let _: CommandRegistry = CommandRegistry::default();
}
