// 【远程白名单】INV-16 的执行点：远程只跑枚举出来的命令，参数逐个过。
//
// 这是 serve 的**安全承重墙**——白名单之外没有第二条路（承 ADR-0016 删 CLI 直通的精神：
// 当初删它就是因为它是「点得中但没证据」的旁路，服务化不许把它以 HTTP 的形式重新引进来）。
// 三道关：
//   ① 命令白名单（含子命令粒度）。`harness` / `security` **不在其中**——它们是 L2 的活，
//      走 L1 就是拿平台的 key 白跑，计费模型当场塌（INV-16 延伸条款，ADR-0022 D3）。
//      `update` / `uninstall` / `doctor --fix` 也不在——联网下载和自我替换是节点运维的事。
//   ② 禁用旗标。会改落点/配置/AI 开关的一律拒，这些由服务端注入（`exec.rs`）。
//   ③ 宿主路径参数表。取宿主路径的参数必须是相对路径，由 `exec.rs` 解析进会话工作区——
//      `tke ocr --image /etc/passwd` 就是靠这张表挡住的。
//
// **没有"绝对路径一律拒"的兜底规则**，这是有意的：`app` 的包名、`http`/`recon` 的 URL、
// `control input` 的文本都可能长得像路径，一刀切会制造大量假阳。防线是"表 + 守卫"而不是"猜"：
// `scripts/check-serve-paths.sh` 盯着 `src/cli/**` 里所有 `PathBuf` 参数，漏登记就报红。
//
// ⚠️ 加了新命令 / 新的取路径参数 → 更新这张表，否则守卫会拦下。

/// 一条命令的远程契约
struct CmdSpec {
    /// 顶层命令名
    name: &'static str,
    /// 允许的子命令；`None` = 不限制子命令（clap 自己会拒绝不存在的）
    subs: Option<&'static [&'static str]>,
    /// 取**宿主**路径的旗标：其值必须相对，且会被解析进会话工作区
    path_flags: &'static [&'static str],
    /// 路径是不是"紧跟命令名的第一个参数"（`run <脚本>` / `report <目录>`）。
    /// 只认这一个位置是有意的：不知道每个旗标吃不吃值就数不准位置参数，
    /// 与其猜，不如要求调用方把路径放在最前面——错了会给一句明确的话
    path_first_arg: bool,
    /// 该命令专属的禁用旗标（全局那批见 `BANNED_FLAGS`）
    banned: &'static [&'static str],
}

/// 白名单本体。**新增命令必须在这里登记**
const ALLOWED: &[CmdSpec] = &[
    // ===== 设备原语 =====
    // `browser-download --dir` 是宿主路径：沙箱进会话工作区后，下载的文件正好能从产物接口取回
    CmdSpec { name: "control",   subs: None, path_flags: &["--dir"], path_first_arg: false, banned: &[] },
    CmdSpec { name: "refresh",   subs: None, path_flags: &["--out"], path_first_arg: false, banned: &[] },
    CmdSpec { name: "fetch",     subs: None, path_flags: &[], path_first_arg: false, banned: &[] },
    CmdSpec { name: "recognize", subs: None, path_flags: &["--lib"], path_first_arg: false, banned: &[] },
    // ===== 安全原语 =====
    CmdSpec { name: "http",      subs: None, path_flags: &[], path_first_arg: false, banned: &[] },
    CmdSpec { name: "recon",     subs: None, path_flags: &[], path_first_arg: false, banned: &[] },
    // ===== 自有工具 =====
    CmdSpec { name: "device",    subs: None, path_flags: &[], path_first_arg: false, banned: &[] },
    CmdSpec { name: "app",       subs: None, path_flags: &[], path_first_arg: false, banned: &[] },
    CmdSpec { name: "element",   subs: None, path_flags: &["--lib"], path_first_arg: false, banned: &[] },
    CmdSpec { name: "ocr",       subs: None, path_flags: &["--image", "-i"], path_first_arg: false, banned: &[] },
    // ===== 脚本 =====
    CmdSpec { name: "steps",     subs: None, path_flags: &[], path_first_arg: false, banned: &[] },
    CmdSpec { name: "run",       subs: None, path_flags: &[], path_first_arg: true,  banned: &[] },
    // ===== 共享生命周期（ADR-0021）=====
    CmdSpec { name: "task",      subs: Some(&["new"]), path_flags: &["--dir"], path_first_arg: false, banned: &[] },
    CmdSpec { name: "report",    subs: None, path_flags: &["--summary-file"], path_first_arg: true, banned: &[] },
    // ===== 环境（只读）=====
    // `--fix`/`-y` 是唯一会联网下载几百 MB 的路径（ADR-0012），那是节点运维的事，不给租户碰
    CmdSpec { name: "doctor",    subs: None, path_flags: &[], path_first_arg: false,
              banned: &["--fix", "-y", "--yes", "--base-url", "--check"] },
];

/// **所有命令通用**的宿主路径参数。`--log` 不禁用而是沙箱化，是有意的：
/// 本地写 `--log logs/scan` 再 `tke report logs/scan`，远程必须是**同一个相对路径**才能对上
/// ——把它吃掉的话证据落进会话默认目录，后面那条 report 就找不着了（实跑安全轨时撞出来的）。
const GLOBAL_PATH_FLAGS: &[&str] = &["--log"];

/// 全局禁用旗标：改落点 / 改配置 / 开 AI / 改设备的，一律由服务端注入，不接受调用方指定。
/// `--headless` 也在内——服务器上开有头窗口必然失败，而且会毁掉正在复用的会话。
const BANNED_FLAGS: &[&str] = &[
    "--config", "-c",
    "--prompts-dir",
    "--cache", "--current-dir", "--scripts",
    "--json",
    "--device", "-d",
    "--copilot",
    "--headless",
];

/// 过关后的 argv：已把 `--k=v` 拆成两个 token（下游只需处理一种形态），
/// 并标出哪些下标的值是**宿主路径**（由 `exec.rs` 解析进工作区）
#[derive(Debug, PartialEq)]
pub struct Validated {
    pub argv: Vec<String>,
    pub host_path_idx: Vec<usize>,
}

/// 被拒的原因——**直接回给调用方**，所以每句都要说清楚该怎么改
#[derive(Debug, PartialEq)]
pub struct Rejected(pub String);

impl std::fmt::Display for Rejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 旗标名归一：`--lib=foo` → `--lib`
fn flag_name(tok: &str) -> &str {
    tok.split_once('=').map(|(k, _)| k).unwrap_or(tok)
}

fn spec_of(cmd: &str) -> Option<&'static CmdSpec> {
    ALLOWED.iter().find(|s| s.name == cmd)
}

/// 白名单里都有哪些命令（`/v1/hello` 回给调用方，省得靠猜）
pub fn allowed_commands() -> Vec<&'static str> {
    ALLOWED.iter().map(|s| s.name).collect()
}

/// 三道关：命令白名单 → 禁用旗标 → 宿主路径标记
pub fn validate(argv: &[String]) -> Result<Validated, Rejected> {
    let cmd = match argv.first() {
        Some(c) if !c.is_empty() => c.as_str(),
        _ => return Err(Rejected("argv 是空的：第一个元素必须是 tke 的子命令。".into())),
    };

    let spec = match spec_of(cmd) {
        Some(s) => s,
        None => {
            // 说清楚"为什么这条不行"，别让调用方以为是拼错了
            let why = match cmd {
                "harness" | "harn" | "security" =>
                    "它带 AI 编排，属于任务层（L2）；命令层不跑服务端 AI（ADR-0022 D3）。请改用任务接口。",
                "update" | "uninstall" | "fix" =>
                    "联网下载与自我替换是节点运维的事，不对远程开放。",
                "serve" => "serve 不能再起一个 serve。",
                _ => "不在远程白名单里。",
            };
            return Err(Rejected(format!(
                "命令 `{cmd}` 不可远程执行：{why}\n可用的命令：{}",
                allowed_commands().join(" / ")
            )));
        }
    };

    // ① 子命令粒度
    if let Some(subs) = spec.subs {
        let sub = argv.get(1).map(|s| s.as_str()).unwrap_or("");
        if !subs.contains(&sub) {
            return Err(Rejected(format!(
                "`{cmd}` 远程只开放子命令：{}（收到 `{}`）。",
                subs.join(" / "),
                if sub.is_empty() { "(空)" } else { sub }
            )));
        }
    }

    // ② + ③ 逐个 token 过
    let mut out: Vec<String> = Vec::with_capacity(argv.len() + 2);
    let mut host_path_idx = Vec::new();
    let mut expect_path_value = false;
    let mut seen_positional = 0usize;
    // 有子命令的，子命令本身不算位置参数
    let skip_positions = if spec.subs.is_some() { 1 } else { 0 };

    for (i, tok) in argv.iter().enumerate() {
        if i == 0 {
            out.push(tok.clone());
            continue;
        }

        if tok == "--" {
            return Err(Rejected("远程不接受 `--` 分隔符（它会把后面的东西整段透传）。".into()));
        }

        if tok.starts_with('-') && tok.len() > 1 {
            let name = flag_name(tok);
            if BANNED_FLAGS.contains(&name) {
                return Err(Rejected(format!(
                    "参数 `{name}` 不接受远程指定：落点/设备/AI 开关由服务端按会话注入。"
                )));
            }
            if spec.banned.contains(&name) {
                return Err(Rejected(format!("`{cmd} {name}` 不对远程开放。")));
            }
            let is_path_flag = spec.path_flags.contains(&name) || GLOBAL_PATH_FLAGS.contains(&&*name);
            match tok.split_once('=') {
                // `--lib=foo` 拆成两个 token，下游只处理一种形态
                Some((k, v)) => {
                    out.push(k.to_string());
                    out.push(v.to_string());
                    if is_path_flag {
                        host_path_idx.push(out.len() - 1);
                    }
                }
                None => {
                    out.push(tok.clone());
                    expect_path_value = is_path_flag;
                }
            }
            continue;
        }

        // 位置参数（或某个旗标的值）
        if expect_path_value {
            host_path_idx.push(out.len());
            expect_path_value = false;
        } else if spec.path_first_arg && seen_positional == skip_positions {
            // `run <脚本>` / `report <目录>`：路径必须紧跟命令名
            host_path_idx.push(out.len());
        }
        seen_positional += 1;
        out.push(tok.clone());
    }

    if expect_path_value {
        return Err(Rejected(format!("`{}` 后面缺一个路径。", out.last().cloned().unwrap_or_default())));
    }
    if spec.path_first_arg && seen_positional <= skip_positions {
        return Err(Rejected(format!(
            "`{cmd}` 的路径参数必须**紧跟在命令后**（如 `{cmd} logs/xxx`）——远程按位置认它。"
        )));
    }

    Ok(Validated { argv: out, host_path_idx })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(args: &[&str]) -> Result<Validated, Rejected> {
        validate(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn 放行普通设备命令() {
        let r = v(&["control", "click", "--at", "100,200"]).unwrap();
        assert_eq!(r.argv, vec!["control", "click", "--at", "100,200"]);
        assert!(r.host_path_idx.is_empty());
    }

    #[test]
    fn ai编排命令一律拒并说清为什么() {
        for cmd in ["harness", "harn", "security"] {
            let e = v(&[cmd, "随便什么"]).unwrap_err();
            assert!(e.0.contains("任务层"), "{cmd} 的拒绝理由要指向 L2：{}", e.0);
        }
    }

    #[test]
    fn 联网下载与自我替换不开放() {
        for cmd in ["update", "uninstall", "fix"] {
            assert!(v(&[cmd]).is_err(), "{cmd} 不该放行");
        }
        // doctor 本体只读，放行；--fix 是同一命令里唯一会联网下的那条路，拦住
        assert!(v(&["doctor"]).is_ok());
        assert!(v(&["doctor", "--fix"]).is_err());
        assert!(v(&["doctor", "-y"]).is_err());
    }

    #[test]
    fn 落点与设备参数不接受远程指定() {
        for bad in ["--cache", "--current-dir", "--config", "--json", "--copilot", "--headless"] {
            let e = v(&["fetch", bad, "x"]).unwrap_err();
            assert!(e.0.contains("服务端"), "{bad}: {}", e.0);
        }
        // 短横线形式与 `=` 形式都要拦住（曾经最容易漏的两种写法）
        assert!(v(&["fetch", "-d", "web:1"]).is_err());
        assert!(v(&["fetch", "--headless=off"]).is_err());
    }

    #[test]
    fn log是所有命令通用的沙箱路径参数() {
        // 本地 `--log logs/scan` → `tke report logs/scan`；远程必须是同一个相对路径才对得上
        let r = v(&["recon", "headers", "https://x", "--log", "logs/scan"]).unwrap();
        assert_eq!(r.host_path_idx, vec![4], "--log 的值要被沙箱进会话工作区");
        assert!(r.argv.contains(&"--log".to_string()), "不能吃掉它——下游要靠它对上目录");
        let r = v(&["steps", "等待 [1s]", "--log=out"]).unwrap();
        assert_eq!(r.argv.last().unwrap(), "out");
    }

    #[test]
    fn 宿主路径参数被标出来() {
        let r = v(&["ocr", "--image", "shot.png"]).unwrap();
        assert_eq!(r.host_path_idx, vec![2], "--image 的值要被标成宿主路径");
        // `=` 形态会被拆开，标的是拆出来那个值
        let r = v(&["recognize", "--lib=foo.tklib", "--name", "登录"]).unwrap();
        assert_eq!(r.argv, vec!["recognize", "--lib", "foo.tklib", "--name", "登录"]);
        assert_eq!(r.host_path_idx, vec![2]);
    }

    #[test]
    fn 守卫抓出来的那两个宿主路径参数() {
        // 这两条是 `check-serve-paths.sh` 的思路刚成形就逼出来的真洞：
        // 命令本身在白名单里，但它的取路径参数不在表里 → 能写/读工作区外
        let r = v(&["refresh", "--out", "crop.png"]).unwrap();
        assert_eq!(r.host_path_idx, vec![2], "refresh --out 是宿主路径");
        let r = v(&["control", "browser-download", "--dir", "dl"]).unwrap();
        assert_eq!(r.host_path_idx, vec![3], "browser-download --dir 是宿主路径");
    }

    #[test]
    fn 首位路径参数按位置认() {
        let r = v(&["run", "cases/login.tks"]).unwrap();
        assert_eq!(r.host_path_idx, vec![1]);
        let r = v(&["report", "logs/task1", "--verdict", "pass"]).unwrap();
        assert_eq!(r.host_path_idx, vec![1]);
        // 没给路径 / 路径没放最前面 → 明确报错，而不是悄悄放过去
        assert!(v(&["run"]).is_err());
        assert!(v(&["run", "--copilot"]).is_err());
    }

    #[test]
    fn 子命令粒度按表限制() {
        assert!(v(&["task", "new", "--kind", "ui"]).is_ok());
        // task 只开放 new：别的子命令（将来若有）不该跟着自动开放
        let e = v(&["task", "list"]).unwrap_err();
        assert!(e.0.contains("new"), "{}", e.0);
        assert!(v(&["task"]).is_err());
    }

    #[test]
    fn 拒绝分隔符透传() {
        let e = v(&["control", "--", "rm", "-rf", "/"]).unwrap_err();
        assert!(e.0.contains("--"), "{}", e.0);
    }

    #[test]
    fn 空的与不存在的命令() {
        assert!(v(&[]).is_err());
        assert!(v(&[""]).is_err());
        let e = v(&["rm"]).unwrap_err();
        assert!(e.0.contains("可用的命令"), "拒绝时要顺手告诉他有哪些能用：{}", e.0);
    }
}
