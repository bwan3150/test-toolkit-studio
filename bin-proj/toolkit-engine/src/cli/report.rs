// Report 命令：重建报告 / 写入任务结论 / 打开给人看
//
// 正常情况下 `tke steps` 每批跑完会自动重建报告，这条命令用于**收尾**：
//   - `--task` / `--verdict` / `--summary` 写上"要验什么、验成没有"
//     —— **tke 自己判断不了这些**：某一步定位没命中只说明那次尝试无效，
//     换个方式点中了就没事；功能到底能不能用，只有走完全程的调用方 AI 知道
//   - `--open` 直接在浏览器里打开，人不用去翻路径
//   - `--full-image` 出一份原图版，逐像素复核用
//   - 汇总**旧布局**的碎批次目录（`steps_*/`，含跨设备分 `web/` `phone/` 的）

use std::path::{Path, PathBuf};

use tke::{JsonOutput, Result, Verdict};

/// Report 命令参数
#[derive(clap::Args)]
pub struct ReportArgs {
    /// 任务目录（`--log` 指的那个，如 ~/.tke/logs/<任务简称>/）
    pub dir: PathBuf,

    /// 这次要验的是什么（写用户的原话），显示在报告开头。
    /// 写 `-` 表示从**标准输入**读（长文本用这个，见 `--summary`）
    #[arg(long)]
    pub task: Option<String>,

    /// 结论：pass=功能可用 / fail=**被测对象有问题** / blocked=没验成（跑不下去）。
    /// 注意：某一步没点中**不算** fail——那是过程里的无效尝试
    #[arg(long, value_parser = ["pass", "fail", "blocked"])]
    pub verdict: Option<String>,

    /// 结论说明。**支持 Markdown**（表格/列表/加粗/小标题都会渲染出来）。
    ///
    /// **长文本写 `-`，内容走标准输入**——不用先落一个临时文件：
    ///
    ///   tke report <目录> --verdict pass --summary - <<'EOF'
    ///   ## 结论
    ///   | 端 | 结果 |
    ///   |---|---|
    ///   | 安卓 | 通过 |
    ///   EOF
    ///
    /// heredoc 天然处理引号与换行，比拼一条超长命令行稳得多。
    /// PowerShell 用 here-string：`@'…'@ | tke report <目录> --summary -`
    #[arg(long)]
    pub summary: Option<String>,

    /// 从文件读结论（内容同 `--summary`）。**已经有现成的 .md 时用这个**；
    /// 只是想传一段长文本的话用 `--summary -`，省掉写临时文件那一步
    #[arg(long, value_name = "文件")]
    pub summary_file: Option<PathBuf>,

    /// 生成后用系统默认程序打开（mac=open / Linux=xdg-open / Windows=start）
    #[arg(long)]
    pub open: bool,

    /// 用**原图**内嵌（默认压缩内嵌：缩到宽 960 + JPEG，体积约 1/3，控件与文字仍清晰）
    #[arg(long)]
    pub full_image: bool,

    /// 仅对**旧布局**（碎批次目录）有效：把截图内嵌成单文件而不是相对链接
    #[arg(long, hide = true)]
    pub embed: bool,
}

pub async fn handle(args: ReportArgs) -> Result<()> {
    if !args.dir.is_dir() {
        JsonOutput::error(format!("目录不存在: {}", args.dir.display()));
    }

    // 领域即数据（ADR-0021）：目录里 task.json 标记 / findings.json → 安全任务，走安全报告；
    // 否则走下面的设备/UI 报告。同一条 `tke report <dir>` 两轨通用。
    if tke::workflow::task::is_security_task(&args.dir) {
        let fj = args.dir.join("findings.json");
        let json = std::fs::read_to_string(&fj).map_err(|e| {
            tke::TkeError::InvalidArgument(format!(
                "安全任务但读不到 {}：{e}（调用方应先把 findings.json 写进任务目录）", fj.display()))
        })?;
        let paths = tke::workflow::security::report::write_reports_from_json(&args.dir, &json)?;
        JsonOutput::print(serde_json::json!({
            "success": true,
            "kind": "security",
            "report_html": paths.html.to_string_lossy(),
            "findings_json": paths.json.to_string_lossy(),
            "vuln_reports": paths.vulns.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>(),
            "out_dir": args.dir.to_string_lossy(),
        }));
        return Ok(());
    }

    // 任务布局：目录里直接躺着 log.json
    let is_task = args.dir.join("log.json").is_file();

    // 先把结论写进 log.json，再渲染——否则这次生成的报告里还是旧结论
    if args.task.is_some() || args.verdict.is_some() || args.summary.is_some()
        || args.summary_file.is_some()
    {
        if !is_task {
            JsonOutput::error("这个目录不是任务布局（没有 log.json），写不了任务结论");
        }
        let log = args.dir.join("log.json");
        let mut t = tke::TaskLog::load(&log);

        // `-` = 从标准输入读。**stdin 只能读一次**，所以 task 与 summary 里
        // 只能有一个用 `-`——两个都写会让第二个拿到空串，那是最难查的那种"成功"
        let both_stdin = args.task.as_deref() == Some("-") && args.summary.as_deref() == Some("-");
        if both_stdin {
            JsonOutput::error("`--task` 和 `--summary` 只能有一个写 `-`（标准输入只能读一次）");
        }
        let read_stdin = || {
            use std::io::Read;
            let mut buf = String::new();
            match std::io::stdin().read_to_string(&mut buf) {
                Ok(_) if !buf.trim().is_empty() => buf.trim().to_string(),
                Ok(_) => JsonOutput::error("`-` 是从标准输入读，但没读到内容"),
                Err(e) => JsonOutput::error(format!("读标准输入失败：{}", e)),
            }
        };

        if let Some(v) = &args.task {
            t.task = Some(if v == "-" { read_stdin() } else { v.clone() });
        }
        // --summary-file 优先：两个都给时，文件里那份通常才是完整的
        if let Some(f) = &args.summary_file {
            match std::fs::read_to_string(f) {
                Ok(s) => t.summary = Some(s.trim().to_string()),
                Err(e) => JsonOutput::error(format!("读不到 {}：{}", f.display(), e)),
            }
        } else if let Some(v) = &args.summary {
            t.summary = Some(if v == "-" { read_stdin() } else { v.clone() });
        }
        if let Some(v) = &args.verdict {
            t.verdict = Verdict::parse(v);
        }
        let json = serde_json::to_string_pretty(&t).map_err(tke::TkeError::JsonError)?;
        std::fs::write(&log, json).map_err(tke::TkeError::IoError)?;
    }

    let r = if is_task {
        tke::workflow::report::write_task_report_with(&args.dir, args.full_image)
    } else {
        tke::workflow::report::write_session_report(&args.dir, args.embed || args.full_image)
    };
    match r {
        Ok(p) => {
            println!("{}", p.display());
            if args.open {
                open_in_browser(&p);
            }
            Ok(())
        }
        Err(e) => JsonOutput::error(e.to_string()),
    }
}

/// 用系统默认程序打开报告。**打不开不算错**——报告已经生成了，路径也打印了，
/// 无头服务器/CI 里本来就没有浏览器可开。
fn open_in_browser(path: &Path) {
    // 无桌面就别费劲了：xdg-open 在无 DISPLAY 时会报一串看不懂的错
    if !tke::utils::params::desktop_available() {
        eprintln!("（无图形界面，没有打开浏览器）");
        return;
    }
    let (prog, pre): (&str, &[&str]) = if cfg!(target_os = "macos") {
        ("open", &[])
    } else if cfg!(target_os = "windows") {
        // start 是 cmd 的内建命令，不是可执行文件；第一个引号参数会被当成窗口标题，故留空
        ("cmd", &["/C", "start", ""])
    } else {
        ("xdg-open", &[])
    };
    let ok = std::process::Command::new(prog)
        .args(pre)
        .arg(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok();
    if !ok {
        eprintln!("（没能自动打开，手动开上面那个路径就行）");
    }
}
