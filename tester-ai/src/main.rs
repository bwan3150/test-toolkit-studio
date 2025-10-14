// AI Tester 主程序 - CLI 入口

use anyhow::{Context, Result};
use clap::Parser;
use std::io::Read;
use tester_ai::{TesterInput, TesterOrchestrator};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

/// AI 自动化测试员 - 基于 RIG 框架的智能测试工具
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 从 stdin 读取 JSON 输入
    #[arg(long)]
    stdin: bool,

    /// 从文件读取 JSON 输入
    #[arg(long, value_name = "FILE")]
    input_file: Option<String>,

    /// 日志级别 (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // 初始化日志
    init_logging(&args.log_level)?;

    info!("AI Tester 启动");

    // 读取输入
    let input = if args.stdin {
        // 从 stdin 读取
        read_input_from_stdin()?
    } else if let Some(file) = args.input_file {
        // 从文件读取
        read_input_from_file(&file)?
    } else {
        error!("必须指定 --stdin 或 --input-file");
        anyhow::bail!("必须指定 --stdin 或 --input-file 参数");
    };

    info!("读取输入成功: 测试用例 {}", input.test_case_name);

    // 创建控制器并运行
    let mut orchestrator = TesterOrchestrator::new(input);

    match orchestrator.run().await {
        Ok(output) => {
            // 输出 JSON 结果到 stdout
            let json = serde_json::to_string_pretty(&output)
                .context("序列化输出失败")?;

            println!("{}", json);

            if output.success {
                info!("测试成功完成");
                std::process::exit(0);
            } else {
                error!("测试失败: {:?}", output.result);
                std::process::exit(1);
            }
        }
        Err(e) => {
            error!("测试执行失败: {:#}", e);

            // 输出错误 JSON
            let error_output = serde_json::json!({
                "success": false,
                "error": e.to_string(),
            });

            println!("{}", serde_json::to_string_pretty(&error_output)?);

            std::process::exit(1);
        }
    }
}

/// 从 stdin 读取 JSON 输入
fn read_input_from_stdin() -> Result<TesterInput> {
    let mut buffer = String::new();
    std::io::stdin()
        .read_to_string(&mut buffer)
        .context("读取 stdin 失败")?;

    serde_json::from_str(&buffer).context("解析 JSON 输入失败")
}

/// 从文件读取 JSON 输入
fn read_input_from_file(path: &str) -> Result<TesterInput> {
    let content = std::fs::read_to_string(path)
        .context(format!("读取文件失败: {}", path))?;

    serde_json::from_str(&content).context("解析 JSON 输入失败")
}

/// 初始化日志系统
fn init_logging(level: &str) -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level))
        .context("初始化日志过滤器失败")?;

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    Ok(())
}
