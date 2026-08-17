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

    /// 这次要验的是什么（写用户的原话），显示在报告开头
    #[arg(long)]
    pub task: Option<String>,

    /// 结论：pass=功能可用 / fail=**被测对象有问题** / blocked=没验成（跑不下去）。
    /// 注意：某一步没点中**不算** fail——那是过程里的无效尝试
    #[arg(long, value_parser = ["pass", "fail", "blocked"])]
    pub verdict: Option<String>,

    /// 结论说明。**支持 Markdown**（表格/列表/加粗/小标题都会渲染出来）
    #[arg(long)]
    pub summary: Option<String>,

    /// 从文件读结论（内容同 `--summary`）。**总结长、带表格时用这个**——
    /// 一大段多行 Markdown 塞进命令行要跟引号和换行搏斗，先写成文件再指过来省事得多
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
        if let Some(v) = &args.task {
            t.task = Some(v.clone());
        }
        // --summary-file 优先：两个都给时，文件里那份通常才是完整的
        if let Some(f) = &args.summary_file {
            match std::fs::read_to_string(f) {
                Ok(s) => t.summary = Some(s.trim().to_string()),
                Err(e) => JsonOutput::error(format!("读不到 {}：{}", f.display(), e)),
            }
        } else if let Some(v) = &args.summary {
            t.summary = Some(v.clone());
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
        eprintln!("（无图形界面，没有打开浏览器；报告在上面那个路径）");
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
