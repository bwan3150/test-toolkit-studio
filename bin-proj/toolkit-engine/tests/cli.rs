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

/// `tke fix --check` 只报告不下载(CI 靠它判断环境是否就绪)
#[test]
fn fix_check_reports_without_downloading() {
    let d = tmp("fix-check");
    // 拷一份 tke 到空目录：那里什么驱动都没有,必然报缺
    let exe = std::path::PathBuf::from(env!("CARGO_BIN_EXE_tke"));
    let mine = d.join("tke");
    std::fs::copy(&exe, &mine).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&mine, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let o = Command::new(&mine).args(["fix", "--check", "--profile", "android"]).output().unwrap();
    let s = format!("{}{}", stdout(&o), stderr(&o));
    // 先清理再断言：这个测试要拷一份 tke 进临时目录（几十 MB），
    // 断言失败就 panic 的话目录会留下来——跑几轮就把 /tmp 撑爆（实际发生过）
    let _ = std::fs::remove_dir_all(&d);
    assert!(s.contains("adb"), "应点名缺 adb:{}", s);
    assert!(!o.status.success(), "缺东西时退出码要非 0,CI 才判得出");
    assert!(s.contains("未下载") || s.contains("check"), "--check 应说明没下载:{}", s);
}

/// fix 的 profile 只认这四个值
#[test]
fn fix_rejects_unknown_profile() {
    let o = tke().args(["fix", "--profile", "harmony"]).output().unwrap();
    assert!(!o.status.success());
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(s.contains("possible values") || s.contains("invalid"), "非法 profile 应被拒:{}", s);
}

/// 帮助里要有 fix(不然使用者不知道有这条命令)
#[test]
fn help_lists_fix() {
    let o = tke().arg("--help").output().unwrap();
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(s.contains("fix"), "帮助应含 fix:{}", s);
}

/// `tke report` 汇总多批产物成一份全流程报告(一次检查会调很多次 steps,碎报告没法审)
#[test]
fn report_merges_batches_into_one() {
    let d = tmp("session-report");
    // 造两批:各带一个 log.json
    for (name, t) in [("steps_a", "2026-08-13T10:00:00+10:00"), ("steps_b", "2026-08-13T10:05:00+10:00")] {
        let bd = d.join(name);
        std::fs::create_dir_all(&bd).unwrap();
        let log = serde_json::json!({
            "success": true, "case_id": "", "script_name": "steps",
            "start_time": t, "end_time": t, "error": null,
            "steps": [{"index":0,"command":"返回","success":true,"error":null,"duration_ms":10}],
        });
        std::fs::write(bd.join("log.json"), log.to_string()).unwrap();
    }

    let o = tke().args(["report", d.to_str().unwrap()]).output().unwrap();
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(o.status.success(), "汇总应成功:{}", s);
    let html = std::fs::read_to_string(d.join("report.html")).expect("应产出 report.html");
    assert_eq!(html.matches(r#"class="batch""#).count(), 2, "两批都要进总报告");
}

/// 目录里没有任何检查记录 → 明确报错,不产出一份骗人的空报告
#[test]
fn report_empty_dir_fails_clearly() {
    let d = tmp("session-empty");
    let o = tke().args(["report", d.to_str().unwrap()]).output().unwrap();
    assert!(!o.status.success());
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(s.contains("没有找到"), "应说清目录里没有记录:{}", s);
}

/// fetch --wait-text / --timeout 装配正确且在帮助里可见(ADR-0013)
#[test]
fn fetch_help_lists_wait_text() {
    let o = tke().args(["fetch", "--help"]).output().unwrap();
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(s.contains("--wait-text"), "fetch 帮助应含 --wait-text:{}", s);
    assert!(s.contains("--timeout"), "fetch 帮助应含 --timeout:{}", s);
}

/// --wait-text 仍然必须给设备——别让人以为"等待"是不需要设备的本地操作
#[test]
fn fetch_wait_text_still_requires_device() {
    let o = tke()
        .args(["fetch", "--wait-text", "随便什么", "--timeout", "1"])
        .output()
        .unwrap();
    assert!(!o.status.success(), "缺 -d 应非零退出");
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(s.contains("设备"), "报错应指出缺设备:{}", s);
}

/// doctor 与 fix 并存，且帮助里都说得清（改名后老写法不能断）
#[test]
fn doctor_and_fix_alias_both_exist() {
    for cmd in ["doctor", "fix"] {
        let o = tke().args([cmd, "--help"]).output().unwrap();
        let s = format!("{}{}", stdout(&o), stderr(&o));
        assert!(o.status.success(), "{} --help 应退出 0:{}", cmd, s);
        assert!(s.contains("--profile"), "{} 应有 --profile:{}", cmd, s);
    }
    // doctor 有 --fix 开关（体检默认不下载）
    let o = tke().args(["doctor", "--help"]).output().unwrap();
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(s.contains("--fix"), "doctor 应有 --fix 开关:{}", s);
}

/// `tke fix --check` 是 install.sh 与用户脚本里的老写法,**不能因为改名而失效**
#[test]
fn fix_check_still_accepted() {
    let o = tke()
        .args(["fix", "--check", "--profile", "web"])
        .env("TKE_BASE_URL", "http://127.0.0.1:9")  // 打不通 → 版本检查静默跳过,不影响体检
        .output()
        .unwrap();
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(s.contains("DOCTOR"), "应走体检输出:{}", s);
    // 只体检就不该出现下载动作
    assert!(!s.contains("要现在下载补齐吗"), "--check 不该询问下载:{}", s);
}

/// update / uninstall 装配正确（ADR-0014：人不该被要求记一串 curl URL）
#[test]
fn update_and_uninstall_are_registered() {
    for cmd in ["update", "uninstall"] {
        let o = tke().args([cmd, "--help"]).output().unwrap();
        let s = format!("{}{}", stdout(&o), stderr(&o));
        assert!(o.status.success(), "{} --help 应退出 0:{}", cmd, s);
    }
    // `tke update` 要**零专属参数**：装的时候已经选过一次 profile，更新时按已装的推断
    let o = tke().args(["update", "--help"]).output().unwrap();
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(!s.contains("--profile"), "update 不该再要人选 profile:{}", s);
    // 卸载只留一个好懂的开关；--dry-run 由确认提示里的清单取代
    let o = tke().args(["uninstall", "--help"]).output().unwrap();
    let s = format!("{}{}", stdout(&o), stderr(&o));
    assert!(s.contains("--all"), "uninstall 应有 --all:{}", s);
    assert!(!s.contains("--dry-run"), "--dry-run 已由确认清单取代:{}", s);
}

/// **绝不执行下载到的非脚本内容**：分发平台对不存在的路径回落 200 + HTML(P-19),
/// `curl … | bash` 会把网页喂给 bash。这里必须先验文件头再执行。
#[test]
fn update_refuses_non_script_payload() {
    let o = tke()
        .args(["update"])
        // 这个路径是 SPA 页面(少了 /sl/preview 前缀)，会回 200 + HTML
        .env("TKE_BASE_URL", "https://cloud.test-toolkit.app/tookit-engine-resource/tke")
        .output()
        .unwrap();
    let s = format!("{}{}", stdout(&o), stderr(&o));
    // 网络不通时会是"取不到"，同样算拒绝——两种都不该执行
    assert!(
        s.contains("不是脚本") || s.contains("取不到"),
        "拿到网页必须拒绝执行:{}",
        s
    );
}
