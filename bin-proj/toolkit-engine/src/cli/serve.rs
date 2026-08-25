//! `tke serve` —— 把这台机器的能力开成 HTTP 接口（ADR-0022 P1）。
//!
//! 参数翻译层，业务在 `tke::serve`（INV-10：cli 不放业务逻辑）。

use std::path::PathBuf;
use std::time::Duration;

use tke::Result;

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
}

pub async fn handle(args: ServeArgs) -> Result<()> {
    let root = args.root.unwrap_or_else(|| {
        dirs::home_dir().unwrap_or_else(std::env::temp_dir).join(".tke").join("serve")
    });
    let token = args.token.or_else(|| std::env::var("TKE_SERVE_TOKEN").ok()).filter(|t| !t.is_empty());

    tke::serve::run(tke::serve::ServeOptions {
        bind: args.bind,
        port: args.port,
        token,
        root,
        session_ttl: Duration::from_secs(args.ttl),
        exec_timeout: Duration::from_secs(args.exec_timeout),
        web_slots: args.web_slots,
        fake_devices: args.fake_device,
        max_upload_bytes: args.max_upload_mb * 1024 * 1024,
    })
    .await
}
