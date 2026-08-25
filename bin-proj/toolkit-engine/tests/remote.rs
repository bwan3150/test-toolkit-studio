// 【黑盒客户端测试】`TKE_REMOTE` 模式:同一条命令行,发到真节点上跑,结果原样带回来。
//
// 这一层要证明的正是 ADR-0022 D4 那个赌注——**本地怎么敲,远程就怎么敲**:
// stdout 原样、退出码原样、`-d` 翻译成租哪台、`--log` 翻译成产物拉回哪里。
// 赌注不成立的话,两条 remote skill 就得各写一份文档(Q-18)。
//
// 与 tests/serve.rs 的分工:那边站在服务端看协议,这边站在**调用方**看体验。
// 同样不需要设备:用 `device` / `task new` 这类不碰设备的命令。
//
// 跑法:cargo test --no-default-features --test remote

mod common;
use common::{get, Server, TOKEN};

use std::path::PathBuf;
use std::process::{Command, Output};

/// 一个干净的 HOME:远程会话状态落在 `~/.tke/remote/`,
/// 测试之间不能互相看见对方的会话(否则"会不会复用会话"就测不准了)
struct Home(PathBuf);

impl Home {
    fn new(tag: &str) -> Self {
        let p = std::env::temp_dir().join(format!(
            "tke-remote-home-{}-{tag}-{:?}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn tke(s: &Server, home: &Home, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tke"))
        .args(args)
        .env("TKE_REMOTE", &s.base)
        .env("TKE_TOKEN", TOKEN)
        .env("HOME", &home.0)
        .env("USERPROFILE", &home.0) // Windows 上 dirs::home_dir 看这个
        .current_dir(&home.0)
        .output()
        .expect("跑 tke 失败")
}

fn so(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).to_string()
}
fn se(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

#[test]
fn 命令原样发出去结果原样带回来() {
    let s = Server::start();
    let h = Home::new("basic");
    let o = tke(&s, &h, &["-d", "fake:api", "device"]);
    assert!(o.status.success(), "stdout={} stderr={}", so(&o), se(&o));
    // 节点的输出原样透传:调用方看到的东西跟本地一样(这是"文档不分叉"的下半截)
    let v: serde_json::Value = serde_json::from_str(so(&o).trim()).unwrap_or_else(|e| panic!("{}: {e}", so(&o)));
    assert!(v.get("targets").is_some() || v.get("devices").is_some(), "{v}");
    assert!(se(&o).contains("租到"), "第一条命令要自动租一台并说一声: {}", se(&o));
}

#[test]
fn 会话被复用而不是每条命令租一台() {
    let s = Server::start();
    let h = Home::new("reuse");
    for _ in 0..3 {
        let o = tke(&s, &h, &["-d", "fake:api", "device"]);
        assert!(o.status.success(), "{}", se(&o));
    }
    // 每条命令都是新进程,会话靠落盘记住——不然三条命令要租三台设备(池子会瞬间见底)
    let (_, v) = get(&s, "/v1/sessions");
    assert_eq!(v["sessions"].as_array().unwrap().len(), 1, "应当只有一份租约: {v}");
}

#[test]
fn 退出码原样透传() {
    let s = Server::start();
    let h = Home::new("exit");
    // 节点上这条会失败(目录不存在),客户端必须跟着非零退出——
    // 否则调用方会把失败当成功,这是最危险的一种不一致
    let o = tke(&s, &h, &["-d", "fake:api", "report", "logs/不存在"]);
    assert!(!o.status.success(), "stdout={} stderr={}", so(&o), se(&o));
}

#[test]
fn log在两边是同一个相对路径() {
    let s = Server::start();
    let h = Home::new("pull");
    // skill 文档里就是这么写的：`--log <相对目录>` 建任务目录，后面的命令按同一个路径引用它。
    // 远程必须原样成立——否则第二条命令找不到第一条留下的东西（这条曾经真的断过）
    let o = tke(&s, &h, &["task", "new", "--kind", "ui", "--log", "logs/scan"]);
    assert!(o.status.success(), "{}", se(&o));

    // ① 产物按**同一个相对路径**拉回本地
    let pulled = h.0.join("logs/scan/task.json");
    assert!(pulled.exists(), "应拉回 {}: stderr={}", pulled.display(), se(&o));
    let got: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&pulled).unwrap()).unwrap();
    assert_eq!(got["kind"], "ui");

    // ② 节点上那份还在原地：下一条命令用**同一个相对路径**引用得到。
    // 这里 report 会失败（空任务目录本地也一样失败——远程忠实复现本地行为），
    // 但**失败的理由**证明它找到了目录：找不到时的原话是「目录不存在」。
    // 断言理由而不是成败，才测得到"路径对上了"这件事
    let o = tke(&s, &h, &["report", "logs/scan"]);
    let all = format!("{}{}", so(&o), se(&o));
    assert!(all.contains("没有找到任何检查记录"), "第二条命令要能找到第一条留下的任务目录: {all}");
    assert!(!all.contains("目录不存在"), "路径没对上: {all}");
}

#[test]
fn 绝对路径的log当场说清楚() {
    let s = Server::start();
    let h = Home::new("abslog");
    let o = tke(&s, &h, &["-d", "fake:api", "task", "new", "--kind", "ui", "--log", "/tmp/x"]);
    assert!(!o.status.success());
    assert!(se(&o).contains("相对路径"), "{}", se(&o));
}

#[test]
fn 被服务端接管的参数就地吃掉并说一声() {
    let s = Server::start();
    let h = Home::new("dropped");
    let o = tke(&s, &h, &["-d", "fake:api", "--json", "--copilot", "false", "device"]);
    assert!(o.status.success(), "{}", se(&o));
    // 静默吃掉参数 = 让人以为它生效了(INV-9)
    assert!(se(&o).contains("忽略"), "要说一声: {}", se(&o));
}

#[test]
fn 不在白名单的命令不被拦截走本地() {
    let s = Server::start();
    let h = Home::new("local");
    // harness 是任务层的活(L2),远程模式下不该被这层截走;
    // 它会走本地 clap 然后自己报错——报什么无所谓,**不能是"节点拒绝"**
    let o = tke(&s, &h, &["harness"]);
    let all = format!("{}{}", so(&o), se(&o));
    assert!(!all.contains("节点"), "不该发到节点上去: {all}");
    // 本地 --help 也不能被截走
    let o = tke(&s, &h, &["--help"]);
    assert!(o.status.success() && so(&o).contains("tke"), "{}", se(&o));
}

#[test]
fn remote状态与关闭() {
    let s = Server::start();
    let h = Home::new("status");
    tke(&s, &h, &["-d", "fake:api", "device"]);

    let o = tke(&s, &h, &["remote", "status"]);
    assert!(o.status.success(), "{}", se(&o));
    let v: serde_json::Value = serde_json::from_str(so(&o).trim()).unwrap();
    assert_eq!(v["session"]["device"], "fake:api", "{v}");
    // 两边同一个二进制,版本必须对得上;对不上要能一眼看见(Q-11 的教训)
    assert_eq!(v["version_match"], true, "{v}");

    let o = tke(&s, &h, &["remote", "close"]);
    assert!(o.status.success(), "{}", se(&o));
    // 释放要带复位回执,设备要回池
    let v: serde_json::Value = serde_json::from_str(so(&o).trim()).unwrap();
    assert!(v["reset"]["actions"].is_array(), "{v}");
    let (_, d) = get(&s, "/v1/devices");
    assert!(d["devices"].as_array().unwrap().iter().all(|x| x["available"] == true), "{d}");

    // 关掉之后再问状态:不该还记着一个已经没了的会话
    let o = tke(&s, &h, &["remote", "status"]);
    let v: serde_json::Value = serde_json::from_str(so(&o).trim()).unwrap();
    assert!(v["session"].is_null(), "{v}");
}

#[test]
fn 点名换一台设备会换会话() {
    let s = Server::start();
    let h = Home::new("switch");
    tke(&s, &h, &["-d", "fake:api", "device"]);
    let o = tke(&s, &h, &["-d", "web:1", "device"]);
    assert!(o.status.success(), "{}", se(&o));
    let o = tke(&s, &h, &["remote", "status"]);
    let v: serde_json::Value = serde_json::from_str(so(&o).trim()).unwrap();
    // 点名了另一台就得真的换过去——沿用旧会话会让人以为换成功了,然后对着错的设备排查
    assert_eq!(v["session"]["device"], "web:1", "{v}");
}

#[test]
fn 不碰设备的命令不去租设备() {
    let s = Server::start();
    let h = Home::new("nodevice");
    // 安全轨的 recon 只打 URL。让它租一台手机 = 让用户为没用到的设备付租金（ADR-0022 D3）。
    // 用一个必然连不上的地址：命令会失败，但**会话是怎么开的**才是这条测的东西
    let o = tke(&s, &h, &["recon", "headers", "http://127.0.0.1:9"]);
    let _ = o;
    let (_, d) = get(&s, "/v1/devices");
    assert!(
        d["devices"].as_array().unwrap().iter().all(|x| x["available"] == true),
        "不该占用任何真设备: {d}"
    );
    let o = tke(&s, &h, &["remote", "status"]);
    let v: serde_json::Value = serde_json::from_str(so(&o).trim()).unwrap();
    assert_eq!(v["session"]["device"], "", "开的应该是无设备会话: {v}");
}

#[test]
fn 没配节点时报错说清楚怎么配() {
    let h = Home::new("noenv");
    let o = Command::new(env!("CARGO_BIN_EXE_tke"))
        .args(["remote", "status"])
        .env_remove("TKE_REMOTE")
        .env("HOME", &h.0)
        .output()
        .unwrap();
    assert!(!o.status.success());
    let all = format!("{}{}", so(&o), se(&o));
    assert!(all.contains("TKE_REMOTE"), "要告诉人怎么配: {all}");
}
