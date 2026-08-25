// ToolkitEngine (tke) CLI Main Entrance
// tke = 所有测试工具的统一入口/协调器，四大块：
//   ① 直通      tke <二进制名> <原生指令>（同目录二进制自动可用，零代码扩展）
//   ② 原子方法  tke refresh / fetch / recognize / control（必带 -d/--device）
//   ③ 工作流    tke run <x.tks|x.toml> / tke steps "指令"... / tke harness <用例> --script <出>
//   ④ 自有工具  tke ocr / file / app / device
//
// 全局参数（均可放入 --config 指定的 tke.toml，CLI 显式参数优先）：
//   -d/--device   目标设备
//   --log         产物输出目录（不传则不保存产物）    --json  强制 NDJSON 输出
//
// Main 只负责路由，所有命令翻译逻辑都在 cli 模块中

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod cli;
use cli::*;

#[derive(Parser)]
#[command(name = "tke")]
#[command(about = "Toolkit Engine - 所有测试工具的统一入口")]
#[command(version = env!("BUILD_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// 目标设备 ID（refresh/fetch/recognize/control 必须指定）
    #[arg(short, long, global = true)]
    device: Option<String>,

    /// 产物输出目录（不传则 run/steps 不保存 log/截图序列/页面结构序列）
    #[arg(long, global = true)]
    log: Option<PathBuf>,

    /// 脚本输出目录（harness 生成的 .tks 落点；文件名由 AI 起、目录内去重。可写入 config 固定）
    #[arg(long, global = true)]
    scripts: Option<PathBuf>,

    /// 缓存目录：运行中间文件（截图/页面结构/会话日志/临时元素库）的落点；不传用系统临时目录。
    /// 这些只是运行中产物、不展示给用户，与脚本/交付文件分开。
    #[arg(long, global = true)]
    cache: Option<PathBuf>,

    /// 工作区目录：AI 能读写的文件范围根（.tks 脚本、save_file 交付文件都落这里及其子目录）。
    /// 不传则用进程的当前目录（CLI/TUI 直接用）；app spawn 时显式指定为用户项目目录。
    #[arg(long, global = true)]
    current_dir: Option<PathBuf>,

    /// 配置文件（缺省自动读 tke 同目录的 config.toml；CLI 显式参数优先于配置）
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// 强制输出 NDJSON 事件流（默认终端为友好格式，管道/重定向自动切 NDJSON）
    #[arg(long, global = true)]
    json: bool,

    /// AI 辅助驾驶（run 回放：开跑前起始态对齐 + 元素定位失败时按当前页面找回，默认开启，
    /// 需配置 [ai]）。--copilot 等价 --copilot true；--copilot false 或 config copilot = false 关闭
    #[arg(long, global = true, num_args = 0..=1, default_missing_value = "true")]
    copilot: Option<bool>,

    /// web 无头模式：auto（默认=无头，且沿用现有会话）/ on（强制无头）/ off（强制有头，
    /// 开窗口给人手动登录用）。裸 `--headless` 等价 `--headless=on`。
    /// **必须用等号形式**（require_equals）：否则 `tke --headless run x.tks` 里的 `run`
    /// 会被当成本参数的值吃掉，子命令就没了（--copilot 踩过同类坑，见 tests/cli.rs 回归）
    #[arg(
        long,
        global = true,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "on",
        value_parser = ["auto", "on", "off"]
    )]
    headless: Option<String>,

    /// 输出 DEBUG 级别日志
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    // ==================== ② 原子方法（必带 -d/--device） ====================
    /// [原子] 刷新页面状态：采集截图 + UI XML 到工作区 (+ OCR / 剪裁元素图)
    Refresh {
        #[command(flatten)]
        args: RefreshArgs,
    },
    /// [原子] 提取当前页面的元素列表（含 xpath），直接输出 JSON 数组
    Fetch {
        #[command(flatten)]
        args: FetchArgs,
    },
    /// [原子] 定位元素：在当前页面找到元素坐标 (xml/ocr/图像匹配)
    Recognize {
        #[command(flatten)]
        args: RecognizeArgs,
    },
    /// [原子] 执行操作：click/press/swipe/input 等统一操作名
    Control {
        #[command(subcommand)]
        action: ControlCommands,
    },

    // ==================== ③ 工作流 ====================
    /// [工作流] 执行 .tks 单脚本 / .toml flow(多脚本顺序执行)
    Run {
        #[command(flatten)]
        args: RunArgs,
    },
    /// [工作流] 不落文件执行一串 .tks 指令: tke steps "点击 [{登录按钮}]" "等待 [2s]"
    Steps {
        #[command(flatten)]
        args: StepsArgs,
    },
    /// [工作流] AI 根据文字用例探索测试并生成 .tks: tke harness <用例.md|文字> --scripts <输出目录>（文件名由 AI 起）
    // 方向已转向"测试 harness"，功能不变；harn 为简写
    #[command(visible_alias = "harn")]
    Harness {
        #[command(flatten)]
        args: HarnessArgs,
    },

    /// [编排] 出报告：按任务类型(ui/security)自动分派: tke report <检查目录>
    Report {
        #[command(flatten)]
        args: ReportArgs,
    },

    /// [编排] 起测试会话：建目录 + 写 task.json 标记（ui/security 共享）: tke task new --kind security --target <url>
    Task {
        #[command(subcommand)]
        action: cli::task::TaskCommands,
    },

    /// [环境] 升级到最新版（跑官方安装脚本；tke 与 skill 一起更新）
    Update {
        #[command(flatten)]
        args: UpdateArgs,
    },

    /// [环境] 卸载 tke 与 skill（默认保留日志与 Chrome）
    Uninstall {
        #[command(flatten)]
        args: UninstallArgs,
    },

    /// [环境] 体检：依赖齐不齐、设备连没连、版本跟不跟得上（加 --fix 才联网补依赖）
    Doctor {
        #[command(flatten)]
        args: FixArgs,
    },

    /// [环境] 补齐缺失的运行依赖——**唯一会联网下载的命令**（= `doctor --fix` 的别名）
    Fix {
        #[command(flatten)]
        args: FixArgs,
    },

    /// [服务] 把这台机器的能力开成 HTTP 接口，供远程调用（ADR-0022）
    Serve {
        #[command(flatten)]
        args: cli::serve::ServeArgs,
    },

    /// [服务] 远程会话管理（配了 TKE_REMOTE 后，普通命令会自动发给节点）
    Remote {
        #[command(subcommand)]
        action: cli::remote::RemoteCommands,
    },

    // ==================== ④ 自有工具 ====================
    /// [工具] OCR 图片文字识别
    Ocr {
        /// 图片路径
        #[arg(short, long)]
        image: PathBuf,

        /// 在线模式（调用 HTTP API）
        #[arg(long)]
        online: bool,

        /// 在线 API 完整 URL (如 http://localhost:8000/ocr)
        #[arg(long)]
        url: Option<String>,

        /// 离线 OCR 语言 (eng, chi_sim 等)
        #[arg(long, default_value = "eng")]
        lang: String,
    },
    /// [工具] Android 设备文件系统管理
    File {
        #[command(subcommand)]
        action: FileCommands,
    },
    /// [工具] 设备应用管理
    App {
        #[command(subcommand)]
        action: AppCommands,
    },
    /// [工具] 设备：list 看有哪些能测 / info 看某台的详情 / prop 读安卓属性
    Device {
        /// 不给子命令就等于 `list`——**问"有哪些设备"是最常见的那次**，
        /// 不该让人多打一个词（`tke device` 直接列出来）
        #[command(subcommand)]
        action: Option<DeviceCommands>,

        /// 连没启动的模拟器一起列（= `tke device list --all`）。
        /// **顶层也收这个参数**：`device` 既然等于 `device list`，
        /// 那 `tke device --all` 就该能用——不然"省一个词"反而变成了要多记一条规矩
        #[arg(long)]
        all: bool,
    },
    /// [工具] 元素库管理（按坐标取元素落库）
    Element {
        #[command(subcommand)]
        action: ElementCommands,
    },

    // ==================== ⑤ 安全测试（ADR-0019，P1 侦察底座） ====================
    /// [安全] 原始 HTTP 探测（落证据）: tke http GET <url> [-H 'K: V'] [-d body]
    Http {
        #[command(flatten)]
        args: cli::security::HttpArgs,
    },
    /// [安全] 侦察检查: tke recon headers <url>（安全响应头等被动判据）
    Recon {
        #[command(subcommand)]
        action: cli::security::ReconCommands,
    },
    /// [安全] 安全测试唯一入口: tke security [url]（默认对话式；--json 无头对接，需 [ai]）
    Security {
        #[command(flatten)]
        args: cli::security::SecurityArgs,
    },

    // ==================== ① 直通（不在静态列表，见 --help 末尾动态清单） ====================
    /// [直通] 透传任意同目录二进制: tke <工具名> <原生指令>
    #[command(external_subcommand)]
    Tool(Vec<String>),
}

#[tokio::main]
async fn main() -> tke::Result<()> {
    // 远程模式拦截（ADR-0022 D4）：**必须在 clap 之前**——远程要原样转发命令，
    // 先过一遍本地 clap 等于要求两端版本严格一致，那正是我们想避免的耦合。
    // 不在白名单里的命令拿不到 Some，照旧走本地
    if let Some(cfg) = tke::remote::RemoteConfig::from_env() {
        if let Some(code) = tke::remote::maybe_dispatch(&cfg) {
            std::process::exit(code);
        }
    }

    let matches = Cli::command()
        .override_help(cli::help::build_help())
        .get_matches();
    let cli = Cli::from_arg_matches(&matches)
        .unwrap_or_else(|e| e.exit());

    // 加载配置文件并合并参数（CLI 显式参数优先）
    // 配置来源优先级（同时只用一个）: --config 指定 > tke 同目录 config.toml > 无
    let config = match &cli.config {
        Some(path) => match tke::utils::TkeConfig::load(path) {
            Ok(c) => c,
            Err(e) => tke::JsonOutput::error(e.to_string()),
        },
        None => {
            let default_config = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("config.toml")))
                .filter(|p| p.exists());
            match default_config {
                Some(path) => match tke::utils::TkeConfig::load(&path) {
                    Ok(c) => c,
                    Err(e) => tke::JsonOutput::error(format!("默认配置 {} 解析失败: {}", path.display(), e)),
                },
                None => tke::utils::TkeConfig::default(),
            }
        }
    };

    // 参数层：CLI + config 解析一次，形成统一参数表（Arc 共享，编排层各模块持有并查表）
    let params = Arc::new(tke::Params::resolve(cli.device, cli.log, cli.scripts, cli.cache, cli.current_dir, cli.json, cli.copilot, cli.headless, config));
    // 进程级设置在线 OCR 地址（识别引擎深处查询）
    tke::utils::params::set_ocr_url(params.ocr_url.clone());
    // 进程级设置 web 无头模式（web 驱动建会话时查询）
    tke::utils::params::set_web_headless(params.headless);
    // 安装进程级 Ctrl+C 中断监听：run/steps/harness 各阶段（含 ScriptRunner 逐步回放）统一查中断、及时停
    tke::utils::interrupt::install();

    // 便捷路由: tke <path.tks|path.toml> 等价于 tke run <path>
    let tool_is_script = matches!(&cli.command, Commands::Tool(args)
        if args.first().map(|f| {
            let l = f.to_lowercase();
            l.ends_with(".tks") || l.ends_with(".toml")
        }).unwrap_or(false));

    // 未知命令那条路只打几行指路文字，不需要日志子系统
    let is_unknown_command = !tool_is_script && matches!(cli.command, Commands::Tool(_));

    if !is_unknown_command {
        // 默认只输出 WARN 以上保持 CLI 干净；-v 输出 DEBUG；日志一律走 stderr
        let level = if cli.verbose {
            tracing::Level::DEBUG
        } else {
            tracing::Level::WARN
        };

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| format!("{}", level).into()),
            )
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::io::stderr)
            )
            .init();
    }

    // 路由到对应的 handler —— 编排层持有 Arc<Params>，查表取参（取代逐层透传）
    match cli.command {
        // ② 原子方法
        Commands::Refresh { args } => {
            atomic::refresh::handle(args, params.clone()).await
        }
        Commands::Fetch { args } => {
            atomic::fetch::handle(args, params.clone()).await
        }
        Commands::Recognize { args } => {
            atomic::recognize::handle(args, params.clone()).await
        }
        Commands::Control { action } => {
            atomic::control::handle(action, params.clone()).await
        }
        // ③ 工作流
        Commands::Run { args } => {
            workflow::run::handle(args, params.clone()).await
        }
        Commands::Steps { args } => {
            workflow::steps::handle(args, params.clone()).await
        }
        Commands::Harness { args } => {
            workflow::harness::handle(args, params.clone()).await
        }
        Commands::Report { args } => {
            report::handle(args).await
        }
        Commands::Task { action } => {
            cli::task::handle(action, params.clone()).await
        }
        Commands::Update { args } => {
            cli::selfmanage::update(args).await
        }
        Commands::Uninstall { args } => {
            cli::selfmanage::uninstall(args).await
        }
        Commands::Doctor { args } => {
            fix::handle(args).await
        }
        // 别名：`tke fix` 的旧语义是"默认就下载"，不能因为改名而变——
        // 已发布的 install.sh 和用户脚本里全是这条
        Commands::Fix { args } => {
            fix::handle_as(args, true).await
        }
        Commands::Serve { args } => {
            cli::serve::handle(args).await
        }
        Commands::Remote { action } => {
            cli::remote::handle(action).await
        }
        // ④ 自有工具
        Commands::Ocr { image, online, url, lang } => {
            tools::ocr::handle(image, online, url, lang).await
        }
        Commands::File { action } => {
            tools::file::handle(action, params.clone()).await
        }
        Commands::App { action } => {
            tools::app::handle(action, params.clone()).await
        }
        Commands::Device { action, all } => {
            // 顶层 `--all` 只对 list 有意义；给了子命令就以子命令自己的参数为准
            let action = action.unwrap_or(cli::tools::DeviceCommands::List { all });
            tools::device::handle(action, params.clone())
        }
        Commands::Element { action } => {
            tools::element::handle(action, params.clone()).await
        }
        // ⑤ 安全测试（ADR-0019）
        Commands::Http { args } => {
            cli::security::http(args, params.clone()).await
        }
        Commands::Recon { action } => {
            cli::security::recon(action, params.clone()).await
        }
        Commands::Security { args } => {
            cli::security::security(args, params.clone()).await
        }
        // 便捷路由：tke <path.tks|path.toml> 等价于 tke run <path>。
        // 认不出的就是**未知命令**——CLI 直通已删（ADR-0015），
        // `tke adb shell input tap …` 这类用法绕过证据留存和坐标换算，
        // 点得中、什么都没留下、报告里一片空白
        Commands::Tool(args) => {
            if tool_is_script {
                let path = PathBuf::from(&args[0]);
                workflow::run::handle(RunArgs { path, ocr: None }, params.clone()).await
            } else {
                let what = args.first().map(String::as_str).unwrap_or("");
                eprintln!("未知命令：{}", what);
                eprintln!("tke 不再透传原生工具——设备操作一律走 tke 指令，由 tke 转译后落到二进制上。");
                eprintln!("  看日志       tke -d <设备> app log -p <包名>");
                eprintln!("  应用/文件/信息 tke app|file|device --help");
                eprintln!("  操作设备      tke control --help  /  tke steps '点击 [\"登录\"]'");
                eprintln!("  跑脚本        tke run <path.tks>（或直接 tke <path.tks>）");
                std::process::exit(2);
            }
        }
    }
}
