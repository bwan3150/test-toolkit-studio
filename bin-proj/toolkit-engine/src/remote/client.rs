// 【HTTP 客户端】远程节点的调用面。用 ureq（同步）——web 驱动已经在用它，
// 不为了这件事再引一套异步客户端。

use super::state::RemoteSession;

pub struct Client {
    pub base: String,
    pub token: Option<String>,
}

type R<T> = Result<T, String>;

impl Client {
    fn req(&self, method: &str, path: &str) -> ureq::Request {
        let r = ureq::request(method, &format!("{}{}", self.base, path));
        match &self.token {
            Some(t) => r.set("Authorization", &format!("Bearer {t}")),
            None => r,
        }
    }

    fn json(r: Result<ureq::Response, ureq::Error>) -> R<serde_json::Value> {
        match r {
            Ok(resp) => resp.into_json().map_err(|e| format!("响应不是 JSON: {e}")),
            // 节点的拒绝理由是写给人看的，原样带出来——别包一层"请求失败"把它埋掉（P-46）
            Err(ureq::Error::Status(code, resp)) => {
                let body: serde_json::Value = resp.into_json().unwrap_or(serde_json::Value::Null);
                let why = body.get("error").and_then(|e| e.as_str()).unwrap_or("(节点没给理由)");
                Err(format!("节点拒绝（HTTP {code}）：{why}"))
            }
            Err(e) => Err(format!("连不上节点: {e}")),
        }
    }

    pub fn hello(&self) -> R<serde_json::Value> {
        Self::json(self.req("GET", "/v1/hello").call())
    }

    pub fn devices(&self) -> R<serde_json::Value> {
        Self::json(self.req("GET", "/v1/devices").call())
    }

    pub fn create_session(&self, platform: Option<&str>, device_id: Option<&str>, ttl_s: u64) -> R<serde_json::Value> {
        let mut caps = serde_json::Map::new();
        if let Some(p) = platform {
            caps.insert("platform".into(), p.into());
        }
        if let Some(d) = device_id {
            caps.insert("device_id".into(), d.into());
        }
        Self::json(self.req("POST", "/v1/sessions").send_json(serde_json::json!({
            "capabilities": caps, "ttl_s": ttl_s
        })))
    }

    pub fn heartbeat(&self, sid: &str, ttl_s: u64) -> R<serde_json::Value> {
        Self::json(
            self.req("POST", &format!("/v1/sessions/{sid}/heartbeat"))
                .send_json(serde_json::json!({"ttl_s": ttl_s})),
        )
    }

    pub fn exec(&self, sid: &str, argv: &[String], timeout_s: u64) -> R<serde_json::Value> {
        Self::json(
            self.req("POST", &format!("/v1/sessions/{sid}/exec"))
                // 客户端侧的超时要比服务端宽：否则节点还在跑、这边先断了，
                // 结果是"看起来失败了但其实做了"——最难查的一种
                .timeout(std::time::Duration::from_secs(timeout_s + 30))
                .send_json(serde_json::json!({"argv": argv, "timeout_s": timeout_s})),
        )
    }

    pub fn release(&self, sid: &str) -> R<serde_json::Value> {
        Self::json(self.req("DELETE", &format!("/v1/sessions/{sid}")).call())
    }

    pub fn list_artifacts(&self, sid: &str, rel: &str) -> R<Vec<String>> {
        // 空 rel = 整个工作区：走不带路径那条路由（拼成 `artifacts/?list=true` 会 404）
        let path = if rel.is_empty() {
            format!("/v1/sessions/{sid}/artifacts")
        } else {
            format!("/v1/sessions/{sid}/artifacts/{rel}?list=true")
        };
        let v = Self::json(self.req("GET", &path).call())?;
        Ok(v["files"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default())
    }

    pub fn get_artifact(&self, sid: &str, rel: &str) -> R<Vec<u8>> {
        match self.req("GET", &format!("/v1/sessions/{sid}/artifacts/{rel}")).call() {
            Ok(resp) => {
                let mut buf = Vec::new();
                std::io::copy(&mut resp.into_reader(), &mut buf).map_err(|e| format!("读 {rel} 失败: {e}"))?;
                Ok(buf)
            }
            Err(e) => Err(format!("下载 {rel} 失败: {e}")),
        }
    }

    pub fn put_workspace(&self, sid: &str, rel: &str, bytes: Vec<u8>) -> R<serde_json::Value> {
        Self::json(self.req("PUT", &format!("/v1/sessions/{sid}/workspace/{rel}")).send_bytes(&bytes))
    }

    /// 把这次新产生的产物拉回本地。`subtree` = 只拉工作区里的这棵子树（`--log` 给的那个相对路径），
    /// 空 = 整个工作区。落点是 `base/<相对路径>`——**与节点上的相对路径一致**，
    /// 这样本地和远程看到的目录结构是同一个。
    /// **只拉新的和变大的**（截图序列会越来越长，每条命令重下一遍是纯浪费）；返回这次拉了几个
    pub fn pull_new(&self, sess: &mut RemoteSession, base: &std::path::Path, subtree: &str) -> R<usize> {
        let files = self.list_artifacts(&sess.session_id, subtree)?;
        let mut n = 0;
        for rel in files {
            let bytes = match self.get_artifact(&sess.session_id, &rel) {
                Ok(b) => b,
                // 单个文件拉不下来不该让整条命令失败，但也不许静默（INV-9）
                Err(e) => {
                    eprintln!("⚠️  产物 {rel} 没拉下来：{e}");
                    continue;
                }
            };
            let len = bytes.len() as u64;
            if sess.pulled.get(&rel) == Some(&len) {
                continue;
            }
            let dest = base.join(&rel);
            if let Some(p) = dest.parent() {
                std::fs::create_dir_all(p).map_err(|e| format!("建目录失败: {e}"))?;
            }
            std::fs::write(&dest, &bytes).map_err(|e| format!("写 {} 失败: {e}", dest.display()))?;
            sess.pulled.insert(rel, len);
            n += 1;
        }
        Ok(n)
    }
}
