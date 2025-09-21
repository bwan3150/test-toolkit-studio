// AI自动化测试员主程序

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

mod agents;
mod core;
mod models;
mod ocr;
mod tke_integration;
mod test_rig;

use core::{test_controller::AiTestController, Result, AiTesterError};
use models::{TestCase, TestResult};

#[derive(Parser)]
#[command(name = "ai-tester")]
#[command(about = "AI自动化测试员 - 基于RIG框架的智能测试系统")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// OpenAI API Key
    #[arg(long)]
    api_key: String,

    /// 项目路径
    #[arg(short, long)]
    project: PathBuf,

    /// 设备ID
    #[arg(short, long)]
    device: String,

    /// 详细输出
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// 运行AI测试
    Test {
        /// 测试用例名称
        #[arg(long)]
        name: String,

        /// 测试用例描述
        #[arg(long)]
        description: String,
    },
    /// 检查系统状态
    Status,
    /// 列出可用设备
    Devices,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 初始化日志
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(log_level))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("AI自动化测试员启动");

    // 创建测试控制器
    let controller = AiTestController::new(
        &cli.project,
        cli.device.clone(),
        cli.api_key,
    )?;

    match cli.command {
        Commands::Test { name, description } => {
            run_test(&controller, name, description).await?;
        }
        Commands::Status => {
            check_status(&controller).await?;
        }
        Commands::Devices => {
            list_devices(&controller).await?;
        }
    }

    Ok(())
}

async fn run_test(
    controller: &AiTestController,
    name: String,
    description: String,
) -> Result<()> {
    info!("开始执行AI测试: {}", name);

    let test_case = TestCase {
        id: Uuid::new_v4(),
        name: name.clone(),
        description,
        project_path: controller.project_path.to_string_lossy().to_string(),
    };

    match controller.run_ai_test(test_case).await? {
        TestResult::InProgress => {
            info!("测试正在进行中...");
        }
        TestResult::Completed => {
            info!("✅ 测试完成且通过!");
        }
        TestResult::Failed(reason) => {
            error!("❌ 测试失败: {}", reason);
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn check_status(controller: &AiTestController) -> Result<()> {
    info!("检查系统状态...");

    // 检查TKE可用性
    if controller.check_tke_availability()? {
        info!("✅ TKE (Toolkit Engine) 正常");
    } else {
        error!("❌ TKE (Toolkit Engine) 不可用");
    }

    // 检查设备连接
    let device_info = controller.get_device_info()?;
    info!("✅ 设备连接正常: {} ({})", device_info.device_id, device_info.platform);

    Ok(())
}

async fn list_devices(controller: &AiTestController) -> Result<()> {
    info!("获取设备列表...");

    // TODO: 通过 TKE 获取设备列表
    info!("可用设备:");
    info!("  - {}", controller.device_id);

    Ok(())
}
