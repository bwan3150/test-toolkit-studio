// 【黑盒任务层测试】L2:服务端跑 AI 编排、事件流、五态出口、webhook 回调。
//
// **不需要 API key**:测的是任务层的骨架——参数校验、进程起没起来、事件有没有流出来、
// 终态怎么判、设备还没还、回调发没发。任务里的 AI 跑不跑得动是另一回事
// (没配 [ai] 的节点上它会立刻失败,而"失败也要正确收束"恰恰是这里要钉住的)。
//
// 测不到的那部分(真的跑完一次探索、拿到 done 事件、出报告)要真 key,归真机验证。
//
// 跑法:cargo test --no-default-features --test task

mod common;
use common::{get, post, Server, TOKEN};

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::mpsc;

/// 等任务到终态(最多 n 秒);返回最后一次看到的视图
fn wait_finished(s: &Server, id: &str, secs: u64) -> serde_json::Value {
    for _ in 0..(secs * 10) {
        let (_, v) = get(s, &format!("/v1/tasks/{id}"));
        if v["state"] == "finished" {
            return v;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let (_, v) = get(s, &format!("/v1/tasks/{id}"));
    panic!("任务没在 {secs}s 内收束: {v}");
}

#[test]
fn 参数不对当场拒绝并说清楚() {
    let s = Server::start();
    let cases: Vec<(serde_json::Value, &str)> = vec![
        // 强度阶梯的远程口子:最极端那档不经过网络暴露(ADR-0022 D5)
        (serde_json::json!({"kind":"security","target":"https://x","mode":"red-team"}), "人在场"),
        (serde_json::json!({"kind":"security","target":"https://x","mode":"胡乱写"}), "不认识的强度档"),
        (serde_json::json!({"kind":"security"}), "target"),
        (serde_json::json!({"kind":"随便编的"}), "ui / security"),
    ];
    for (body, want) in cases {
        let (code, v) = post(&s, "/v1/tasks", body.clone());
        assert_eq!(code, 400, "{body} 应被拒: {v}");
        assert!(v["error"].as_str().unwrap_or("").contains(want), "{body} 的理由要说清楚: {v}");
    }
}

#[test]
fn 任务跑完要收束并把设备还回去() {
    let s = Server::start();
    let (code, v) = post(
        &s,
        "/v1/tasks",
        serde_json::json!({"kind":"security","target":"http://127.0.0.1:9","timeout_s":60}),
    );
    assert_eq!(code, 202, "起任务应 202(异步): {v}");
    let id = v["task_id"].as_str().unwrap().to_string();
    assert_eq!(v["state"], "running");

    let done = wait_finished(&s, &id, 60);
    // 节点没配 [ai],任务里的 AI 起不来 —— **但任务层必须正确收束**:
    // 没有 done 事件就是没跑完,不能报成功(退出码 0 也不行)
    assert_eq!(done["outcome"], "error", "{done}");
    assert_eq!(done["exit_code"], 4, "五态退出码要对得上 ADR-0009: {done}");
    assert!(done["detail"]["why"].as_str().unwrap_or("").contains("没有正常收束"), "{done}");
    // **失败时必须给出线索**：调用方看不到节点的日志,不给 stderr 尾巴他只知道"失败了",
    // 不知道是缺 API key 还是别的(这次就是缺 key)
    let tail = done["detail"]["stderr_tail"].as_array().expect("错误终态要带 stderr 尾巴");
    assert!(!tail.is_empty() && tail.iter().any(|l| l.as_str().unwrap_or("").contains("Error")), "{done}");
    assert!(done["finished_at"].is_number());

    // 安全任务不该占设备;而且不管成败,会话都要还回去
    let (_, d) = get(&s, "/v1/devices");
    assert!(d["devices"].as_array().unwrap().iter().all(|x| x["available"] == true), "{d}");
    let (_, sess) = get(&s, "/v1/sessions");
    assert_eq!(sess["sessions"].as_array().unwrap().len(), 0, "任务结束要释放会话: {sess}");
}

#[test]
fn 事件流能重放已经发生的() {
    let s = Server::start();
    let (_, v) = post(&s, "/v1/tasks", serde_json::json!({"kind":"security","target":"http://127.0.0.1:9","timeout_s":60}));
    let id = v["task_id"].as_str().unwrap().to_string();
    let done = wait_finished(&s, &id, 60);
    // 至少有终局那条:晚来的订阅者要能看到结局,不能只看到一片空白
    assert!(done["events"].as_u64().unwrap() > 0, "终局事件要进重放缓冲: {done}");

    // 任务早就结束了才来订阅 —— **照样要能看到全过程**
    // (下发完关掉终端、回头再连上的人,不该看不到前半截)
    let r = ureq::get(&format!("{}/v1/tasks/{id}/events", s.base))
        .set("Authorization", &format!("Bearer {TOKEN}"))
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .expect("SSE 拿不到");
    // SSE 是长连接:读一小段就够,不等它关
    let mut buf = [0u8; 4096];
    let n = std::io::Read::read(&mut r.into_reader(), &mut buf).unwrap_or(0);
    let text = String::from_utf8_lossy(&buf[..n]).to_string();
    assert!(text.contains("data:"), "应是 SSE 格式: {text}");
    assert!(text.contains("task_end"), "重放里要有结局: {text}");
}

#[test]
fn headless任务不给交互通道但要说清楚为什么() {
    let s = Server::start();
    let (_, v) = post(&s, "/v1/tasks", serde_json::json!({"kind":"security","target":"http://127.0.0.1:9","timeout_s":60}));
    let id = v["task_id"].as_str().unwrap().to_string();

    // 不是接上一个哑口的连接,而是明说:headless 的问题是**回传**的,不是等人回答的。
    // 要带上真正的 WS 握手头 —— 否则连不到我们的 handler 就被框架的 upgrade 校验挡掉了,
    // 测到的是框架而不是我们的规矩
    let r = ureq::get(&format!("{}/v1/tasks/{id}/session", s.base))
        .set("Authorization", &format!("Bearer {TOKEN}"))
        .set("Connection", "Upgrade")
        .set("Upgrade", "websocket")
        .set("Sec-WebSocket-Version", "13")
        .set("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
        .call();
    let (code, v) = common::resp(r);
    assert_eq!(code, 400, "{v}");
    assert!(v["error"].as_str().unwrap_or("").contains("needs_decision"), "{v}");
    wait_finished(&s, &id, 60);
}

#[test]
fn 还没有报告时说清楚去看哪儿() {
    let s = Server::start();
    let (_, v) = post(&s, "/v1/tasks", serde_json::json!({"kind":"security","target":"http://127.0.0.1:9","timeout_s":60}));
    let id = v["task_id"].as_str().unwrap().to_string();
    wait_finished(&s, &id, 60);
    let (code, v) = get(&s, &format!("/v1/tasks/{id}/report"));
    assert_eq!(code, 404);
    assert!(v["error"].as_str().unwrap_or("").contains("outcome"), "要指路去看终态: {v}");
}

#[test]
fn 终态会回调webhook() {
    // 起一个一次性的 HTTP 收听端:任务结束时应该收到一条带 outcome 的 POST
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut body = String::new();
            let mut len = 0usize;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    break;
                }
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    len = v.trim().parse().unwrap_or(0);
                }
                if line.trim().is_empty() {
                    break;
                }
            }
            if len > 0 {
                let mut buf = vec![0u8; len];
                let _ = std::io::Read::read_exact(&mut reader, &mut buf);
                body = String::from_utf8_lossy(&buf).to_string();
            }
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
            let _ = tx.send(body);
        }
    });

    let s = Server::start();
    let (_, v) = post(
        &s,
        "/v1/tasks",
        serde_json::json!({
            "kind": "security", "target": "http://127.0.0.1:9", "timeout_s": 60,
            "callback_url": format!("http://127.0.0.1:{port}/hook")
        }),
    );
    let id = v["task_id"].as_str().unwrap().to_string();
    wait_finished(&s, &id, 60);

    // "下发完就脱手,结束收报告"全靠这一下:收不到的话用户永远在等
    let body = rx.recv_timeout(std::time::Duration::from_secs(20)).expect("没收到回调");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_else(|e| panic!("回调不是 JSON: {body} ({e})"));
    assert_eq!(v["task_id"], id.as_str());
    assert_eq!(v["outcome"], "error");
    assert!(v["report"].as_str().unwrap_or("").contains(&id), "回调要带报告地址: {v}");
}

#[test]
fn 任务列表看得到跑过的() {
    let s = Server::start();
    let (_, v) = post(&s, "/v1/tasks", serde_json::json!({"kind":"security","target":"http://127.0.0.1:9","timeout_s":60}));
    let id = v["task_id"].as_str().unwrap().to_string();
    wait_finished(&s, &id, 60);
    let (code, v) = get(&s, "/v1/tasks");
    assert_eq!(code, 200);
    assert_eq!(v["tasks"].as_array().unwrap().len(), 1, "{v}");
    assert_eq!(get(&s, "/v1/tasks/不存在").0, 404);
}
