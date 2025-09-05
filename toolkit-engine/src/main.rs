// ToolkitEngine (tke) - 自动化测试CLI工具主入口

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::io::Read;
use tracing::{info, error, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use tke::{
    Controller, LocatorFetcher, Recognizer, ScriptParser, 
    Runner, Result, TkeError
};

#[derive(Parser)]
#[command(name = "tke")]
#[command(about = "Toolkit Engine - 自动化测试CLI工具")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// 设备ID (可选，用于多设备环境)
    #[arg(short, long)]
    device: Option<String>,
    
    /// 项目路径
    #[arg(short, long)]
    project: Option<PathBuf>,
    
    /// 详细输出
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Controller - ADB控制功能
    Controller {
        #[command(subcommand)]
        action: ControllerCommands,
    },
    /// LocatorFetcher - XML元素获取
    Fetcher {
        #[command(subcommand)]
        action: FetcherCommands,
    },
    /// Recognizer - 元素识别
    Recognizer {
        #[command(subcommand)]
        action: RecognizerCommands,
    },
    /// ScriptParser - 脚本解析
    Parser {
        #[command(subcommand)]
        action: ParserCommands,
    },
    /// Runner - 脚本执行
    Run {
        #[command(subcommand)]
        action: RunCommands,
    },
}

#[derive(Subcommand)]
enum ControllerCommands {
    /// 获取连接的设备列表
    Devices,
    /// 获取设备截图和XML并保存到项目workarea
    Capture,
    /// 点击坐标
    Tap {
        /// X坐标
        x: i32,
        /// Y坐标  
        y: i32,
    },
    /// 滑动
    Swipe {
        /// 起点X坐标
        x1: i32,
        /// 起点Y坐标
        y1: i32,
        /// 终点X坐标
        x2: i32,
        /// 终点Y坐标
        y2: i32,
        /// 持续时间(毫秒)
        #[arg(short, long, default_value = "300")]
        duration: u32,
    },
    /// 启动应用
    Launch {
        /// 应用包名
        package: String,
        /// Activity名
        activity: String,
    },
    /// 停止应用
    Stop {
        /// 应用包名
        package: String,
    },
    /// 输入文本
    Input {
        /// 要输入的文本
        text: String,
    },
    /// 返回键
    Back,
    /// 主页键
    Home,
    /// 获取UI XML内容
    GetXml,
}

#[derive(Subcommand)]
enum FetcherCommands {
    /// 从XML文件获取所有UI元素
    Extract {
        /// XML文件路径
        xml_path: PathBuf,
    },
    /// 从当前workarea获取UI元素
    Current,
    /// 过滤可交互元素
    Interactive,
    /// 过滤有文本的元素
    Text,
    /// 从XML推断屏幕尺寸
    InferScreenSize {
        /// XML内容 (从stdin读取如果未提供)
        xml_content: Option<String>,
    },
    /// 优化UI树结构
    OptimizeUITree {
        /// XML内容 (从stdin读取如果未提供)
        xml_content: Option<String>,
    },
    /// 从UI树提取元素列表
    ExtractUIElements {
        /// XML内容 (从stdin读取如果未提供)
        xml_content: Option<String>,
        /// 屏幕宽度
        #[arg(long)]
        width: Option<i32>,
        /// 屏幕高度
        #[arg(long)]
        height: Option<i32>,
    },
    /// 生成UI树的字符串描述
    GenerateTreeString {
        /// XML内容 (从stdin读取如果未提供)
        xml_content: Option<String>,
    },
}

#[derive(Subcommand)]
enum RecognizerCommands {
    /// 根据XML locator查找元素位置
    FindXml {
        /// Locator名称
        locator_name: String,
    },
    /// 根据图像locator查找元素位置
    FindImage {
        /// Locator名称
        locator_name: String,
    },
    /// 根据文本查找元素位置
    FindText {
        /// 文本内容
        text: String,
    },
}

#[derive(Subcommand)]
enum ParserCommands {
    /// 解析TKS脚本文件
    Parse {
        /// 脚本文件路径
        script_path: PathBuf,
    },
    /// 验证脚本语法
    Validate {
        /// 脚本文件路径
        script_path: PathBuf,
    },
    /// 获取语法高亮信息
    Highlight {
        /// 脚本文件路径
        script_path: PathBuf,
    },
}

#[derive(Subcommand)]
enum RunCommands {
    /// 运行单个脚本文件
    Script {
        /// 脚本文件路径
        script_path: PathBuf,
    },
    /// 运行项目中所有脚本
    Project,
    /// 运行指定内容的脚本
    Content {
        /// 脚本内容
        content: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
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
    
    // 对于Fetcher命令，将日志输出到stderr以避免干扰JSON输出
    let is_fetcher_command = matches!(cli.command, Commands::Fetcher { .. });
    
    if is_fetcher_command {
        eprintln!("项目路径: {:?}", project_path);
        if let Some(ref device) = cli.device {
            eprintln!("目标设备: {}", device);
        }
    } else {
        info!("项目路径: {:?}", project_path);
        if let Some(ref device) = cli.device {
            info!("目标设备: {}", device);
        }
    }
    
    match cli.command {
        Commands::Controller { action } => {
            handle_controller_commands(action, cli.device).await
        }
        Commands::Fetcher { action } => {
            handle_fetcher_commands(action, project_path).await
        }
        Commands::Recognizer { action } => {
            handle_recognizer_commands(action, project_path).await
        }
        Commands::Parser { action } => {
            handle_parser_commands(action).await
        }
        Commands::Run { action } => {
            handle_run_commands(action, project_path, cli.device).await
        }
    }
}

async fn handle_controller_commands(action: ControllerCommands, device_id: Option<String>) -> Result<()> {
    let controller = Controller::new(device_id)?;
    
    match action {
        ControllerCommands::Devices => {
            let devices = controller.get_devices()?;
            if devices.is_empty() {
                println!("没有检测到连接的设备");
            } else {
                println!("已连接的设备:");
                for device in devices {
                    println!("  - {}", device);
                }
            }
        }
        ControllerCommands::Capture => {
            let project_path = std::env::current_dir()
                .map_err(|e| TkeError::IoError(e))?;
            controller.capture_ui_state(&project_path).await?;
            println!("UI状态已捕获并保存到workarea");
        }
        ControllerCommands::Tap { x, y } => {
            controller.tap(x, y)?;
            println!("已点击坐标: ({}, {})", x, y);
        }
        ControllerCommands::Swipe { x1, y1, x2, y2, duration } => {
            controller.swipe(x1, y1, x2, y2, duration)?;
            println!("已滑动: ({}, {}) -> ({}, {}) 持续{}ms", x1, y1, x2, y2, duration);
        }
        ControllerCommands::Launch { package, activity } => {
            controller.launch_app(&package, &activity)?;
            println!("已启动应用: {}/{}", package, activity);
        }
        ControllerCommands::Stop { package } => {
            controller.stop_app(&package)?;
            println!("已停止应用: {}", package);
        }
        ControllerCommands::Input { text } => {
            controller.input_text(&text)?;
            println!("已输入文本: {}", text);
        }
        ControllerCommands::Back => {
            controller.back()?;
            println!("已按返回键");
        }
        ControllerCommands::Home => {
            controller.home()?;
            println!("已按主页键");
        }
        ControllerCommands::GetXml => {
            let xml_content = controller.get_ui_xml().await?;
            println!("{}", xml_content);
        }
    }
    
    Ok(())
}

async fn handle_fetcher_commands(action: FetcherCommands, project_path: PathBuf) -> Result<()> {
    let fetcher = LocatorFetcher::new();
    
    match action {
        FetcherCommands::Extract { xml_path } => {
            let elements = fetcher.fetch_elements_from_file(&xml_path)?;
            println!("从XML文件提取了 {} 个元素:", elements.len());
            for element in elements {
                println!("  [{}] {}", element.index, element.to_ai_text());
            }
        }
        FetcherCommands::Current => {
            let xml_path = project_path.join("workarea").join("current_ui_tree.xml");
            if !xml_path.exists() {
                return Err(TkeError::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "当前UI树文件不存在，请先运行 tke controller capture"
                )));
            }
            let elements = fetcher.fetch_elements_from_file(&xml_path)?;
            println!("从当前UI树提取了 {} 个元素:", elements.len());
            for element in elements {
                println!("  [{}] {}", element.index, element.to_ai_text());
            }
        }
        FetcherCommands::Interactive => {
            let xml_path = project_path.join("workarea").join("current_ui_tree.xml");
            let elements = fetcher.fetch_elements_from_file(&xml_path)?;
            let interactive = fetcher.filter_interactive_elements(&elements);
            println!("找到 {} 个可交互元素:", interactive.len());
            for element in interactive {
                println!("  [{}] {}", element.index, element.to_ai_text());
            }
        }
        FetcherCommands::Text => {
            let xml_path = project_path.join("workarea").join("current_ui_tree.xml");
            let elements = fetcher.fetch_elements_from_file(&xml_path)?;
            let text_elements = fetcher.filter_text_elements(&elements);
            println!("找到 {} 个有文本的元素:", text_elements.len());
            for element in text_elements {
                println!("  [{}] {}", element.index, element.to_ai_text());
            }
        }
        FetcherCommands::InferScreenSize { xml_content } => {
            let xml = get_xml_content(xml_content)?;
            if let Some((width, height)) = fetcher.infer_screen_size_from_xml(&xml)? {
                println!("{{\"width\": {}, \"height\": {}}}", width, height);
            } else {
                println!("null");
            }
        }
        FetcherCommands::OptimizeUITree { xml_content } => {
            let xml = get_xml_content(xml_content)?;
            let optimized = fetcher.optimize_ui_tree(&xml)?;
            println!("{}", optimized);
        }
        FetcherCommands::ExtractUIElements { xml_content, width, height } => {
            let xml = get_xml_content(xml_content)?;
            let elements = if let (Some(w), Some(h)) = (width, height) {
                fetcher.extract_ui_elements_with_size(&xml, w, h)?
            } else {
                fetcher.extract_ui_elements(&xml)?
            };
            let elements_json = serde_json::to_string(&elements)?;
            println!("{}", elements_json);
        }
        FetcherCommands::GenerateTreeString { xml_content } => {
            let xml = get_xml_content(xml_content)?;
            let tree_string = fetcher.generate_tree_string(&xml)?;
            println!("{}", tree_string);
        }
    }
    
    Ok(())
}

async fn handle_recognizer_commands(action: RecognizerCommands, project_path: PathBuf) -> Result<()> {
    let recognizer = Recognizer::new(project_path)?;
    
    match action {
        RecognizerCommands::FindXml { locator_name } => {
            let point = recognizer.find_xml_element(&locator_name)?;
            println!("找到XML元素 '{}' 的位置: ({}, {})", locator_name, point.x, point.y);
        }
        RecognizerCommands::FindImage { locator_name } => {
            let point = recognizer.find_image_element(&locator_name)?;
            println!("找到图像元素 '{}' 的位置: ({}, {})", locator_name, point.x, point.y);
        }
        RecognizerCommands::FindText { text } => {
            let point = recognizer.find_element_by_text(&text)?;
            println!("找到文本 '{}' 的位置: ({}, {})", text, point.x, point.y);
        }
    }
    
    Ok(())
}

async fn handle_parser_commands(action: ParserCommands) -> Result<()> {
    let parser = ScriptParser::new();
    
    match action {
        ParserCommands::Parse { script_path } => {
            let script = parser.parse_file(&script_path)?;
            println!("脚本解析成功:");
            println!("  用例ID: {}", script.case_id);
            println!("  脚本名: {}", script.script_name);
            println!("  详情数: {}", script.details.len());
            println!("  步骤数: {}", script.steps.len());
            
            if !script.steps.is_empty() {
                println!("\n步骤列表:");
                for (i, step) in script.steps.iter().enumerate() {
                    println!("  {}. {} (行号: {})", i + 1, step.raw, step.line_number);
                }
            }
        }
        ParserCommands::Validate { script_path } => {
            let script = parser.parse_file(&script_path)?;
            
            // 基本验证
            if script.case_id.is_empty() {
                warn!("脚本缺少用例ID");
            }
            if script.script_name.is_empty() {
                warn!("脚本缺少脚本名");
            }
            if script.steps.is_empty() {
                error!("脚本没有定义任何步骤");
                return Err(TkeError::ScriptParseError("脚本验证失败".to_string()));
            }
            
            println!("脚本验证通过 ✓");
            println!("  用例ID: {}", script.case_id);
            println!("  脚本名: {}", script.script_name);
            println!("  步骤数: {}", script.steps.len());
        }
        ParserCommands::Highlight { script_path } => {
            let content = std::fs::read_to_string(&script_path)
                .map_err(|e| TkeError::IoError(e))?;
            let highlights = parser.get_syntax_highlights(&content);
            
            // 输出JSON格式的语法高亮信息
            let json_output = serde_json::to_string(&highlights)?;
            println!("{}", json_output);
        }
    }
    
    Ok(())
}

fn get_xml_content(xml_content: Option<String>) -> Result<String> {
    match xml_content {
        Some(content) => Ok(content),
        None => {
            // 从stdin读取XML内容
            let mut buffer = String::new();
            std::io::stdin().read_to_string(&mut buffer)?;
            Ok(buffer)
        }
    }
}

async fn handle_run_commands(action: RunCommands, project_path: PathBuf, device_id: Option<String>) -> Result<()> {
    let mut runner = Runner::new(project_path, device_id);
    
    match action {
        RunCommands::Script { script_path } => {
            println!("开始执行脚本: {:?}", script_path);
            let result = runner.run_script_file(&script_path).await?;
            
            println!("\n执行结果:");
            println!("  状态: {}", if result.success { "成功 ✓" } else { "失败 ✗" });
            println!("  用例ID: {}", result.case_id);
            println!("  脚本名: {}", result.script_name);
            println!("  开始时间: {}", result.start_time);
            println!("  结束时间: {}", result.end_time);
            println!("  总步骤数: {}", result.steps.len());
            
            let successful_steps = result.steps.iter().filter(|s| s.success).count();
            println!("  成功步骤: {}", successful_steps);
            
            if let Some(ref error) = result.error {
                println!("  错误信息: {}", error);
            }
            
            // 显示失败的步骤
            let failed_steps: Vec<_> = result.steps.iter().filter(|s| !s.success).collect();
            if !failed_steps.is_empty() {
                println!("\n失败的步骤:");
                for step in failed_steps {
                    println!("  步骤{}: {}", step.index + 1, step.command);
                    if let Some(ref error) = step.error {
                        println!("    错误: {}", error);
                    }
                }
            }
        }
        RunCommands::Project => {
            println!("开始执行项目中的所有脚本...");
            let results = runner.run_project_scripts().await?;
            
            println!("\n项目执行结果:");
            println!("  总脚本数: {}", results.len());
            
            let successful_scripts = results.iter().filter(|r| r.success).count();
            println!("  成功脚本: {}", successful_scripts);
            println!("  失败脚本: {}", results.len() - successful_scripts);
            
            // 显示各个脚本的结果
            for result in results {
                let status = if result.success { "✓" } else { "✗" };
                println!("  {} {} ({})", status, result.script_name, result.case_id);
                if let Some(ref error) = result.error {
                    println!("      错误: {}", error);
                }
            }
        }
        RunCommands::Content { content } => {
            println!("开始执行脚本内容...");
            let result = runner.run_script_content(&content).await?;
            
            println!("执行结果: {}", if result.success { "成功 ✓" } else { "失败 ✗" });
            if let Some(ref error) = result.error {
                println!("错误信息: {}", error);
            }
        }
    }
    
    Ok(())
}
