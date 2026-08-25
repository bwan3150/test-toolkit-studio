// 【测试共享件】起一个真的 `tke serve`,拿到真实端口。
// serve.rs（服务端契约）与 remote.rs（客户端）都要它——同一个起法只写一遍。
#![allow(dead_code)] // 两个测试目标各用一部分

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

pub const TOKEN: &str = "test-token-1234";

pub struct Server {
    child: Child,
    pub base: String,
    pub root: PathBuf,
}

impl Server {
    pub fn start() -> Self {
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
            // 客户端测试会给子进程设 TKE_REMOTE,服务端自己**绝不能**也带着它——
            // 那样节点上的子进程会把命令再转发一次,兜圈子
            .env_remove("TKE_REMOTE")
            .env_remove("TKE_TOKEN")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("起 tke serve 失败");

        // 监听行是契约:`{"success":true,"listening":"127.0.0.1:PORT",...}`
        let out = child.stdout.take().unwrap();
        let line = BufReader::new(out).lines().next().expect("serve 没打印监听行").expect("读监听行失败");
        let v: serde_json::Value =
            serde_json::from_str(&line).unwrap_or_else(|e| panic!("监听行不是 JSON: {line} ({e})"));
        let addr = v["listening"].as_str().expect("监听行缺 listening").to_string();
        Self { child, base: format!("http://{addr}"), root }
    }

    pub fn url(&self, path: &str) -> String {
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

pub fn get(s: &Server, path: &str) -> (u16, serde_json::Value) {
    resp(ureq::get(&s.url(path)).set("Authorization", &format!("Bearer {TOKEN}")).call())
}

pub fn post(s: &Server, path: &str, body: serde_json::Value) -> (u16, serde_json::Value) {
    resp(ureq::post(&s.url(path)).set("Authorization", &format!("Bearer {TOKEN}")).send_json(body))
}

pub fn del(s: &Server, path: &str) -> (u16, serde_json::Value) {
    resp(ureq::delete(&s.url(path)).set("Authorization", &format!("Bearer {TOKEN}")).call())
}

pub fn resp(r: Result<ureq::Response, ureq::Error>) -> (u16, serde_json::Value) {
    match r {
        Ok(r) => {
            let code = r.status();
            (code, r.into_json().unwrap_or(serde_json::Value::Null))
        }
        Err(ureq::Error::Status(code, r)) => (code, r.into_json().unwrap_or(serde_json::Value::Null)),
        Err(e) => panic!("请求失败: {e}"),
    }
}
