// ToolkitEngine (tke) CLI Main Entrancee

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::io::Read;
use tracing::{info, error, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use tke::{
    Controller, LocatorFetcher, Recognizer, ScriptParser,
    Runner, Result, TkeError, ocr
};

#[derive(Parser)]
#[command(name = "tke")]
#[command(about = "Toolkit Engine")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Phone Device ID (optional
    #[arg(short, long)]
    device: Option<String>,
    
    /// Proj Path
    #[arg(short, long)]
    project: Option<PathBuf>,
    
    /// verbose lwvel
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
    /// Recognizer - recongnize image element from large image
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
    /// 运行指定内容的脚本 (Toolkit Studio专用，返回JSON)
    Content {
        /// 脚本内容
        content: String,
    },
    /// 执行单个步骤 (Toolkit Studio专用)
    Step {
        /// 脚本内容
        content: String,
        /// 要执行的步骤索引 (0开始)
        step_index: usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
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

        if is_json_output_command {
            // JSON输出命令完全静默,确保stdout只有纯JSON
        } else {
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
        Commands::Adb { args } => {
            handle_adb_command(args, cli.device).await
        }
        Commands::Ocr { image, online, url, lang } => {
            handle_ocr_command(image, online, url, lang).await
        }
    }
}

async fn handle_controller_commands(action: ControllerCommands, device_id: Option<String>) -> Result<()> {
    let controller = Controller::new(device_id)?;

    match action {
        ControllerCommands::Devices => {
            let devices = controller.get_devices()?;
            let json = serde_json::json!({
                "devices": devices
            });
            println!("{}", serde_json::to_string(&json)?);
        }
        ControllerCommands::Capture => {
            let project_path = std::env::current_dir()
                .map_err(|e| TkeError::IoError(e))?;
            controller.capture_ui_state(&project_path).await?;

            let screenshot_path = project_path.join("workarea").join("current_screenshot.png");
            let xml_path = project_path.join("workarea").join("current_ui_tree.xml");

            let json = serde_json::json!({
                "success": true,
                "screenshot": screenshot_path.to_string_lossy(),
                "xml": xml_path.to_string_lossy()
            });
            println!("{}", serde_json::to_string(&json)?);
        }
        ControllerCommands::Tap { x, y } => {
            controller.tap(x, y)?;
            let json = serde_json::json!({
                "success": true,
                "x": x,
                "y": y
            });
            println!("{}", serde_json::to_string(&json)?);
        }
        ControllerCommands::Swipe { x1, y1, x2, y2, duration } => {
            controller.swipe(x1, y1, x2, y2, duration)?;
            let json = serde_json::json!({
                "success": true,
                "from": {"x": x1, "y": y1},
                "to": {"x": x2, "y": y2},
                "duration": duration
            });
            println!("{}", serde_json::to_string(&json)?);
        }
        ControllerCommands::Launch { package, activity } => {
            controller.launch_app(&package, &activity)?;
            let json = serde_json::json!({
                "success": true,
                "package": package,
                "activity": activity
            });
            println!("{}", serde_json::to_string(&json)?);
        }
        ControllerCommands::Stop { package } => {
            controller.stop_app(&package)?;
            let json = serde_json::json!({
                "success": true,
                "package": package
            });
            println!("{}", serde_json::to_string(&json)?);
        }
        ControllerCommands::Input { text } => {
            controller.input_text(&text)?;
            let json = serde_json::json!({
                "success": true,
                "text": text
            });
            println!("{}", serde_json::to_string(&json)?);
        }
        ControllerCommands::Back => {
            controller.back()?;
            let json = serde_json::json!({
                "success": true
            });
            println!("{}", serde_json::to_string(&json)?);
        }
        ControllerCommands::Home => {
            controller.home()?;
            let json = serde_json::json!({
                "success": true
            });
            println!("{}", serde_json::to_string(&json)?);
        }
    }

    Ok(())
}

async fn handle_fetcher_commands(action: FetcherCommands, project_path: PathBuf) -> Result<()> {
    let fetcher = LocatorFetcher::new();

    match action {
        FetcherCommands::Extract { xml_path } => {
            let elements = fetcher.fetch_elements_from_file(&xml_path)?;
            let json = serde_json::to_string(&elements)?;
            println!("{}", json);
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
            let json = serde_json::to_string(&elements)?;
            println!("{}", json);
        }
        FetcherCommands::Interactive => {
            let xml_path = project_path.join("workarea").join("current_ui_tree.xml");
            let elements = fetcher.fetch_elements_from_file(&xml_path)?;
            let interactive = fetcher.filter_interactive_elements(&elements);
            let json = serde_json::to_string(&interactive)?;
            println!("{}", json);
        }
        FetcherCommands::Text => {
            let xml_path = project_path.join("workarea").join("current_ui_tree.xml");
            let elements = fetcher.fetch_elements_from_file(&xml_path)?;
            let text_elements = fetcher.filter_text_elements(&elements);
            let json = serde_json::to_string(&text_elements)?;
            println!("{}", json);
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
            let json = serde_json::json!({
                "success": true,
                "locator": locator_name,
                "x": point.x,
                "y": point.y
            });
            println!("{}", serde_json::to_string(&json)?);
        }
        RecognizerCommands::FindImage { locator_name } => {
            let point = recognizer.find_image_element(&locator_name)?;
            let json = serde_json::json!({
                "success": true,
                "locator": locator_name,
                "x": point.x,
                "y": point.y
            });
            println!("{}", serde_json::to_string(&json)?);
        }
        RecognizerCommands::FindText { text } => {
            let point = recognizer.find_element_by_text(&text)?;
            let json = serde_json::json!({
                "success": true,
                "text": text,
                "x": point.x,
                "y": point.y
            });
            println!("{}", serde_json::to_string(&json)?);
        }
    }

    Ok(())
}

async fn handle_parser_commands(action: ParserCommands) -> Result<()> {
    let parser = ScriptParser::new();
    
    match action {
        ParserCommands::Parse { script_path } => {
            let script = parser.parse_file(&script_path)?;
            
            // 输出JSON格式的解析结果
            let json_output = serde_json::json!({
                "success": true,
                "case_id": script.case_id,
                "script_name": script.script_name,
                "details": script.details,
                "steps": script.steps.iter().map(|step| serde_json::json!({
                    "command": step.raw,
                    "line_number": step.line_number,
                    "command_type": step.command,
                    "params": step.params
                })).collect::<Vec<_>>()
            });
            
            println!("{}", serde_json::to_string(&json_output)?);
        }
        ParserCommands::Validate { script_path } => {
            let script = parser.parse_file(&script_path)?;

            // 基本验证
            let mut warnings = Vec::new();
            if script.case_id.is_empty() {
                warnings.push("脚本缺少用例ID");
            }
            if script.script_name.is_empty() {
                warnings.push("脚本缺少脚本名");
            }
            if script.steps.is_empty() {
                let json = serde_json::json!({
                    "valid": false,
                    "error": "脚本没有定义任何步骤"
                });
                println!("{}", serde_json::to_string(&json)?);
                return Err(TkeError::ScriptParseError("脚本验证失败".to_string()));
            }

            let json = serde_json::json!({
                "valid": true,
                "case_id": script.case_id,
                "script_name": script.script_name,
                "steps_count": script.steps.len(),
                "warnings": warnings
            });
            println!("{}", serde_json::to_string(&json)?);
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
    let mut runner = Runner::new(project_path.clone(), device_id.clone());
    
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
            // 为 Toolkit Studio 专用：返回 JSON 格式的结果
            let result = runner.run_script_content(&content).await?;
            
            // 输出 JSON 格式的执行结果
            let json_result = serde_json::json!({
                "success": result.success,
                "case_id": result.case_id,
                "script_name": result.script_name,
                "start_time": result.start_time,
                "end_time": result.end_time,
                "error": result.error,
                "steps": result.steps.iter().map(|step| serde_json::json!({
                    "index": step.index,
                    "command": step.command,
                    "success": step.success,
                    "error": step.error,
                    "duration_ms": step.duration_ms
                })).collect::<Vec<_>>()
            });
            
            println!("{}", serde_json::to_string(&json_result)?);
        }
        RunCommands::Step { content, step_index } => {
            // 为 Toolkit Studio 专用：执行单个步骤并返回 JSON 结果
            let script = runner.parser.parse(&content)?;
            
            if step_index >= script.steps.len() {
                let error_result = serde_json::json!({
                    "success": false,
                    "error": format!("步骤索引超出范围: {} >= {}", step_index, script.steps.len()),
                    "step_index": step_index
                });
                println!("{}", serde_json::to_string(&error_result)?);
                return Ok(());
            }
            
            let step = &script.steps[step_index];
            
            // 初始化解释器
            let mut interpreter = tke::ScriptInterpreter::new(
                project_path.clone(),
                device_id
            )?;
            
            let start_time = std::time::Instant::now();
            
            // 执行单个步骤
            let step_result = match interpreter.interpret_step(step).await {
                Ok(()) => serde_json::json!({
                    "success": true,
                    "step_index": step_index,
                    "command": step.raw,
                    "line_number": step.line_number,
                    "duration_ms": start_time.elapsed().as_millis(),
                    "error": null
                }),
                Err(e) => serde_json::json!({
                    "success": false,
                    "step_index": step_index,
                    "command": step.raw,
                    "line_number": step.line_number,
                    "duration_ms": start_time.elapsed().as_millis(),
                    "error": e.to_string()
                })
            };
            
            println!("{}", serde_json::to_string(&step_result)?);
        }
    }
    
    Ok(())
}

// OCR 命令处理
async fn handle_ocr_command(
    image_path: PathBuf,
    online: bool,
    url: Option<String>,
    lang: String,
) -> Result<()> {
    let image_data = std::fs::read(&image_path)
        .map_err(|e| TkeError::IoError(e))?;

    let result = if online {
        let url = url.ok_or_else(|| {
            TkeError::InvalidArgument("在线模式需要提供 --url 参数".to_string())
        })?;
        ocr(&image_data, true, &url).await
    } else {
        ocr(&image_data, false, &lang).await
    };

    match result {
        Ok(ocr_result) => {
            let json = serde_json::to_string(&ocr_result)
                .map_err(|e| TkeError::JsonError(e))?;
            println!("{}", json);
            Ok(())
        }
        Err(e) => {
            let error_json = serde_json::json!({
                "error": e.to_string()
            });
            println!("{}", serde_json::to_string(&error_json).unwrap());
            Err(TkeError::OcrError(e.to_string()))
        }
    }
}

// ADB 直通命令处理
async fn handle_adb_command(args: Vec<String>, device_id: Option<String>) -> Result<()> {
    use std::process::Command;
    
    // 创建 AdbManager 来获取 ADB 路径，静默模式
    let adb_manager = match tke::AdbManager::new() {
        Ok(manager) => manager,
        Err(_) => {
            // 如果获取ADB失败，直接退出，不输出错误信息
            std::process::exit(1);
        }
    };
    
    // 构建 ADB 命令
    let mut cmd = Command::new(adb_manager.adb_path());
    
    // 如果指定了设备ID，添加 -s 参数
    if let Some(ref device) = device_id {
        cmd.arg("-s").arg(device);
    }
    
    // 添加用户提供的参数
    cmd.args(&args);
    
    // 执行命令并继承标准输入输出（完全透明传递）
    // 直接传递退出码，不做任何处理
    let status = cmd
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .unwrap_or_else(|_| {
            // 如果执行失败，静默退出
            std::process::exit(1)
        });
    
    // 直接使用ADB的退出码退出，不做任何额外处理
    std::process::exit(status.code().unwrap_or(1));
}
