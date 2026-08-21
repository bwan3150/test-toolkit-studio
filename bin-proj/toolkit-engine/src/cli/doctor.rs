// 【体检报告的排版】`tke doctor` 打出来的那张表——**只负责组织与呈现**，
// 依赖检测与下载留在 `fix.rs`。
//
// 为什么单独一层：早先所有信息挤在一处顺序打印，写的时候是"想到什么加一行"，
// 于是版本、设备、路径、提示混在一起，用户的原话是「信息太多而且顺序混乱看起来很难受」。
// 一份体检报告要能一眼扫完，靠的是**分组**和**对齐**，不是把该说的都说了。
//
// 三段，固定顺序（每段回答一个问题）：
//   1. 这套工具本身怎么样  —— 平台 / 依赖 / Engine 版本 / Skill 版本
//   2. 能测什么            —— 四端 + 显示器环境（真机在前、模拟器在后）
//   3. 东西落在哪          —— Engine / Skill / 日志
// 最后一行才是结论（上面每行都只是一项检查，见 INV-9：失败必须可见）。
//
// 标签列按**显示宽度**对齐（`utils::text::pad_right`）：中文在等宽终端占两格，
// `{:<14}` 按字符数填必然错位。

use std::path::{Path, PathBuf};

use tke::tools::discover::{discover_with, Discovery};
use tke::utils::text::{disp_width, pad_right};
use tke::utils::update::Staleness;

// ── 外观 ──（与 install.sh / install.ps1 同一套：符号 + 颜色，**不用 emoji**——
// 等宽终端里对不齐，SSH/CI 日志里还常变成方块）
pub fn tty() -> bool {
    std::io::IsTerminal::is_terminal(&std::io::stdout())
}
pub fn c(code: &str) -> String {
    if tty() { format!("\x1b[{}m", code) } else { String::new() }
}
pub fn sym_ok() -> String { format!("{}✓{}", c("38;5;42"), c("0")) }
pub fn sym_warn() -> String { format!("{}!{}", c("38;5;214"), c("0")) }
pub fn sym_err() -> String { format!("{}✗{}", c("38;5;203"), c("0")) }
pub fn sym_dot() -> String { format!("{}·{}", c("38;5;245"), c("0")) }
pub fn dim(s: &str) -> String { format!("{}{}{}", c("38;5;245"), s, c("0")) }
pub fn section(title: &str) {
    println!("\n{}{}▸ {}{}", c("1"), c("38;5;39"), title, c("0"));
}

/// 标签列宽度：段二最长的 `Android模拟器`（7 + 3×2 = 13）再留两格。
/// 写死而不是动态求最大值，是为了**三段共用同一列**——分段算各自的最大宽度，
/// 三段就会各自对齐到不同位置，整张表反而更乱
const LABEL_W: usize = 15;

/// 一行的语气。**上色是例外，不是常态**——用户的话：「不要绿色泛滥」。
///
/// 一张体检表大多数时候通篇都是好的，每行都染绿等于没染：眼睛扫过去，
/// 真正要人动手的那一两行反而淹了。所以正常状态一律不上色，
/// 只有**需要人做点什么**的行才有颜色。
#[derive(Clone, Copy, PartialEq)]
pub enum Tone {
    /// 正常（就绪、可用、已是最新）与纯事实（平台、路径）——**不上色**
    Plain,
    /// 有可用更新——这行是要人动手的，给它颜色
    Update,
    /// 缺依赖 / 装了但用不了——红
    Bad,
    /// 没有 / 查不了 / 不支持——灰，一眼跳过
    Muted,
}

/// 一行：`  标签      值 (补充)`。
///
/// 标签一律 dim、值按语气上色、补充永远 dim——**队形靠这三条固定下来**。
/// 补充不跟着值一起上色是有意的：一行里两段亮色，扫下来就分不出哪个是状态了
fn row(label: &str, value: &str, note: &str, tone: Tone) {
    let value = match tone {
        Tone::Update => format!("{}{}{}", c("38;5;42"), value, c("0")),
        Tone::Bad => format!("{}{}{}", c("38;5;203"), value, c("0")),
        Tone::Muted => dim(value),
        Tone::Plain => value.to_string(),
    };
    let text = if note.is_empty() {
        value
    } else {
        format!("{} {}", value, dim(&format!("({})", note)))
    };
    println!("  {}{}", dim(&pad_right(label, LABEL_W)), text);
}

/// 一项依赖的现状（fix.rs 检测出来，这里只管怎么说）
pub struct Dep {
    pub name: &'static str,
    pub what: &'static str,
    pub size: &'static str,
    pub is_chrome: bool,
    pub present: bool,
}

/// 体检采到的全部事实。**采集与打印分开**：`--fix` 要在下载前后各用一次，
/// 而联网查版本这种事一次就够
pub struct Health {
    pub st: Option<Staleness>,
    pub disc: Discovery,
    /// 新终端里还能不能直接敲 tke；None = 无从判断（Windows 的 PATH 在注册表）
    pub path_persisted: Option<bool>,
    pub exe_dir: PathBuf,
    pub platform: String,
    pub profile: String,
}

impl Health {
    /// 采集。**会联网**（只读几十字节的 VERSION，不下载依赖，见 ADR-0012）：
    /// `max_age=0` = 人特地跑 doctor 来问，就不吃缓存，给他最新的答案
    pub fn probe(exe_dir: &Path, platform: &str, profile: &str) -> Self {
        Self {
            st: tke::utils::update::check(0),
            // `all=true`：关着的模拟器也要数进去——「有 24 台但都没启动」和
            // 「一台都没有」是两回事，只报能用的那种会让人以为机器不支持
            disc: discover_with(true),
            path_persisted: path_persisted(exe_dir),
            exe_dir: exe_dir.to_path_buf(),
            platform: platform.to_string(),
            profile: profile.to_string(),
        }
    }

    /// 三段正文。缺失依赖的明细跟在「依赖」那行下面——它是那一项的展开，不是新的一段
    pub fn print(&self, deps: &[Dep]) {
        self.print_basics(deps);
        println!();
        self.print_targets();
        println!();
        self.print_paths();
    }

    // ── 段一：这套工具本身 ──
    fn print_basics(&self, deps: &[Dep]) {
        row("平台", &friendly_os(), &self.platform, Tone::Plain);

        let missing: Vec<&Dep> = deps.iter().filter(|d| !d.present).collect();
        let scope = if self.profile == "all" { String::new() } else { format!("仅 {}", self.profile) };
        if deps.is_empty() {
            row("依赖", "无需补齐", "此机型不支持该平台", Tone::Muted);
        } else if missing.is_empty() {
            row("依赖", &format!("{} 项已就绪", deps.len()), &scope, Tone::Plain);
        } else {
            row("依赖", &format!("缺 {} 项 / 共 {} 项", missing.len(), deps.len()), &scope, Tone::Bad);
            for m in &missing {
                println!(
                    "  {}{} {:<16} {}",
                    " ".repeat(LABEL_W),
                    sym_err(),
                    m.name,
                    dim(&format!("{} · 约 {}", m.what, m.size))
                );
            }
        }

        // 版本只报"一致/不一致"，不摆箭头暗示方向——本地可能是刚编出来的、比分发源还新
        let local = env!("BUILD_VERSION");
        match &self.st {
            None => row("Engine版本", local, "离线，未校验", Tone::Muted),
            Some(s) if s.tke_stale => {
                row("Engine版本", "有可用更新", &format!("{} → {}", local, s.remote.tke), Tone::Update)
            }
            Some(_) => row("Engine版本", "已是最新", local, Tone::Plain),
        }

        // skill 新鲜度比的是 **build 戳**而不是版本号：SKILL.md 天天改，
        // 而版本号只在 bump 时才动——只比版本号的话，用户抱着两天前的旧文档，
        // 体检照样说"一致"（Q-11 就是这么发生的）
        match &self.st {
            None => row("Skill版本", "未校验", "离线", Tone::Muted),
            Some(s) => match (&s.skill_dir, &s.local_skill_build) {
                (Some(_), Some(local_b)) if s.skill_stale => {
                    row("Skill版本", "有可用更新", &format!("{} → {}", local_b, s.remote.build), Tone::Update)
                }
                (Some(_), Some(local_b)) => row("Skill版本", "已是最新", local_b, Tone::Plain),
                // 老安装器装的没写版本文件——不当成过期（无从判断），但要说清为什么看不出来
                (Some(_), None) => row("Skill版本", "无版本信息", "由旧安装器安装", Tone::Muted),
                (None, _) => row("Skill版本", "未安装", "", Tone::Muted),
            },
        }
    }

    // ── 段二：能测什么 ──（真机在前、模拟器在后，四端顺序固定）
    fn print_targets(&self) {
        let want = |k: &str| {
            self.profile == "all"
                || match k {
                    "web" => self.profile == "web",
                    "android" => self.profile == "android",
                    _ => self.profile == "ios",
                }
        };
        // 安卓真机与安卓模拟器在 adb 眼里长得一样，靠 `emulator-` 前缀分开——
        // 两者都能操作（都是 adb），但"没插手机"和"没开模拟器"的下一步完全不同
        let androids = || self.disc.targets.iter().filter(|t| t.kind == "android");
        let n_emu = androids().filter(|t| t.id.starts_with("emulator-")).count();
        let n_real = androids().filter(|t| !t.id.starts_with("emulator-")).count();

        // **一个平台的真机与模拟器挨着**：人是按平台找这张表的（"我的安卓那边怎么样"），
        // 而不是按"先看所有真机再看所有模拟器"
        if want("android") {
            match (n_real, self.skip_why("android")) {
                (0, Some(w)) => row("Android真机", "未检测", &w, Tone::Muted),
                (0, None) => row("Android真机", "不可用", "尚未连接设备", Tone::Muted),
                (n, _) => row("Android真机", "可用", &format!("{} 台", n), Tone::Plain),
            }
            // **选装**（用户拍板 2026-08-21）：iOS 模拟器是 macOS 自带的，
            // 而这套要 1GB 上下（emulator 包 + 系统镜像），且安卓真机很好开——
            // 所以没装**不算环境不完整**，只如实说一句，不进「下一步」催人装
            let n_avd = self.disc.targets.iter().filter(|t| t.kind == "android-avd").count();
            match (n_emu, n_avd) {
                (0, 0) if tke::drivers::avd::emulator_bin().is_none() => {
                    row("Android模拟器", "未安装", "选装", Tone::Muted)
                }
                (0, 0) => row("Android模拟器", "不可用", "装了 SDK 但一个 AVD 都没建", Tone::Muted),
                (0, m) => row("Android模拟器", "待启动", &format!("{} 台可选", m), Tone::Muted),
                (n, 0) => row("Android模拟器", "可用", &format!("{} 台已启动", n), Tone::Plain),
                (n, m) => {
                    row("Android模拟器", "可用", &format!("{} 台已启动 · 另有 {} 台可选", n, m), Tone::Plain)
                }
            }
        }
        if want("ios") {
            // 宿主机做不了 iOS 时说"不可用"而不是"未检测"——那不是没查，是这台机器不行
            let n = self.disc.targets.iter().filter(|t| t.kind == "ios").count();
            if !tke::utils::capability::ios_supported() {
                row("iOS真机", "不可用", "需 macOS", Tone::Muted);
            } else {
                match (n, self.skip_why("ios")) {
                    (0, Some(w)) => row("iOS真机", "未检测", &w, Tone::Muted),
                    (0, None) => row("iOS真机", "不可用", "尚未连接设备", Tone::Muted),
                    (n, _) => row("iOS真机", "可用", &format!("{} 台", n), Tone::Plain),
                }
            }
            self.print_ios_sim();
        }
        if want("web") {
            let chrome = tke::utils::deps::chrome_for_testing_bin().is_some();
            let driver = tke::utils::deps::present_in(&self.exe_dir, "chromedriver");
            // 缺哪几样就都说出来：只报第一个，人补完还得再跑一次才知道另一个也缺
            match (chrome, driver) {
                (true, true) => row("Web浏览器", "可用", "Chrome for Testing", Tone::Plain),
                _ => {
                    let mut lack = Vec::new();
                    if !chrome { lack.push("Chrome for Testing") }
                    if !driver { lack.push("chromedriver") }
                    row("Web浏览器", "不可用", &format!("缺 {}", lack.join(" / ")), Tone::Bad)
                }
            }
        }

        // 体检只报**状态**，不教用法（怎么开窗口是 --help 的事），也不解释这意味着什么
        let headed = tke::utils::params::desktop_available();
        row("显示器环境", if headed { "有头" } else { "无头" }, "", Tone::Plain);
    }

    /// 模拟器这行要同时说清两件事：**有没有**（simctl）和**操作得了吗**（WDA）。
    /// 少说哪一件都会撞上那个最难查的组合：列得出来、命令也不报错，就是点不动
    fn print_ios_sim(&self) {
        if !cfg!(target_os = "macos") {
            row("iOS模拟器", "不可用", "需 macOS", Tone::Muted);
            return;
        }
        let sims: Vec<_> = self.disc.targets.iter().filter(|t| t.kind == "ios-sim").collect();
        let booted = sims.iter().filter(|t| t.ready).count();
        let wda = wda_app_path().is_some();
        if sims.is_empty() {
            row("iOS模拟器", "不可用", "尚未创建模拟器", Tone::Muted);
        } else if !wda {
            row("iOS模拟器", "操作不了", "缺 WebDriverAgent", Tone::Bad);
        } else if booted == 0 {
            row("iOS模拟器", "待启动", &format!("{} 台可选", sims.len()), Tone::Muted);
        } else {
            row("iOS模拟器", "可用", &format!("{} 台已启动", booted), Tone::Plain);
        }
    }

    // ── 段三：东西落在哪 ──（人找报告时不用回头问 AI）
    fn print_paths(&self) {
        row("Engine落点", &self.exe_dir.display().to_string(), "", Tone::Plain);
        match self.st.as_ref().and_then(|s| s.skill_dir.clone()).or_else(tke::utils::update::skill_dir) {
            Some(d) => row("Skill落点", &d.display().to_string(), "", Tone::Plain),
            None => row("Skill落点", "未安装", "", Tone::Muted),
        }
        if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            let logs = PathBuf::from(home).join(".tke").join("logs");
            row("默认日志落点", &logs.display().to_string(), "", Tone::Plain);
        }
    }

    /// 结论 + 提醒。**对钩在最后**：上面每行是一项检查，这行才是"到底行不行"
    pub fn print_verdict(&self, missing: usize) {
        println!();
        if missing == 0 && self.path_persisted == Some(false) {
            // 依赖齐了但 PATH 没落地——**不能说"全局已就绪"**，它恰恰只在这个窗口里就绪
            println!("  {} {}", sym_warn(), "当前窗口可用 · 新终端里还找不到 tke");
        } else if missing == 0 {
            println!("  {} {}", sym_ok(), "全局已就绪");
        } else {
            println!("  {} 环境不完整 · 缺 {} 项", sym_err(), missing);
        }
        if self.path_persisted == Some(false) && missing > 0 {
            println!("  {} {}", sym_warn(), "PATH 没写进 shell 配置 · 新终端里找不到 tke");
        }
        if self.st.as_ref().is_some_and(|s| s.any_stale()) {
            println!("  {} {}", sym_warn(), "有可用更新");
        }

        self.print_next_steps(missing);
    }

    /// 该敲的命令**全在这一块**，正文里一条都不留。
    ///
    /// 早先是哪行发现问题就在哪行缀一句「· tke doctor --fix」，看着贴心，实际是把
    /// 一份体检表撒成了几段说明书——用户的原话是「太散乱」。状态归状态、动作归动作，
    /// 要动手的人往最后看一眼就够了
    fn print_next_steps(&self, missing: usize) {
        let mut steps: Vec<(String, String)> = Vec::new();
        if missing > 0 {
            steps.push(("tke doctor --fix".into(), format!("补齐缺的 {} 项依赖", missing)));
        }
        // 模拟器缺 WDA 不算"环境不完整"（只影响模拟器），但要动手的话是另一条命令
        if cfg!(target_os = "macos")
            && matches!(self.profile.as_str(), "ios" | "all")
            && wda_app_path().is_none()
            && self.disc.targets.iter().any(|t| t.kind == "ios-sim")
        {
            steps.push((
                "tke doctor --fix --profile ios".into(),
                "装模拟器要用的 WebDriverAgent".into(),
            ));
        }
        if let Some(st) = self.st.as_ref().filter(|s| s.any_stale()) {
            // skill 旧的时候多说一句：调用方 AI 的上下文里那份是会话开始时加载的，
            // 只换磁盘文件它是不知道的——不重读等于白更新（P-41）
            let why = if st.skill_stale {
                "更新 tke 与 skill（更新后重读 SKILL.md）"
            } else {
                "更新到最新版"
            };
            steps.push(("tke update".into(), why.into()));
        }
        if self.path_persisted == Some(false) {
            steps.push((
                format!("echo 'export PATH=\"{}:$PATH\"' >> ~/.zshrc", self.exe_dir.display()),
                "让新终端也找得到 tke".into(),
            ));
        }
        if steps.is_empty() {
            return;
        }

        // 一行放得下的（`命令  说明`）对齐成一列；**放不下的自成两行**：说明在上、
        // 命令缩进在下。那条 export PATH 有八十多个字符，硬排进同一列会把其余几行的
        // 说明推到屏幕外——为一条长命令破掉整块的队形不值
        const CMD_W: usize = 34;
        let w = steps
            .iter()
            .map(|(c, _)| disp_width(c))
            .filter(|n| *n <= CMD_W)
            .max()
            .unwrap_or(0);
        println!();
        println!("  {}", dim("下一步"));
        for (cmd, why) in &steps {
            if disp_width(cmd) <= CMD_W {
                println!("    {}  {}", pad_right(cmd, w), dim(why));
            } else {
                println!("    {}", dim(why));
                println!("      {}", cmd);
            }
        }
    }

    /// 某一类**没查成**的原因（缺工具 / 平台不支持）。
    /// 「没查」与「没连」在结果上长得一样，不说清楚人只会去插拔数据线（INV-9）
    fn skip_why(&self, kind: &str) -> Option<String> {
        self.disc.skipped.iter().find(|s| s.kind == kind).map(|s| {
            // discover 的措辞是给 `device list` 那段用的整句
            //（「安卓未检测 · 缺 adb · tke doctor --fix」）。这里只取**中间那段原因**：
            // "未检测"已经是这行的值了，而那条命令归最后的「下一步」——
            // 每行都缀一句命令，一份表就散成了几段说明书
            let mut parts = s.why.split(" · ").skip(1);
            match parts.next() {
                Some(reason) if reason.starts_with("缺 ") => "缺少依赖".to_string(),
                Some(reason) => reason.to_string(),
                None => s.why.clone(),
            }
        })
    }
}

/// 给人看的系统名。分发源的平台标签（`darwin-arm64`）放括号里——
/// 那串是给机器和报 issue 用的，不该占主位
fn friendly_os() -> String {
    match std::env::consts::OS {
        "macos" => "macOS",
        "linux" => "Linux",
        "windows" => "Windows",
        o => o,
    }
    .to_string()
}

/// 新开的终端里还能不能直接敲 `tke`——**只看 shell rc 文件的内容**。
///
/// 为什么不看 `command -v tke` / 当前 PATH：**当前进程能跑，什么都证明不了**。
/// PATH 可能是刚才手动 `export` 的，也可能是安装脚本临时加的，窗口一关就没了。
/// 用户真踩过（P-33）：doctor 一路绿灯说"全局已就绪"，开个新 tab 就 `tke: command not found`。
/// 体检报的是"这台机器行不行"，不是"这个窗口行不行"。
///
/// 返回 `None` = 无从判断（Windows 的 PATH 在注册表里，不归 rc 文件管）。
pub fn path_persisted(exe_dir: &Path) -> Option<bool> {
    if cfg!(windows) {
        return None;
    }
    // 装进系统默认目录的（比如自己 cp 到 /usr/local/bin），rc 里当然不会有，但它本来就持久
    let dir = exe_dir.to_string_lossy().to_string();
    if matches!(
        dir.as_str(),
        "/usr/local/bin" | "/usr/bin" | "/bin" | "/opt/homebrew/bin" | "/usr/local/sbin"
    ) {
        return Some(true);
    }
    let home = PathBuf::from(std::env::var_os("HOME")?);
    // 覆盖 zsh / bash / sh 三种；任一命中就算数（新终端只会读其中之一）
    let hit = [".zshrc", ".zprofile", ".bashrc", ".bash_profile", ".profile"]
        .iter()
        .any(|f| {
            std::fs::read_to_string(home.join(f))
                .map(|s| s.contains(&dir))
                .unwrap_or(false)
        });
    Some(hit)
}

/// 模拟器用的 WDA runner 装在哪（`~/.tke/wda/WebDriverAgentRunner-Runner.app`）。
/// **与 drivers/wda/infra.rs 的 find_wda_app 必须同一口径**
pub fn wda_app_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".tke").join("wda"))
}

pub fn wda_app_path() -> Option<PathBuf> {
    let p = wda_app_dir()?.join("WebDriverAgentRunner-Runner.app");
    p.exists().then_some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 标签列必须按显示宽度对齐——中文占两格，`{:<15}` 按字符数填会错位
    #[test]
    fn labels_align_by_display_width() {
        use tke::utils::text::disp_width;
        for l in ["平台", "Android模拟器", "默认日志落点", "Engine版本"] {
            assert_eq!(disp_width(&pad_right(l, LABEL_W)), LABEL_W, "标签 {} 没对齐", l);
        }
    }

    /// 最长的标签不能挤爆列宽，否则那一行的值会顶出去
    #[test]
    fn longest_label_fits() {
        use tke::utils::text::disp_width;
        assert!(disp_width("Android模拟器") < LABEL_W);
    }
}
