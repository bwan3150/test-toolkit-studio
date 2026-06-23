// ToolkitEngine (tke) CLI Main Entrance
// tke = 所有测试工具的统一入口/协调器，四大块：
//   ① 直通      tke <二进制名> <原生指令>（同目录二进制自动可用，零代码扩展）
//   ② 原子方法  tke refresh / fetch / recognize / control（必带 -d/--device）
//   ③ 工作流    tke run <x.tks|x.toml> / tke steps "指令"... / tke harness <用例> --script <出>
//   ④ 自有工具  tke ocr / file / app / device
//
// 全局参数（均可放入 --config 指定的 tke.toml，CLI 显式参数优先）：
//   -d/--device   目标设备    --element  元素库路径
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

    /// 元素库 element.json 路径（缺省按 ./element.json → ./locator/element.json 查找）
    #[arg(long, global = true)]
    element: Option<PathBuf>,

    /// 产物输出目录（不传则 run/steps 不保存 log/截图序列/页面结构序列）
    #[arg(long, global = true)]
    log: Option<PathBuf>,

    /// 脚本输出目录（harness 生成的 .tks 落点；文件名由 AI 起、目录内去重。可写入 config 固定）
    #[arg(long, global = true)]
    scripts: Option<PathBuf>,

    /// 配置文件（缺省自动读 tke 同目录的 config.toml；CLI 显式参数优先于配置）
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// 强制输出 NDJSON 事件流（默认终端为友好格式，管道/重定向自动切 NDJSON）
    #[arg(long, global = true)]
    json: bool,

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
    /// [工具] 设备详细信息
    Device {
        #[command(subcommand)]
        action: DeviceCommands,
    },
    /// [工具] 元素库管理（按坐标取元素落库）
    Element {
        #[command(subcommand)]
        action: ElementCommands,
    },

    // ==================== ① 直通（不在静态列表，见 --help 末尾动态清单） ====================
    /// [直通] 透传任意同目录二进制: tke <工具名> <原生指令>
    #[command(external_subcommand)]
    Tool(Vec<String>),
}

#[tokio::main]
async fn main() -> tke::Result<()> {
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
    let params = Arc::new(tke::Params::resolve(cli.device, cli.element, cli.log, cli.scripts, cli.json, config));
    // 进程级设置在线 OCR 地址（识别引擎深处查询）
    tke::utils::params::set_ocr_url(params.ocr_url.clone());
    // 安装进程级 Ctrl+C 中断监听：run/steps/harness 各阶段（含 ScriptRunner 逐步回放）统一查中断、及时停
    tke::utils::interrupt::install();

    // 便捷路由: tke <path.tks|path.toml> 等价于 tke run <path>
    let tool_is_script = matches!(&cli.command, Commands::Tool(args)
        if args.first().map(|f| {
            let l = f.to_lowercase();
            l.ends_with(".tks") || l.ends_with(".toml")
        }).unwrap_or(false));

    // 直通命令：完全跳过日志初始化，保持原生工具体验
    // 注：harness 已改为内置 AI 闭环（不再透传 tester-ai），因此走正常日志初始化
    let is_passthrough_command = !tool_is_script && matches!(
        cli.command,
        Commands::Tool(_)
    );

    if !is_passthrough_command {
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
        Commands::Device { action } => {
            tools::device::handle(action, params.clone())
        }
        Commands::Element { action } => {
            tools::element::handle(action, params.clone()).await
        }
        // ① 直通（.tks/.toml 路径自动转 run）
        Commands::Tool(args) => {
            if tool_is_script {
                let path = PathBuf::from(&args[0]);
                workflow::run::handle(RunArgs { path, ocr: None }, params.clone()).await
            } else {
                passthrough::handle(args, params.clone()).await
            }
        }
    }
}
