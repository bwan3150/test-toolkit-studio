//! `tke remote` —— 远程会话的显式管理（隐式那条路见 `tke::remote::maybe_dispatch`）。
//!
//! 平时不用敲它：配好 `TKE_REMOTE` 后第一条命令会自动租一台。
//! 但"我现在连着谁、租着哪台、还剩多久"必须问得出来——**查得出、说得清**（INV-12 的精神）。

use std::path::PathBuf;

use tke::remote::{state, RemoteConfig};
use tke::{JsonOutput, Result, TkeError};

#[derive(clap::Subcommand)]
pub enum RemoteCommands {
    /// 连着谁、租着哪台、还剩多久、版本对不对得上
    Status,
    /// 显式租一台（不敲也行：第一条命令会自动租）
    Open {
        /// 平台（web/android/ios）或点名一台设备
        #[arg(short, long)]
        device: Option<String>,
    },
    /// 还回去：释放租约（节点会复位设备）
    Close {
        /// 释放前把产物拉到这里
        #[arg(long)]
        into: Option<PathBuf>,
    },
    /// 把这次会话的产物拉回本地
    Pull {
        #[arg(long, default_value = ".")]
        into: PathBuf,
    },
    /// 节点上有哪些设备、谁在租
    Devices,
    /// 往会话工作区传文件（APK/IPA、.tks+.tklib 两件套）
    Push {
        /// 本地文件
        file: PathBuf,
        /// 放到工作区的哪个相对路径（默认同名）
        #[arg(long)]
        r#as: Option<String>,
    },
}

fn cfg() -> Result<RemoteConfig> {
    RemoteConfig::from_env().ok_or_else(|| {
        TkeError::InvalidArgument(
            "没配远程节点：先 `export TKE_REMOTE=http://<节点>:8787`（要凭据再加 TKE_TOKEN）。".into(),
        )
    })
}

pub async fn handle(cmd: RemoteCommands) -> Result<()> {
    let cfg = cfg()?;
    let c = cfg.client();
    let err = |e: String| TkeError::NetworkError(e);

    match cmd {
        RemoteCommands::Status => {
            let hello = c.hello().map_err(err)?;
            let sess = state::load(&cfg.base);
            JsonOutput::print(serde_json::json!({
                "success": true,
                "node": cfg.base,
                "node_version": hello["tke_version"],
                "local_version": tke::version_line(),
                // 对不上就说出来：沉默会让人得出"没改善"的假结论（Q-11）。
                // **两边都用 version_line()**：节点报的是"版本号 (构建号)"，
                // 只拿 BUILD_VERSION 去比，同一份二进制也会判成不一致。
                "version_match": hello["tke_version"].as_str() == Some(tke::version_line().as_str()),
                "host_os": hello["host_os"],
                "session": sess.as_ref().map(|s| serde_json::json!({
                    "session_id": s.session_id, "device": s.device_id,
                    "label": s.device_label, "expires_at": s.expires_at,
                    "pulled_files": s.pulled.len(),
                })),
            }));
            Ok(())
        }
        RemoteCommands::Open { device } => {
            let s = tke::remote::open_session(&c, device.as_deref()).map_err(err)?;
            JsonOutput::print(serde_json::json!({
                "success": true, "session_id": s.session_id,
                "device": s.device_id, "label": s.device_label, "expires_at": s.expires_at,
            }));
            Ok(())
        }
        RemoteCommands::Close { into } => {
            let Some(mut s) = state::load(&cfg.base) else {
                return Err(TkeError::InvalidArgument("现在没有租着的会话。".into()));
            };
            // 先拉产物再释放：释放之后节点可以回收目录，那时候再拉就晚了
            if let Some(dir) = into {
                let n = c.pull_new(&mut s, &dir, "").map_err(err)?;
                eprintln!("📥 拉回 {n} 个产物 → {}", dir.display());
            }
            let r = c.release(&s.session_id).map_err(err)?;
            state::clear(&cfg.base);
            JsonOutput::print(serde_json::json!({"success": true, "released": s.session_id, "reset": r["reset"]}));
            Ok(())
        }
        RemoteCommands::Pull { into } => {
            let Some(mut s) = state::load(&cfg.base) else {
                return Err(TkeError::InvalidArgument("现在没有租着的会话。".into()));
            };
            let n = c.pull_new(&mut s, &into, "").map_err(err)?;
            let _ = state::save(&s);
            JsonOutput::print(serde_json::json!({"success": true, "pulled": n, "into": into.to_string_lossy()}));
            Ok(())
        }
        RemoteCommands::Devices => {
            JsonOutput::print(c.devices().map_err(err)?);
            Ok(())
        }
        RemoteCommands::Push { file, r#as } => {
            let bytes = std::fs::read(&file)?;
            let name = r#as.unwrap_or_else(|| {
                file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "upload.bin".into())
            });
            let s = tke::remote::ensure_session(&c, None).map_err(err)?;
            let v = c.put_workspace(&s.session_id, &name, bytes).map_err(err)?;
            JsonOutput::print(v);
            Ok(())
        }
    }
}
