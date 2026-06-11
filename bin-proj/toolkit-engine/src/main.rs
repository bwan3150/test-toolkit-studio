// ToolkitEngine (tke) CLI Main Entrance
// tke = 所有测试工具的统一入口/协调器，三大块：
//   ① 工具直通  tke adb/aapt/k6/ffmpeg/... <原生指令>（同目录二进制，零代码扩展）
//   ② 原子方法  tke refresh / fetch / recognize / control（必带 -d/--device）
//   ③ 工作流    tke run <x.tks|x.toml> / tke steps "指令"... / tke case <用例> --script <出>
//
// 全局参数（均可放入 --config 指定的 tke.toml，CLI 显式参数优先）：
//   -d/--device   目标设备
//   --element     元素库 element.json 路径
//   --log         产物输出目录（不传则不保存 log/截图/结构文件）
//   -c/--config   配置文件 tke.toml
//
// Main 只负责路由，所有命令处理逻辑都在 handlers 模块中

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod handlers;
use handlers::*;

#[derive(Parser)]
#[command(name = "tke")]
#[command(about = "Toolkit Engine - 所有测试工具的统一入口")]
#[command(version = env!("BUILD_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// 目标设备 ID（fetch/recognize/control 必须指定）
    #[arg(short, long, global = true)]
    device: Option<String>,

    /// 元素库 element.json 路径（缺省按 ./element.json → ./locator/element.json 查找）
    #[arg(long, global = true)]
    element: Option<PathBuf>,

    /// 产物输出目录（不传则 run 不保存 log/截图序列/页面结构序列）
    #[arg(long, global = true)]
    log: Option<PathBuf>,

    /// 配置文件 tke.toml（等同自动输入 device/element/log 等参数，CLI 显式参数优先）
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
    /// [工作流] 不落文件执行一串 .tks 指令: tke steps "点击 [{登录按钮}]" "等待 [2]"
    Steps {
        #[command(flatten)]
        args: StepsArgs,
    },
    /// [工作流] AI 根据文字用例探索测试并生成 .tks: tke case <用例.md|文字> --script <导出路径>
    Case {
        #[command(flatten)]
        args: CaseArgs,
    },

    // ==================== ① 工具直通 ====================
    /// [直通] ADB（-d 自动转为 adb -s）
    Adb {
        /// 透传给内嵌 adb 的参数
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// [直通] AAPT
    Aapt {
        /// 透传给内嵌 aapt 的参数
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    // ==================== 设备工具命令 ====================
    /// OCR - 图片文字识别
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
    /// File - Android 设备文件系统管理
    File {
        #[command(subcommand)]
        action: FileCommands,
    },
    /// App - 管理设备上的应用信息
    App {
        #[command(subcommand)]
        action: AppCommands,
    },
    /// Device - 获取设备详细信息
    Device {
        #[command(subcommand)]
        action: DeviceCommands,
    },

    // ==================== ① 工具直通（通用扩展位） ====================
    /// [直通] 其他任意工具: tke <工具名> <原生指令>（k6/ffmpeg/opencv/scrcpy/tester-ai...）
    #[command(external_subcommand)]
    Tool(Vec<String>),
}

#[tokio::main]
async fn main() -> tke::Result<()> {
    let cli = Cli::parse();

    // 加载配置文件并合并参数（CLI 显式参数优先）
    let config = match &cli.config {
        Some(path) => match tke::utils::TkeConfig::load(path) {
            Ok(c) => c,
            Err(e) => tke::JsonOutput::error(e.to_string()),
        },
        None => tke::utils::TkeConfig::default(),
    };

    let device = cli.device.or(config.device);
    let element = cli.element.or(config.element);
    let log = cli.log.or(config.log);

    // 直通命令：完全跳过日志初始化，保持原生工具体验
    let is_passthrough_command = matches!(
        cli.command,
        Commands::Adb { .. } | Commands::Aapt { .. } | Commands::Tool(_) | Commands::Case { .. }
    );

    if !is_passthrough_command {
        // 所有命令输出 JSON/NDJSON 到 stdout：日志一律走 stderr
        // 默认只输出 WARN 以上，保持 CLI 干净；-v 时输出 DEBUG
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

    // 路由到对应的 handler
    match cli.command {
        // ② 原子方法
        Commands::Refresh { args } => {
            refresh::handle(args, device).await
        }
        Commands::Fetch { args } => {
            fetch::handle(args, device).await
        }
        Commands::Recognize { args } => {
            recognize::handle(args, device, element).await
        }
        Commands::Control { action } => {
            control::handle(action, device).await
        }
        // ③ 工作流
        Commands::Run { args } => {
            runner::handle(args, device, element, log, cli.json).await
        }
        Commands::Steps { args } => {
            steps::handle(args, device, element, log, cli.json).await
        }
        Commands::Case { args } => {
            case_cmd::handle(args, device, element).await
        }
        // ① 工具直通
        Commands::Adb { args } => {
            adb::handle(args, device).await
        }
        Commands::Aapt { args } => {
            aapt::handle(args).await
        }
        Commands::Tool(args) => {
            tools::handle(args, device).await
        }
        // 设备工具命令
        Commands::Ocr { image, online, url, lang } => {
            ocr::handle(image, online, url, lang).await
        }
        Commands::File { action } => {
            file::handle(action, device).await
        }
        Commands::App { action } => {
            app::handle(action, device).await
        }
        Commands::Device { action } => {
            device::handle(action, device)
        }
    }
}
