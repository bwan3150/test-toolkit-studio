//! `tke serve` —— 把这台机器的能力开成 HTTP 接口（ADR-0022 P1）。
//!
//! 参数翻译层，业务在 `tke::serve`（INV-10：cli 不放业务逻辑）。

use std::path::PathBuf;
use std::time::Duration;

use tke::{Result, TkeError};

#[derive(clap::Args)]
pub struct ServeArgs {
    /// 监听地址。**非回环必须给 --token**——不设防的端口能操作真机
    #[arg(long, default_value = "127.0.0.1")]
    pub bind: String,

    /// 监听端口；`0` = 随便挑一个（起来后打印真实端口，接口测试靠它）
    #[arg(long, default_value_t = 8787)]
    pub port: u16,

    /// 访问凭据（也可用环境变量 `TKE_SERVE_TOKEN`）
    #[arg(long)]
    pub token: Option<String>,

    /// 会话根目录（每个会话一个隔离子目录，INV-17）
    #[arg(long)]
    pub root: Option<PathBuf>,

    /// 会话默认存活秒数；心跳续命，断了就回收并复位设备
    #[arg(long, default_value_t = 1800)]
    pub ttl: u64,

    /// 单条命令的默认超时秒数（请求体里可以按次覆盖）
    #[arg(long, default_value_t = 120)]
    pub exec_timeout: u64,

    /// 同时能开几个无头浏览器（`web:1` … `web:N`）
    #[arg(long, default_value_t = 4)]
    pub web_slots: u8,

    /// 上传体积上限（MB）：APK/IPA 走这条路
    #[arg(long, default_value_t = 256)]
    pub max_upload_mb: usize,

    /// 测试专用：往设备池里塞 `fake:` 设备（可多次给）
    #[arg(long, hide = true)]
    pub fake_device: Vec<String>,

    // ===== 向测试管理平台报到（可选）=====
    /// 平台地址，如 `https://test-platform.example`。给了就每隔十几秒报一次到
    #[arg(long)]
    pub platform: Option<String>,

    /// 节点报到用的凭据（也可用环境变量 `TKE_PLATFORM_TOKEN`）。
    /// **这是节点唯一的第二样凭据**——业务凭据（AI key 之类）一概由平台随任务下发
    #[arg(long)]
    pub platform_token: Option<String>,

    /// 这个节点在平台上显示的名字（默认取主机名）
    #[arg(long)]
    pub node_name: Option<String>,

    /// **平台怎么够着我** —— 如 `https://node-1.internal:8787`。
    /// 不给就用监听地址，但那多半是 `0.0.0.0`/`127.0.0.1`，平台照着它连不上。
    /// 走 `--link` 时用不上它（那条路平台不需要够得着节点）
    #[arg(long)]
    pub advertise: Option<String>,

    /// **反向通道**：节点主动连平台，之后所有指令都在这条连接上跑（ADR-0024）。
    ///
    /// 内网机器用这个：只出不进，不需要公网地址、不需要隧道、不需要 VPN；
    /// 连上即注册，断开即注销。
    ///
    /// 不加则走老路（平台按 `--advertise` 反过来敲节点）—— 同内网部署时那条更简单。
    /// **两条路不自动切换**：自动切换会让"到底走的哪条"变成运行时才知道的事
    #[arg(long)]
    pub link: bool,
}

pub async fn handle(args: ServeArgs) -> Result<()> {
    let root = args.root.unwrap_or_else(|| {
        dirs::home_dir().unwrap_or_else(std::env::temp_dir).join(".tke").join("serve")
    });
    let token = args.token.or_else(|| std::env::var("TKE_SERVE_TOKEN").ok()).filter(|t| !t.is_empty());

    // 平台对接：地址与凭据缺一不可，只给一个多半是配漏了——直接说出来，
    // 而不是"静默不报到"（那会让人对着平台上空空的节点列表查半天）
    let platform_token = args
        .platform_token
        .or_else(|| std::env::var("TKE_PLATFORM_TOKEN").ok())
        .filter(|t| !t.is_empty());
    let platform = match (args.platform.as_deref(), platform_token) {
        (Some(base), Some(token)) => Some(tke::serve::heartbeat::PlatformLink {
            base: base.to_string(),
            token,
            name: args.node_name.unwrap_or_else(|| {
                std::env::var("HOSTNAME")
                    .ok()
                    .filter(|h| !h.is_empty())
                    .unwrap_or_else(|| "tke-node".to_string())
            }),
            advertise: args.advertise,
        }),
        (Some(_), None) => {
            return Err(TkeError::InvalidArgument(
                "给了 --platform 但没有凭据：加 --platform-token 或设环境变量 TKE_PLATFORM_TOKEN".into(),
            ))
        }
        (None, Some(_)) => {
            return Err(TkeError::InvalidArgument(
                "给了平台凭据但没有 --platform：不知道该报到哪儿".into(),
            ))
        }
        (None, None) => None,
    };

    tke::serve::run(tke::serve::ServeOptions {
        bind: args.bind,
        port: args.port,
        token,
        root,
        session_ttl: Duration::from_secs(args.ttl),
        exec_timeout: Duration::from_secs(args.exec_timeout),
        web_slots: args.web_slots,
        fake_devices: args.fake_device,
        platform,
        link: args.link,
        max_upload_bytes: args.max_upload_mb * 1024 * 1024,
    })
    .await
}
