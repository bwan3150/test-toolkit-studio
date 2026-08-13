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
    // 带 -d：设备现在是 run 的硬前提，不给会先报缺设备、盖住这里要测的缺包契约
    let o = tke().args(["-d", "web", "run", tks.to_str().unwrap()]).output().unwrap();
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
    let o = tke().args(["-d", "web", "run", tks.to_str().unwrap(), "--copilot"]).output().unwrap();
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

/// --headless 出现在全局参数表里
#[test]
fn help_lists_headless() {
    let o = tke().arg("--help").output().unwrap();
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(s.contains("headless"), "帮助应含 --headless(全局参数表):{}", s);
}

/// --headless 裸旗标**不能吞掉后面的子命令**
/// (回归 --copilot 同类坑:num_args=0..=1 的可选值参数容易把子命令名当成值吃掉。
///  若被吃掉,这里会报"无法识别的 headless 值 run"而不是"文件不存在")
#[test]
fn headless_bare_flag_does_not_swallow_subcommand() {
    let o = tke()
        .args(["--headless", "run", "/nonexistent/x.tks"])
        .output()
        .unwrap();
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(!o.status.success(), "不存在的脚本应失败");
    assert!(
        s.contains("不存在"),
        "应走到 run 并报文件不存在(说明 run 没被当成 --headless 的值):{}",
        s
    );
}

/// --headless 带无法识别的值 → 明确报错,不静默兜底成 auto(INV-9)
#[test]
fn headless_invalid_value_fails_clearly() {
    let o = tke()
        .args(["--headless=bogus", "run", "/nonexistent/x.tks"])
        .output()
        .unwrap();
    assert!(!o.status.success(), "无法识别的 headless 值应失败");
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(s.contains("headless"), "报错应点名 headless:{}", s);
    assert!(s.contains("auto") && s.contains("off"), "报错应给出可用值:{}", s);
}

/// --headless=off 可被接受(强制有头;此处只验参数装配,不真起浏览器)
#[test]
fn headless_off_is_accepted() {
    let o = tke()
        .args(["--headless=off", "run", "/nonexistent/x.tks"])
        .output()
        .unwrap();
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(s.contains("不存在"), "应走到 run(参数被接受):{}", s);
}

/// web 的 `control close` 可省略包名(= 销毁浏览器会话)
/// 省得让人记 `rm -f $TMPDIR/tke/web/*.json` + `pkill Chrome` 这种命令
#[test]
fn web_control_close_allows_omitting_package() {
    let o = tke().args(["-d", "web", "control", "close"]).output().unwrap();
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(o.status.success(), "web control close 应可省略包名:{}", s);
}

/// 非 web 平台省略包名 → 明确报错(不拿空串去 force-stop)
#[test]
fn nonweb_control_close_requires_package() {
    let o = tke().args(["-d", "emulator-5554", "control", "close"]).output().unwrap();
    assert!(!o.status.success(), "移动端省略包名应失败");
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(s.contains("包名"), "报错应点明需要包名:{}", s);
}

/// `tke run <.tks>` 不给设备、又无从推断(没有同名元素包) → 明确报缺设备
/// (.tks 不记平台,不给会被当 Android 报「adb 缺失」)
#[test]
fn run_requires_device() {
    let d = tmp("need-device");
    let tks = d.join("case.tks");
    std::fs::write(&tks, "步骤:\n返回\n").unwrap();
    let o = tke().args(["run", tks.to_str().unwrap()]).output().unwrap();
    assert!(!o.status.success(), "不给设备应失败");
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(s.contains("设备"), "报错应点名设备:{}", s);
}

/// 造一个 `<dir>/<stem>.tks` + `<stem>.tklib` 两件套,元素包记录平台为 `platform`
fn make_pack(dir: &std::path::Path, stem: &str, platform: &str, device: &str) -> PathBuf {
    let lib_dir = dir.join("lib");
    std::fs::create_dir_all(&lib_dir).unwrap();
    let lib_json = lib_dir.join("element.json");
    std::fs::write(&lib_json, r#"{"elements":{}}"#).unwrap();
    let tks = dir.join(format!("{}.tks", stem));
    std::fs::write(&tks, "步骤:\n返回\n").unwrap();
    tke::utils::tklib::pack(
        &lib_json,
        &tke::utils::tklib::tklib_path(&tks),
        &tke::utils::tklib::TklibMeta::new(platform, device),
    )
    .unwrap();
    tks
}

/// 平台自包含(Q-6):缺 -d 时从同名元素包的 meta.json 读平台。
/// iOS 用例——UDID 不可照搬,仍要求显式给,但报错要把录制时的 UDID 摆出来对照。
#[test]
fn run_infers_platform_from_pack_ios_still_needs_device() {
    let d = tmp("infer-ios");
    let tks = make_pack(&d, "case", "ios", "00008030-ABCD");
    let o = tke().args(["run", tks.to_str().unwrap()]).output().unwrap();
    assert!(!o.status.success(), "iOS 缺设备应失败");
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(s.contains("iOS"), "应点明是 iOS 用例:{}", s);
    assert!(s.contains("00008030-ABCD"), "应附上录制时的 UDID 便于对照:{}", s);
}

/// 元素包里的平台不认识 → 报错要说清是包里的值有问题,别只甩一句缺设备
#[test]
fn run_unknown_platform_in_pack_reports_clearly() {
    let d = tmp("infer-bogus");
    let tks = make_pack(&d, "case", "harmony", "x");
    let o = tke().args(["run", tks.to_str().unwrap()]).output().unwrap();
    assert!(!o.status.success());
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(s.contains("harmony"), "应回显包里那个认不出的平台:{}", s);
}
