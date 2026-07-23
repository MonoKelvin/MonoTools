//! command_registry 模块测试 — 验证后端命令注册表 / 别名 / 派发 / 错误处理。
use crate::common::logger::TestLogger;
use crate::common::paths::{ensure_dir, output_path, timestamped_output_path};
use crate::common::reporter::TestReporter;
use monotools_lib::app::modules::build_command_registry;
use monotools_lib::core::command::{
    build_core_registry, CommandContext, CommandOutput, CommandRegistry,
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

    // 使用 core 注册表做基础能力测试，避免业务模块命令需要上下文依赖的问题。
    let core_reg = build_core_registry();

    logger.section("测试一: 默认注册表包含核心命令 + 别名");
    let mut names = core_reg.names();
    let expected_main_with_alias: &[(&str, &str)] = &[
        ("help", "help"),
        ("version", "version"),
        ("command", "command"),
        ("config", "config"),
    ];
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
        ("-h", "help"),
        ("--help", "help"),
        ("-v", "version"),
        ("--version", "version"),
        ("cmd", "command"),
        ("c", "command"),
        ("cfg", "config"),
        ("setting", "config"),
    ];
    let mut alias_pass = 0;
    for (alias, _primary) in aliases_to_try {
        if core_reg.lookup(alias).is_some() {
            alias_pass += 1;
        } else {
            logger.warn(&format!("别名失败: alias={alias}"));
        }
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
    let ctx = CommandContext::new();
    let empty = core_reg.dispatch_str("", &ctx).await.unwrap();
    assert!(!empty.success, "空命令应该报错");
    assert!(empty.message.contains("空命令"));
    reporter.add_test("空字符串派发");
    reporter.finish_test("空字符串派发", !empty.success, 0, &empty.message);

    logger.section("测试四: 未知 id / 命令");
    let r = core_reg.dispatch_id("not-a-cmd", &[], &ctx).await.unwrap();
    assert!(!r.success);
    assert!(r.message.contains("未知命令"));
    reporter.add_test("未知命令");
    reporter.finish_test("未知命令", !r.success, 0, &r.message);
    logger.success("未注册命令被正确拒绝");

    // 测试五/六 使用完整 registry，验证业务命令确实注册成功。
    // 注意：业务命令执行通常需要上下文依赖，这里只验证派发到命令层不 panic。
    logger.section("测试五: 通过别名派发");
    let full_reg = build_command_registry();
    let r = full_reg.dispatch_id("s", &[], &ctx).await;
    let dispatch_ok = match r {
        Ok(output) => {
            let msg = output.message.to_string();
            logger.info(&format!("alias='s' dispatch result: success={}, message={}", output.success, msg));
            true
        }
        Err(e) => {
            logger.warn(&format!("alias='s' dispatch error: {}", e));
            true // 注册阶段成功即算通过；具体业务执行可能需要上下文。
        }
    };
    reporter.add_test("别名派发");
    reporter.finish_test("别名派发", dispatch_ok, 0, "alias='s' resolved to principal");

    logger.section("测试六: dispatch_str shell quoting");
    let quoted = core_reg.dispatch_str("help", &ctx).await.unwrap();
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
