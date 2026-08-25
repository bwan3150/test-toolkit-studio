//! HTTP 探测引擎：security 领域一切对目标的请求都从这里走。
//!
//! 为什么抽成 trait：① 真实实现走 ureq；② fake 实现让 recon/探测逻辑能**脱离网络单测**
//! （沿用项目的 FakeDriver/FakeLlm 文化）。AI 工具与 CLI primitive 共用同一份实现——
//! 「一个问题不该有两套答案」。
//!
//! 安全语义的关键取舍：
//!   - **4xx/5xx 是正常响应，照收不报错**——探测的目的就是看状态码，401/500 是信息不是失败。
//!   - **默认不跟随重定向**（`redirects(0)`）——3xx 本身是要观察的对象，跟过去就看不见了。
//!   - **响应体限长**（`MAX_BODY`）——防一条命令把大文件拉爆内存。
//!   - 每个 ureq 调用**必带 timeout**（Q-4 纪律：全链路超时，杜绝无限挂）。

use std::io::Read;
use std::time::{Duration, Instant};

use crate::{Result, TkeError};

/// 响应体最大读取字节（2 MiB）：侦察只需看结构与头部特征，不需要整包大文件。
pub const MAX_BODY: usize = 2 * 1024 * 1024;

/// 一次 HTTP 请求（探测的输入）。
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}

impl HttpRequest {
    pub fn new(method: impl Into<String>, url: impl Into<String>) -> Self {
        Self { method: method.into(), url: url.into(), headers: Vec::new(), body: None }
    }
    pub fn header(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.headers.push((k.into(), v.into()));
        self
    }
    pub fn body(mut self, b: impl Into<Vec<u8>>) -> Self {
        self.body = Some(b.into());
        self
    }
}

/// 一次 HTTP 响应（探测的输出）。`truncated` 标记响应体是否被 `MAX_BODY` 截断。
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub truncated: bool,
    pub elapsed_ms: u128,
}

impl HttpResponse {
    /// 取某个响应头（大小写不敏感）。
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
    /// 响应体按 UTF-8 有损解码（探测里看结构足够，不追求精确编码）。
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// HTTP 引擎抽象。实现方保证：4xx/5xx 作为正常 `HttpResponse` 返回，
/// 只有**传输层**失败（连不上/超时/DNS）才返回 `Err`。
pub trait HttpEngine {
    fn send(&self, req: &HttpRequest) -> Result<HttpResponse>;
}

/// 真实引擎：ureq，带全链路 timeout、默认不跟随重定向。
pub struct UreqEngine {
    timeout: Duration,
}

impl UreqEngine {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl Default for UreqEngine {
    fn default() -> Self {
        Self { timeout: Duration::from_secs(15) }
    }
}

impl HttpEngine for UreqEngine {
    fn send(&self, req: &HttpRequest) -> Result<HttpResponse> {
        let agent = ureq::AgentBuilder::new()
            .timeout(self.timeout)
            .redirects(0) // 3xx 是观察对象，不跟随
            .build();

        let mut rb = agent.request(&req.method, &req.url);
        for (k, v) in &req.headers {
            rb = rb.set(k, v);
        }

        let start = Instant::now();
        let sent = match &req.body {
            Some(b) => rb.send_bytes(b),
            None => rb.call(),
        };

        // 关键：4xx/5xx/3xx 都以 `Status` 形式回来，照收当正常响应；只有 Transport 才是真失败。
        let resp = match sent {
            Ok(r) => r,
            Err(ureq::Error::Status(_code, r)) => r,
            Err(ureq::Error::Transport(t)) => {
                return Err(TkeError::NetworkError(format!("{} {} 传输失败: {}", req.method, req.url, t)));
            }
        };

        let status = resp.status();
        let header_names = resp.headers_names();
        let headers: Vec<(String, String)> = header_names
            .iter()
            .filter_map(|n| resp.header(n).map(|v| (n.clone(), v.to_string())))
            .collect();

        let mut body = Vec::new();
        let mut truncated = false;
        // 只读到 MAX_BODY；多出来的丢弃并打标
        resp.into_reader()
            .take((MAX_BODY + 1) as u64)
            .read_to_end(&mut body)
            .map_err(|e| TkeError::NetworkError(format!("读取响应体失败: {}", e)))?;
        if body.len() > MAX_BODY {
            body.truncate(MAX_BODY);
            truncated = true;
        }

        Ok(HttpResponse { status, headers, body, truncated, elapsed_ms: start.elapsed().as_millis() })
    }
}

/// Fake 引擎：按「方法 + URL 子串」匹配脚本化响应，让探测逻辑脱离网络单测。
pub struct FakeEngine {
    pub routes: Vec<FakeRoute>,
    /// 无匹配路由时是否报传输错误（true）还是返回 404（false，模拟服务器存在但无此路径）。
    pub strict: bool,
}

/// 一条 fake 路由。`method=None` 表示不限方法。
pub struct FakeRoute {
    pub method: Option<String>,
    pub url_contains: String,
    pub resp: HttpResponse,
}

impl FakeEngine {
    pub fn new() -> Self {
        Self { routes: Vec::new(), strict: true }
    }
    /// 便捷：给定 URL 子串 + 状态码 + 头 + 体，压入一条路由。
    pub fn route(
        mut self,
        url_contains: &str,
        status: u16,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Self {
        self.routes.push(FakeRoute {
            method: None,
            url_contains: url_contains.to_string(),
            resp: HttpResponse {
                status,
                headers: headers.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
                body: body.as_bytes().to_vec(),
                truncated: false,
                elapsed_ms: 1,
            },
        });
        self
    }
}

impl Default for FakeEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpEngine for FakeEngine {
    fn send(&self, req: &HttpRequest) -> Result<HttpResponse> {
        for r in &self.routes {
            let method_ok = r.method.as_ref().map_or(true, |m| m.eq_ignore_ascii_case(&req.method));
            if method_ok && req.url.contains(&r.url_contains) {
                return Ok(r.resp.clone());
            }
        }
        if self.strict {
            Err(TkeError::NetworkError(format!("fake: 无匹配路由 {} {}", req.method, req.url)))
        } else {
            Ok(HttpResponse {
                status: 404,
                headers: Vec::new(),
                body: Vec::new(),
                truncated: false,
                elapsed_ms: 1,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_matches_by_url_and_returns_status() {
        let eng = FakeEngine::new().route("/login", 401, &[("content-type", "application/json")], r#"{"error":"x"}"#);
        let resp = eng.send(&HttpRequest::new("POST", "https://t.example/api/login")).unwrap();
        assert_eq!(resp.status, 401);
        assert_eq!(resp.header("Content-Type"), Some("application/json"));
        assert!(resp.text().contains("error"));
    }

    #[test]
    fn fake_strict_errors_on_no_route() {
        let eng = FakeEngine::new();
        assert!(eng.send(&HttpRequest::new("GET", "https://t.example/")).is_err());
    }

    #[test]
    fn fake_non_strict_returns_404() {
        let mut eng = FakeEngine::new();
        eng.strict = false;
        let resp = eng.send(&HttpRequest::new("GET", "https://t.example/missing")).unwrap();
        assert_eq!(resp.status, 404);
    }

    #[test]
    fn header_lookup_is_case_insensitive() {
        let resp = HttpResponse {
            status: 200,
            headers: vec![("Strict-Transport-Security".into(), "max-age=1".into())],
            body: Vec::new(),
            truncated: false,
            elapsed_ms: 0,
        };
        assert!(resp.header("strict-transport-security").is_some());
    }
}
