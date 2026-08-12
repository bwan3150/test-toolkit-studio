// 【黑盒 CLI 契约测试】spawn 真二进制,测 clap 装配层与输出协议——
// src 内的单测/无设备集成测试(#[cfg(test)] 就地放)覆盖不到这一层:
// 参数解析(--copilot 裸旗标曾要求带值,真机才暴露)、两件套缺包报错、JSON error 契约。
// 不依赖设备/AI:所有场景都在走到设备操作前就该返回。
// 跑法:cargo test --no-default-features --test cli(ADR-0008)

use std::path::PathBuf;
use std::process::{Command, Output};

fn tke() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tke"))
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("tke-cli-test-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).to_string()
}
fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

/// --help 可用且是 tke 的帮助(自定义 help 文案)
#[test]
fn help_works() {
    let o = tke().arg("--help").output().unwrap();
    assert!(o.status.success(), "--help 应退出 0:{}", stderr(&o));
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(s.contains("tke"), "帮助应含 tke:{}", s);
    assert!(s.contains("copilot"), "帮助应含 --copilot(全局参数表):{}", s);
}

/// run 不存在的文件 → 非 0 退出 + 明确报错
#[test]
fn run_missing_file_fails_clearly() {
    let o = tke().args(["run", "/nonexistent/x.tks"]).output().unwrap();
    assert!(!o.status.success(), "不存在的脚本应失败");
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(s.contains("不存在"), "应说清文件不存在:{}", s);
}

/// 两件套契约:有 .tks 无 .tklib → 报缺元素包(不静默降级,INV-7)
#[test]
fn run_without_tklib_reports_missing_pack() {
    let d = tmp("no-tklib");
    let tks = d.join("case.tks");
    std::fs::write(&tks, "步骤:\n点击 [{某按钮}]\n").unwrap();
    let o = tke().args(["run", tks.to_str().unwrap()]).output().unwrap();
    assert!(!o.status.success(), "缺元素包应失败");
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(s.contains("元素包") || s.contains("tklib"), "应报缺元素包:{}", s);
}

/// --copilot 裸旗标可用(回归:曾要求必须带值,真机踩过)
#[test]
fn copilot_bare_flag_accepted() {
    let d = tmp("copilot-bare");
    let tks = d.join("case.tks");
    std::fs::write(&tks, "步骤:\n点击 [{某按钮}]\n").unwrap();
    let o = tke().args(["run", tks.to_str().unwrap(), "--copilot"]).output().unwrap();
    let s = format!("{}{}", stdout(&o), stderr(&o));
    // 裸旗标不该报参数错;应走到业务层报缺元素包
    assert!(!s.contains("a value is required"), "--copilot 裸旗标不该要求带值:{}", s);
    assert!(s.contains("元素包") || s.contains("tklib"), "应走到缺包报错:{}", s);
}

/// --copilot 非法值 → clap 拒绝
#[test]
fn copilot_invalid_value_rejected() {
    let o = tke().args(["run", "x.tks", "--copilot", "maybe"]).output().unwrap();
    assert!(!o.status.success());
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(
        s.contains("possible values") || s.contains("invalid"),
        "非法值应被 clap 拒绝:{}",
        s
    );
}

/// --json 模式的错误输出必须是合法 JSON 且带 success/error 字段(App 消费契约)
#[test]
fn json_error_output_is_valid_json() {
    let o = tke().args(["run", "/nonexistent/x.tks", "--json"]).output().unwrap();
    assert!(!o.status.success());
    let out = stdout(&o);
    let line = out.lines().find(|l| l.trim_start().starts_with('{')).unwrap_or_else(|| {
        panic!("--json 模式 stdout 应有 JSON 行:{}", out)
    });
    let v: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|e| {
        panic!("stdout JSON 行应合法({}):{}", e, line)
    });
    assert_eq!(v["success"], false, "错误契约 success=false:{}", line);
    assert!(v["error"].is_string(), "错误契约应带 error 字符串:{}", line);
}

/// 不认识的文件类型 → 明确报错(不是 panic)
#[test]
fn run_unknown_extension_fails_clearly() {
    let d = tmp("bad-ext");
    let f = d.join("case.txt");
    std::fs::write(&f, "hi").unwrap();
    let o = tke().args(["run", f.to_str().unwrap()]).output().unwrap();
    assert!(!o.status.success());
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(s.contains("无法识别") || s.contains(".tks"), "应说清支持的类型:{}", s);
}
