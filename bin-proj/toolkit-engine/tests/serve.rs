// 【黑盒接口测试】起真的 `tke serve`,发真的 HTTP,跑真的子进程——但**不需要设备**。
//
// 这一层测的是 src 内单测覆盖不到的东西:路由装配、鉴权中间件、
// 请求体反序列化、真子进程的参数注入是否被 clap 接受、产物读写、租约的 HTTP 语义。
// 单测测得了"白名单该拒谁",测不了"拒了之后 HTTP 是不是 400"。
//
// 不需要设备是有意的:执行用的是 `task new` / `device` / `report` 这类不碰设备的命令,
// 会话仍然要租一台(租约是必经之路),租的是 `--fake-device` 塞进池子的假设备。
// **接口调用真的把设备操作了**那一层归 `tests/e2e/serve-smoke.sh`(要真浏览器/真机)。
//
// 跑法:cargo test --no-default-features --test serve

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

const TOKEN: &str = "test-token-1234";

/// 起一个 serve,读它打印的监听行拿到真实端口(`--port 0`)
struct Server {
    child: Child,
    base: String,
    root: PathBuf,
}

impl Server {
    fn start() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tke-serve-test-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_tke"))
            .args(["serve", "--port", "0", "--token", TOKEN])
            .args(["--root".as_ref(), root.as_os_str()])
            .args(["--fake-device", "fake:api", "--web-slots", "2", "--exec-timeout", "60"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("起 tke serve 失败");

        // 监听行是契约:`{"success":true,"listening":"127.0.0.1:PORT",...}`
        let out = child.stdout.take().unwrap();
        let mut lines = BufReader::new(out).lines();
        let line = lines.next().expect("serve 没打印监听行").expect("读监听行失败");
        let v: serde_json::Value = serde_json::from_str(&line).unwrap_or_else(|e| panic!("监听行不是 JSON: {line} ({e})"));
        let addr = v["listening"].as_str().expect("监听行缺 listening").to_string();
        Self { child, base: format!("http://{addr}"), root }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

// ureq 已经是依赖(web 驱动在用),测试直接借它发请求,不引新东西
fn get(s: &Server, path: &str) -> (u16, serde_json::Value) {
    resp(ureq::get(&s.url(path)).set("Authorization", &format!("Bearer {TOKEN}")).call())
}

fn post(s: &Server, path: &str, body: serde_json::Value) -> (u16, serde_json::Value) {
    resp(ureq::post(&s.url(path)).set("Authorization", &format!("Bearer {TOKEN}")).send_json(body))
}

fn del(s: &Server, path: &str) -> (u16, serde_json::Value) {
    resp(ureq::delete(&s.url(path)).set("Authorization", &format!("Bearer {TOKEN}")).call())
}

fn resp(r: Result<ureq::Response, ureq::Error>) -> (u16, serde_json::Value) {
    match r {
        Ok(r) => {
            let code = r.status();
            (code, r.into_json().unwrap_or(serde_json::Value::Null))
        }
        Err(ureq::Error::Status(code, r)) => (code, r.into_json().unwrap_or(serde_json::Value::Null)),
        Err(e) => panic!("请求失败: {e}"),
    }
}

fn new_session(s: &Server) -> String {
    let (code, v) = post(s, "/v1/sessions", serde_json::json!({"capabilities": {"platform": "fake"}}));
    assert_eq!(code, 201, "建会话应 201: {v}");
    v["session_id"].as_str().unwrap().to_string()
}

#[test]
fn 没带凭据一律401() {
    let s = Server::start();
    let r = ureq::get(&s.url("/v1/hello")).call();
    let (code, _) = resp(r);
    assert_eq!(code, 401, "裸调用必须被挡");
    // 带错的也一样
    let r = ureq::get(&s.url("/v1/hello")).set("Authorization", "Bearer wrong").call();
    assert_eq!(resp(r).0, 401);
}

#[test]
fn 握手给出版本与白名单() {
    let s = Server::start();
    let (code, v) = get(&s, "/v1/hello");
    assert_eq!(code, 200);
    assert_eq!(v["api_version"], "v1");
    assert!(v["tke_version"].is_string(), "要给版本,client/node 漂移得能立刻看出来");
    let cmds: Vec<String> = serde_json::from_value(v["allowed_commands"].clone()).unwrap();
    assert!(cmds.contains(&"control".to_string()));
    assert!(!cmds.contains(&"harness".to_string()), "AI 编排不在命令层");
}

#[test]
fn 设备清单标出谁在租() {
    let s = Server::start();
    let (_, v) = get(&s, "/v1/devices");
    let devs = v["devices"].as_array().unwrap();
    assert!(devs.iter().any(|d| d["id"] == "web:1"), "浏览器槽位要在池里: {v}");
    assert!(devs.iter().any(|d| d["id"] == "fake:api"));
    assert!(devs.iter().all(|d| d["available"] == true), "还没人租");

    let sid = new_session(&s);
    let (_, v) = get(&s, "/v1/devices");
    let fake = v["devices"].as_array().unwrap().iter().find(|d| d["id"] == "fake:api").unwrap().clone();
    assert_eq!(fake["available"], false);
    assert_eq!(fake["leased_by"], sid.as_str());
}

#[test]
fn 同一台设备第二个人来租拿到409() {
    let s = Server::start();
    let _sid = new_session(&s);
    let (code, v) = post(&s, "/v1/sessions", serde_json::json!({"capabilities": {"platform": "fake"}}));
    // 409 而不是 404:设备存在、只是被占着——调用方该等,不是该换节点
    assert_eq!(code, 409, "{v}");
    let (code, v) = post(&s, "/v1/sessions", serde_json::json!({"capabilities": {"platform": "ios"}}));
    assert_eq!(code, 404, "本节点没有这类设备,是另一回事: {v}");
}

#[test]
fn 执行真的跑起了子进程并落了文件() {
    let s = Server::start();
    let sid = new_session(&s);
    // task new 不碰设备,但会真的建目录 + 写 task.json(ADR-0021 的生命周期起点)
    let (code, v) = post(
        &s,
        &format!("/v1/sessions/{sid}/exec"),
        serde_json::json!({"argv": ["task", "new", "--kind", "ui", "--dir", "logs/t1"]}),
    );
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["exit_code"], 0, "{v}");
    assert_eq!(v["stdout_json"]["success"], true, "{v}");
    // 分层计时要在(Q-17:先量再优化)
    assert!(v["timing"]["spawn_ms"].is_number() && v["timing"]["total_ms"].is_number(), "{v}");

    // 产物接口能看到它写出来的文件
    let (code, v) = get(&s, &format!("/v1/sessions/{sid}/artifacts/logs?list=true"));
    assert_eq!(code, 200);
    let files: Vec<String> = serde_json::from_value(v["files"].clone()).unwrap();
    assert!(files.iter().any(|f| f.ends_with("task.json")), "落点应在会话目录内: {files:?}");
}

#[test]
fn 注入的参数被clap接受() {
    let s = Server::start();
    let sid = new_session(&s);
    // 这条测的是"注入顺序对不对":全局参数放子命令后面会撞 P-44,
    // 单测只能证明顺序,证明不了 clap 真的收
    let (code, v) = post(&s, &format!("/v1/sessions/{sid}/exec"), serde_json::json!({"argv": ["device"]}));
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["exit_code"], 0, "注入的 --json/--log/--cache/--current-dir/-d/--copilot 应被接受: {v}");
}

#[test]
fn 白名单在http层就把人挡住() {
    let s = Server::start();
    let sid = new_session(&s);
    let cases: Vec<(Vec<&str>, &str)> = vec![
        (vec!["harness", "随便"], "任务层"),
        (vec!["update"], "运维"),
        (vec!["doctor", "--fix"], "开放"),
        (vec!["fetch", "--log", "/etc"], "服务端"),
        (vec!["fetch", "-d", "web:1"], "服务端"),
        (vec!["ocr", "--image", "/etc/passwd"], "越界"),
        (vec!["ocr", "--image", "../../etc/passwd"], "越界"),
        (vec!["control", "--", "rm"], "--"),
    ];
    for (argv, want) in cases {
        let (code, v) = post(&s, &format!("/v1/sessions/{sid}/exec"), serde_json::json!({"argv": argv}));
        assert_eq!(code, 400, "{argv:?} 应被拒: {v}");
        let err = v["error"].as_str().unwrap_or("");
        assert!(err.contains(want), "{argv:?} 的拒绝理由要说清楚(期望含 `{want}`): {err}");
    }
}

#[test]
fn 上传与下载走同一个沙箱() {
    let s = Server::start();
    let sid = new_session(&s);
    let body = "启动环境\n打开网页 [https://example.com]\n";
    let r = ureq::put(&s.url(&format!("/v1/sessions/{sid}/workspace/cases/login.tks")))
        .set("Authorization", &format!("Bearer {TOKEN}"))
        .send_string(body);
    let (code, v) = resp(r);
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["bytes"], body.len());

    let got = ureq::get(&s.url(&format!("/v1/sessions/{sid}/artifacts/cases/login.tks")))
        .set("Authorization", &format!("Bearer {TOKEN}"))
        .call()
        .unwrap()
        .into_string()
        .unwrap();
    assert_eq!(got, body);

    // 跳出工作区一律拒(与 exec 的路径参数同一条规则、同一处实现)。
    // **必须用百分号编码**:裸 `../..` 会被 HTTP 客户端在本地就归一化掉,
    // 那样测的是客户端而不是服务端——编码过的才真的把 `..` 送到服务端手里
    let r = ureq::put(&s.url(&format!("/v1/sessions/{sid}/workspace/%2e%2e%2f%2e%2e%2fevil.txt")))
        .set("Authorization", &format!("Bearer {TOKEN}"))
        .send_string("x");
    let (code, v) = resp(r);
    assert_eq!(code, 400, "跳出工作区必须被拒: {v}");
    assert!(!s.root.join("evil.txt").exists() && !std::env::temp_dir().join("evil.txt").exists());
}

#[test]
fn 会话生命周期完整走一遍() {
    let s = Server::start();
    let sid = new_session(&s);

    let (code, v) = get(&s, &format!("/v1/sessions/{sid}"));
    assert_eq!(code, 200);
    let first_exp = v["expires_at"].as_u64().unwrap();

    let (code, v) = post(&s, &format!("/v1/sessions/{sid}/heartbeat"), serde_json::json!({"ttl_s": 3600}));
    assert_eq!(code, 200);
    assert!(v["expires_at"].as_u64().unwrap() >= first_exp, "心跳要能续命");

    let (code, v) = get(&s, "/v1/sessions");
    assert_eq!(code, 200);
    assert_eq!(v["sessions"].as_array().unwrap().len(), 1);

    // 释放要带复位回执(INV-17):fake 设备没有复位动作,但字段必须在——
    // 静默的复位等于没有复位
    let (code, v) = del(&s, &format!("/v1/sessions/{sid}"));
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["released"], true);
    assert!(v["reset"]["actions"].is_array() && v["reset"]["elapsed_ms"].is_number(), "{v}");

    // 释放后设备回池、会话没了
    assert_eq!(get(&s, &format!("/v1/sessions/{sid}")).0, 404);
    assert_eq!(post(&s, &format!("/v1/sessions/{sid}/exec"), serde_json::json!({"argv": ["device"]})).0, 404);
    let (_, v) = get(&s, "/v1/devices");
    assert!(v["devices"].as_array().unwrap().iter().all(|d| d["available"] == true));
}

#[test]
fn 会话之间目录互不可见() {
    let s = Server::start();
    // 两个会话租不同设备(fake:api 与 web:1),各写各的,谁也看不见谁——P-10 由设计消除
    let (_, a) = post(&s, "/v1/sessions", serde_json::json!({"capabilities": {"device_id": "fake:api"}}));
    let (_, b) = post(&s, "/v1/sessions", serde_json::json!({"capabilities": {"device_id": "web:1"}}));
    let (a, b) = (a["session_id"].as_str().unwrap(), b["session_id"].as_str().unwrap());

    ureq::put(&s.url(&format!("/v1/sessions/{a}/workspace/mine.txt")))
        .set("Authorization", &format!("Bearer {TOKEN}"))
        .send_string("A 的东西")
        .unwrap();
    let (code, _) = get(&s, &format!("/v1/sessions/{b}/artifacts/mine.txt"));
    assert_eq!(code, 404, "B 不该看得见 A 的文件");
}
