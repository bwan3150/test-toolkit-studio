// ToolkitEngine (tke) CLI Main Entrance
// tke = 所有测试工具的统一入口/协调器，三大块：
//   ① 工具直通  tke adb/aapt/k6/ffmpeg/... <原生指令>（同目录二进制，零代码扩展）
//   ② 原子方法  tke fetch / recognize / control（必带 -d/--device）
//   ③ 工作流    tke run script / flow / ai / step
// Main 只负责路由，所有命令处理逻辑都在 handlers 模块中

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;
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

    /// 项目路径（默认当前目录）
    #[arg(short, long, global = true)]
    project: Option<PathBuf>,

    /// 输出 DEBUG 级别日志
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    // ==================== ② 原子方法（必带 -d/--device） ====================
    /// [原子] 采集页面：截图 + UI XML (+ OCR / 元素列表 / 裁剪元素图)
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
    /// [工作流] 执行 .tks 脚本 / flow / AI 探索生成
    Run {
        #[command(subcommand)]
        action: RunCommands,
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

    // ==================== legacy 兼容命令 ====================
    /// (legacy) ADB 控制，由 fetch/control 替代
    Controller {
        #[command(subcommand)]
        action: ControllerCommands,
    },
    /// (legacy) XML 元素解析，由 fetch --elements 替代
    Fetcher {
        #[command(subcommand)]
        action: FetcherCommands,
    },
    /// (legacy) 元素识别，由 recognize 替代
    Recognizer {
        #[command(subcommand)]
        action: RecognizerCommands,
    },
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

    // 直通命令：完全跳过日志初始化和项目信息输出，保持原生工具体验
    let is_passthrough_command = matches!(
        cli.command,
        Commands::Adb { .. } | Commands::Aapt { .. } | Commands::Tool(_)
    ) || matches!(cli.command, Commands::Run { action: RunCommands::Ai { .. } });

    let project_path = if !is_passthrough_command {
        // 除直通外所有命令都输出 JSON：日志一律走 stderr，保持 stdout 纯净
        let level = if cli.verbose {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
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

        let project_path = cli.project.unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        });

        if cli.verbose {
            info!("项目路径: {:?}", project_path);
            if let Some(ref device) = cli.device {
                info!("目标设备: {}", device);
            }
        }

        project_path
    } else {
        cli.project.unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        })
    };

    // 路由到对应的 handler
    match cli.command {
        // ② 原子方法
        Commands::Fetch { args } => {
            fetch::handle(args, cli.device, project_path).await
        }
        Commands::Recognize { args } => {
            recognize::handle(args, cli.device, project_path).await
        }
        Commands::Control { action } => {
            control::handle(action, cli.device).await
        }
        // ③ 工作流
        Commands::Run { action } => {
            runner::handle(action, project_path, cli.device).await
        }
        // ① 工具直通
        Commands::Adb { args } => {
            adb::handle(args, cli.device).await
        }
        Commands::Aapt { args } => {
            aapt::handle(args).await
        }
        Commands::Tool(args) => {
            tools::handle(args, cli.device).await
        }
        // legacy 兼容命令
        Commands::Controller { action } => {
            controller::handle(action, cli.device, project_path).await
        }
        Commands::Fetcher { action } => {
            fetcher::handle(action, project_path).await
        }
        Commands::Recognizer { action } => {
            recognizer::handle(action, project_path).await
        }
        Commands::Ocr { image, online, url, lang } => {
            ocr::handle(image, online, url, lang).await
        }
        Commands::File { action } => {
            file::handle(action, cli.device).await
        }
        Commands::App { action } => {
            app::handle(action, cli.device).await
        }
        Commands::Device { action } => {
            device::handle(action, cli.device)
        }
    }
}
