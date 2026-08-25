// 【客户端 argv 翻译】远程模式下，同一条命令行怎么变成一次 HTTP 调用。
//
// 这是 ADR-0022 D4「文档不分叉」能不能成立的关键：本地怎么敲，远程就怎么敲。
// 所以 `-d` 和 `--log` **不是简单丢掉，而是翻译**：
//   `-d web` / `-d android` / `-d <设备id>` → 租哪台设备（服务端会把它注入回子进程）
//   `--log <目录>`                          → 产物往哪拉（服务端自己管落点，拉回来放这儿）
// 其余由服务端接管的参数（--cache/--current-dir/--json/--copilot…）就地吃掉并说一声，
// 不能让人以为它生效了（INV-9：被环境跳过的东西必须可见）。
//
// 为什么要自己扫 argv 而不是先过 clap：拦截发生在 clap 之前——远程要原样转发命令，
// 让 clap 先解析一遍等于要求客户端和节点的版本严格一致，那正是我们想避免的耦合。

use std::path::PathBuf;

/// 会吃掉下一个 token 当值的全局参数（必须认全，否则 `-d web fetch` 里的 `web` 会被当成命令）
const GLOBAL_VALUE_FLAGS: &[&str] =
    &["-d", "--device", "--log", "--cache", "--current-dir", "--scripts", "-c", "--config"];

#[derive(Debug, Default, PartialEq)]
pub struct ClientInvocation {
    /// 子命令（第一个非旗标 token）；None = 光有全局参数（如 `tke --help`）
    pub command: Option<String>,
    /// 要发给节点的 argv（已剔除服务端接管的那些）
    pub argv: Vec<String>,
    /// `-d` 的值：设备 id 或平台名
    pub device: Option<String>,
    /// `--log` 的值：产物拉回本地放哪
    pub log: Option<PathBuf>,
    /// 就地吃掉的参数——**要说出来**，别让人以为它生效了
    pub dropped: Vec<String>,
}

/// 本地就该拒的：说清楚为什么，别等发到节点再被白名单拒一次
#[derive(Debug, PartialEq)]
pub struct ClientReject(pub String);

pub fn parse(args: &[String]) -> Result<ClientInvocation, ClientReject> {
    let mut inv = ClientInvocation::default();
    let mut i = 0;
    while i < args.len() {
        let tok = &args[i];
        let (name, inline) = match tok.split_once('=') {
            Some((k, v)) if k.starts_with('-') => (k.to_string(), Some(v.to_string())),
            _ => (tok.clone(), None),
        };

        // 取值型全局参数：值可能在 `=` 后面，也可能是下一个 token
        if GLOBAL_VALUE_FLAGS.contains(&name.as_str()) {
            let val = match inline {
                Some(v) => v,
                None => {
                    i += 1;
                    match args.get(i) {
                        Some(v) => v.clone(),
                        None => return Err(ClientReject(format!("`{name}` 后面缺一个值。"))),
                    }
                }
            };
            match name.as_str() {
                "-d" | "--device" => inv.device = Some(val),
                "--log" => {
                    // 必须是相对路径：绝对路径既跳不出会话工作区（会被沙箱拒），
                    // 也没法在两边表示同一个位置
                    if PathBuf::from(&val).is_absolute() {
                        return Err(ClientReject(format!(
                            "远程模式的 `--log` 要给相对路径（收到 `{val}`）：\n                             节点按会话工作区解析它，跑完再按同一个相对路径拉回本地。"
                        )));
                    }
                    inv.log = Some(PathBuf::from(&val));
                    inv.argv.push("--log".into());
                    inv.argv.push(val);
                }
                "-c" | "--config" => {
                    // 配置是节点的事：让客户端指定它，等于让租户改节点行为
                    return Err(ClientReject(
                        "远程模式不接受 `--config`：节点用节点自己的配置。".into(),
                    ));
                }
                other => inv.dropped.push(format!("{other}（服务端按会话决定落点）")),
            }
            i += 1;
            continue;
        }

        // 无值型：服务端接管的就地吃掉
        match name.as_str() {
            "--json" => inv.dropped.push("--json（远程一律 JSON）".into()),
            "-v" | "--verbose" => inv.dropped.push("--verbose（节点侧日志看节点）".into()),
            "--copilot" => {
                // 裸 `--copilot` 后面可能跟 true/false，跟着的话一起吃掉
                if inline.is_none() && matches!(args.get(i + 1).map(|s| s.as_str()), Some("true") | Some("false")) {
                    i += 1;
                }
                inv.dropped.push("--copilot（命令层是零 LLM 面，见 INV-16）".into());
            }
            "--headless" => inv.dropped.push("--headless（服务器上没有显示器，节点强制无头）".into()),
            "--prompts-dir" => {
                return Err(ClientReject("远程模式不接受 `--prompts-dir`：提示词是节点的事。".into()))
            }
            _ => {
                if inv.command.is_none() && !tok.starts_with('-') {
                    inv.command = Some(tok.clone());
                }
                inv.argv.push(tok.clone());
            }
        }
        i += 1;
    }
    Ok(inv)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(a: &[&str]) -> Result<ClientInvocation, ClientReject> {
        parse(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn 设备参数被翻译成租哪台而不是转发() {
        let inv = p(&["-d", "web", "fetch", "--interactive"]).unwrap();
        assert_eq!(inv.device.as_deref(), Some("web"));
        assert_eq!(inv.command.as_deref(), Some("fetch"), "`web` 是 -d 的值，不是命令");
        assert_eq!(inv.argv, vec!["fetch", "--interactive"], "-d 由服务端注入，不转发");
    }

    #[test]
    fn 等号形态与分开形态等价() {
        assert_eq!(p(&["--device=android", "refresh"]).unwrap().device.as_deref(), Some("android"));
        assert_eq!(p(&["-d", "android", "refresh"]).unwrap().device.as_deref(), Some("android"));
    }

    #[test]
    fn log既转发也记下来拉回哪里() {
        // 这条是"文档不分叉"的关键：`--log logs/scan` 在两边是**同一个相对路径**，
        // 所以后面那条 `tke report logs/scan` 照样对得上
        let inv = p(&["steps", "等待 [1s]", "--log", "logs/scan"]).unwrap();
        assert_eq!(inv.log, Some(PathBuf::from("logs/scan")));
        assert_eq!(inv.argv, vec!["steps", "等待 [1s]", "--log", "logs/scan"], "要转发给节点");
        assert_eq!(inv.command.as_deref(), Some("steps"));
    }

    #[test]
    fn 绝对路径的log当场拒绝() {
        let e = p(&["steps", "等待 [1s]", "--log", "/tmp/out"]).unwrap_err();
        assert!(e.0.contains("相对路径"), "{}", e.0);
    }

    #[test]
    fn 服务端接管的参数就地吃掉但要说一声() {
        let inv = p(&["--json", "--copilot", "false", "--cache", "/tmp/x", "fetch"]).unwrap();
        assert_eq!(inv.argv, vec!["fetch"]);
        assert_eq!(inv.dropped.len(), 3, "吃掉三个：{:?}", inv.dropped);
        assert!(inv.dropped.iter().any(|d| d.contains("零 LLM")), "{:?}", inv.dropped);
        // 裸 --copilot 后面的 false 也要一起吃掉，否则它会变成命令
        assert_eq!(inv.command.as_deref(), Some("fetch"));
    }

    #[test]
    fn 属于节点的东西当场拒绝并说清楚() {
        for (args, want) in [
            (vec!["--config", "x.toml", "fetch"], "节点自己的配置"),
            (vec!["--prompts-dir", "p", "fetch"], "提示词"),
        ] {
            let e = p(&args).unwrap_err();
            assert!(e.0.contains(want), "{args:?}: {}", e.0);
        }
        assert!(p(&["-d"]).is_err(), "缺值要报错，不能静默");
    }

    #[test]
    fn 没有子命令时不认领() {
        // `tke --help` 这类要留给本地 clap 处理
        assert_eq!(p(&["--help"]).unwrap().command, None);
        assert_eq!(p(&[]).unwrap().command, None);
    }

    #[test]
    fn 命令自己的参数原样转发() {
        let inv = p(&["control", "click", "--at", "100,200"]).unwrap();
        assert_eq!(inv.argv, vec!["control", "click", "--at", "100,200"]);
        assert!(inv.dropped.is_empty());
    }
}
