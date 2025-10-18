// ToolkitEngine (tke) CLI Main Entrance
// Main 只负责路由，所有命令处理逻辑都在 handlers 模块中

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod handlers;
use handlers::*;

#[derive(Parser)]
#[command(name = "tke")]
#[command(about = "Toolkit Engine")]
#[command(version = env!("BUILD_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Phone Device ID (optional)
    #[arg(short, long)]
    device: Option<String>,

    /// Project Path
    #[arg(short, long)]
    project: Option<PathBuf>,

    /// Verbose level
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Controller - ADB control
    Controller {
        #[command(subcommand)]
        action: ControllerCommands,
    },
    /// LocatorFetcher - fetch useful element from XML
    Fetcher {
        #[command(subcommand)]
        action: FetcherCommands,
    },
    /// Recognizer - recognize image element from large image
    Recognizer {
        #[command(subcommand)]
        action: RecognizerCommands,
    },
    /// ScriptParser - for .tks script highlight and render
    Parser {
        #[command(subcommand)]
        action: ParserCommands,
    },
    /// Runner - run .tks script in cli(not used in Toolkit Studio Desktop App)
    Run {
        #[command(subcommand)]
        action: RunCommands,
    },
    /// ADB - THIS IS JUST ADB!!!!! directly adb
    Adb {
        /// forward adb command to inner adb
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// OCR - extract words from image
    Ocr {
        /// image path
        #[arg(short, long)]
        image: PathBuf,

        /// Online with api or Offline with tesseract and tesseract.rs
        #[arg(long)]
        online: bool,

        /// Full URL for online api (e.g. http://localhost:8000/ocr)
        #[arg(long)]
        url: Option<String>,

        /// language selection for offline ocr (eng, chi_sim, etc.)
        #[arg(long, default_value = "eng")]
        lang: String,
    },
}

#[tokio::main]
async fn main() -> tke::Result<()> {
    let cli = Cli::parse();

    // 检查是否是 ADB 直通命令
    let is_adb_command = matches!(cli.command, Commands::Adb { .. });

    // 对于 ADB 直通命令，完全跳过日志初始化和项目信息输出
    let project_path = if !is_adb_command {
        // 初始化日志
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
            .with(tracing_subscriber::fmt::layer())
            .init();

        // 获取项目路径
        let project_path = cli.project.unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        });

        // 对于输出JSON的命令,完全静默,不输出任何日志
        // 包括: Fetcher、Parser、OCR、Controller、Recognizer
        let is_json_output_command = matches!(
            cli.command,
            Commands::Fetcher { .. } |
            Commands::Parser { .. } |
            Commands::Ocr { .. } |
            Commands::Controller { .. } |
            Commands::Recognizer { .. }
        );

        if !is_json_output_command {
            info!("项目路径: {:?}", project_path);
            if let Some(ref device) = cli.device {
                info!("目标设备: {}", device);
            }
        }

        project_path
    } else {
        // ADB 直通模式：获取项目路径但不输出任何信息
        cli.project.unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        })
    };

    // 路由到对应的 handler
    match cli.command {
        Commands::Controller { action } => {
            controller::handle(action, cli.device, project_path).await
        }
        Commands::Fetcher { action } => {
            fetcher::handle(action, project_path).await
        }
        Commands::Recognizer { action } => {
            recognizer::handle(action, project_path).await
        }
        Commands::Parser { action } => {
            parser::handle(action).await
        }
        Commands::Run { action } => {
            runner::handle(action, project_path, cli.device).await
        }
        Commands::Adb { args } => {
            adb::handle(args, cli.device).await
        }
        Commands::Ocr { image, online, url, lang } => {
            ocr::handle(image, online, url, lang).await
        }
    }
}
