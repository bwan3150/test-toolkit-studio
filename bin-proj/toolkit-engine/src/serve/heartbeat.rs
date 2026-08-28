// 【心跳】节点主动向测试管理平台报到（ADR-0023 D7）。
//
// 为什么是节点报而不是平台轮询（用户 2026-08-26 拍板）：轮询是平台去敲 N 个节点——
// 节点挂了要等超时、节点多了是 N 倍请求、平台还得维护一份"有哪些节点"的清单。
// 心跳反过来：**节点自己报，死了就是不报了**，平台只需要一张表和一个超时判据。
//
// 协议就一个端点，**幂等 upsert**：第一次心跳就是注册。节点不需要知道自己的 node_id，
// 也就不需要"先注册拿 id 再心跳"那一步——少一个会失败的环节，少一份要持久化的状态。
//
// **每次带全量设备清单**，平台整份替换。事件式（插了一台推一条）漏一条就永久错位，
// 全量替换会自愈：下一次心跳就对上了。清单很小，这点开销可以忽略。
//
// 节点在这条链路上只多持有一样东西：`--platform-token`。**业务凭据一概没有**——
// 它不认识 App、不认识用户、不持有 AI key（那些由平台随任务下发，见 ADR-0023 D6）。

use std::sync::Arc;
use std::time::Duration;

use super::ServeState;

/// 平台对接配置
#[derive(Clone)]
pub struct PlatformLink {
    /// 平台地址，如 `https://test-platform.example`
    pub base: String,
    /// 节点报到用的凭据（平台先建节点行、给出 token）
    pub token: String,
    /// 节点名（给人看的）
    pub name: String,
    /// **我怎么被够着** —— 节点自报，平台不猜。
    /// 不给就用监听地址，但那多半是 0.0.0.0/127.0.0.1，平台够不着
    pub advertise: Option<String>,
}

/// 平台回的东西
#[derive(Debug, Default, serde::Deserialize)]
struct HeartbeatReply {
    #[serde(default)]
    node_id: String,
    /// **周期由平台定**，节点照做。想让所有节点降频不用挨个改配置
    #[serde(default)]
    interval_s: Option<u64>,
}

/// 默认心跳周期；平台没给 `interval_s` 时用它
const DEFAULT_INTERVAL: u64 = 15;
/// 心跳周期的下限——平台给了个离谱的小值也不能把自己打死
const MIN_INTERVAL: u64 = 5;

fn payload(st: &ServeState, link: &PlatformLink, listen: &str) -> serde_json::Value {
    let pool = st.leases.pool();
    let busy: Vec<String> = st
        .leases
        .active()
        .iter()
        .map(|l| l.device.id.clone())
        .filter(|id| !id.is_empty())
        .collect();
    serde_json::json!({
        "name": link.name,
        "base_url": link.advertise.clone().unwrap_or_else(|| format!("http://{listen}")),
        "host_os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "tke_version": env!("BUILD_VERSION"),
        "devices": pool.iter().map(|d| serde_json::json!({
            "id": d.id, "kind": d.kind, "platform": d.platform(),
            "model": d.model, "os": d.os, "label": d.label,
            "ready": true, "physical": d.physical(),
        })).collect::<Vec<_>>(),
        // 节点自己认为正被占用的。**对账用，不作为真相**——计时以平台为准（节点会重启）
        "busy": busy,
    })
}

fn post(link: &PlatformLink, path: &str, body: serde_json::Value) -> Result<HeartbeatReply, String> {
    let url = format!("{}{}", link.base.trim_end_matches('/'), path);
    match ureq::post(&url)
        .set("Authorization", &format!("Bearer {}", link.token))
        .timeout(Duration::from_secs(15))
        .send_json(body)
    {
        Ok(resp) => {
            let v: serde_json::Value = resp.into_json().unwrap_or(serde_json::Value::Null);
            // 平台的统一响应是 {code,data,msg}，我们要的在 data 里
            let data = v.get("data").cloned().unwrap_or(v);
            Ok(serde_json::from_value(data).unwrap_or_default())
        }
        // 平台的拒绝理由原样带出来——被包一层"心跳失败"就查不下去了（P-46）
        Err(ureq::Error::Status(code, resp)) => {
            let body: serde_json::Value = resp.into_json().unwrap_or(serde_json::Value::Null);
            let why = body.get("msg").and_then(|m| m.as_str()).unwrap_or("(平台没给理由)");
            Err(format!("平台拒绝（HTTP {code}）：{why}"))
        }
        Err(e) => Err(format!("连不上平台: {e}")),
    }
}

/// 起心跳循环（后台任务）。
///
/// **连不上平台不影响节点自己干活**：报不上去只是平台看不见它，
/// 本地的 exec / 任务照跑。所以这里只记日志、退避重试，绝不退出进程。
pub fn spawn(st: Arc<ServeState>, link: PlatformLink, listen: String) {
    tokio::spawn(async move {
        let mut interval = DEFAULT_INTERVAL;
        let mut failures = 0u32;
        loop {
            let body = payload(&st, &link, &listen);
            let l = link.clone();
            let r = tokio::task::spawn_blocking(move || post(&l, "/api/v1/node/heartbeat", body)).await;
            let err: Option<String> = match r {
                Ok(Ok(reply)) => {
                    if failures > 0 {
                        tracing::info!(target: "tke::heartbeat", "已重新连上平台（node_id={}）", reply.node_id);
                    }
                    failures = 0;
                    interval = reply.interval_s.unwrap_or(DEFAULT_INTERVAL).max(MIN_INTERVAL);
                    None
                }
                Ok(Err(e)) => Some(e),
                Err(e) => Some(format!("心跳任务异常: {e}")),
            };
            if let Some(msg) = err {
                failures += 1;
                // 只在第一次和每 20 次失败时喊一声——平台维护一小时的话，
                // 每 15 秒一条 WARN 会把日志淹掉，而信息量只有第一条
                if failures == 1 || failures % 20 == 0 {
                    tracing::warn!(target: "tke::heartbeat", "心跳失败（第 {failures} 次）：{msg}");
                }
                // 退避：失败越久报得越稀，封顶 2 分钟
                interval = (DEFAULT_INTERVAL * failures.min(8) as u64).min(120);
            }
            tokio::time::sleep(Duration::from_secs(interval)).await;
        }
    });
}

/// 优雅退出时说一声。收不到也没关系——平台的 `last_seen_at` 超时一样会判 offline，
/// 这只是让它快一点
pub fn offline(link: &PlatformLink) {
    if let Err(e) = post(link, "/api/v1/node/offline", serde_json::json!({"name": link.name})) {
        tracing::warn!(target: "tke::heartbeat", "下线通知没发出去（不影响，平台会超时判定）：{e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serve::lease::{LeaseTable, PoolDevice};

    fn state() -> Arc<ServeState> {
        let tmp = std::env::temp_dir().join(format!("tke-hb-{}", std::process::id()));
        let pool = vec![
            PoolDevice { id: "web:1".into(), kind: "web".into(), label: "Chrome 无头 #1".into() , model: String::new(), os: String::new() },
            PoolDevice { id: "f64b3b4d".into(), kind: "android".into(), label: "CPH2305".into() , model: String::new(), os: String::new() },
        ];
        Arc::new(ServeState {
            tasks: crate::serve::task::TaskTable::new(),
            token: None,
            bin: std::path::PathBuf::from("/bin/true"),
            leases: LeaseTable::new(tmp, pool, Duration::from_secs(600)),
            default_timeout: Duration::from_secs(10),
            max_upload_bytes: 1024,
            local_ws_base: "ws://127.0.0.1:0".into(),
            sandbox_root: None,
        })
    }

    fn link() -> PlatformLink {
        PlatformLink {
            base: "https://p.example".into(),
            token: "t".into(),
            name: "node-1".into(),
            advertise: Some("https://node-1.internal:8787".into()),
        }
    }

    #[test]
    fn 心跳带全量设备清单() {
        // 全量替换而不是推事件：漏一条事件就永久错位，全量下一次心跳就自愈
        let body = payload(&state(), &link(), "0.0.0.0:8787");
        let devs = body["devices"].as_array().unwrap();
        assert_eq!(devs.len(), 2);
        assert_eq!(devs[0]["id"], "web:1");
        assert_eq!(devs[1]["platform"], "android");
        assert_eq!(body["tke_version"], env!("BUILD_VERSION"));
    }

    #[test]
    fn 自报可达地址而不是监听地址() {
        // 监听地址多半是 0.0.0.0 / 127.0.0.1，平台照着它根本够不着
        let body = payload(&state(), &link(), "0.0.0.0:8787");
        assert_eq!(body["base_url"], "https://node-1.internal:8787");

        let mut l = link();
        l.advertise = None;
        let body = payload(&state(), &l, "10.0.0.5:8787");
        assert_eq!(body["base_url"], "http://10.0.0.5:8787", "没给 advertise 才回落到监听地址");
    }

    #[test]
    fn 占用中的设备一起报上去对账() {
        let st = state();
        let l = st.leases.acquire(Some("android"), None, None).unwrap();
        let body = payload(&st, &link(), "x");
        assert_eq!(body["busy"], serde_json::json!(["f64b3b4d"]));
        st.leases.take(&l.id);
        assert_eq!(payload(&st, &link(), "x")["busy"], serde_json::json!([]));
    }

    #[test]
    fn 无设备会话不进占用清单() {
        // 它压根没占设备（安全轨只打 URL），报上去会让平台以为有台设备忙着
        let st = state();
        st.leases.acquire(Some("none"), None, None).unwrap();
        assert_eq!(payload(&st, &link(), "x")["busy"], serde_json::json!([]));
    }
}
