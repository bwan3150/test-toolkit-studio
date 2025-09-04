// Runner模块 - 脚本执行器

use crate::{
    Result, TkeError, TksScript, ScriptParser, ScriptInterpreter, 
    ExecutionResult, StepResult
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH, Instant};
use tracing::{info, error, warn};
use tokio::signal;

pub struct Runner {
    project_path: PathBuf,
    device_id: Option<String>,
    parser: ScriptParser,
    interpreter: Option<ScriptInterpreter>,
    is_running: bool,
    should_stop: bool,
}

impl Runner {
    pub fn new(project_path: PathBuf, device_id: Option<String>) -> Self {
        Self {
            project_path,
            device_id,
            parser: ScriptParser::new(),
            interpreter: None,
            is_running: false,
            should_stop: false,
        }
    }
    
    // 设置设备ID
    pub fn set_device(&mut self, device_id: Option<String>) {
        self.device_id = device_id;
    }
    
    // 运行脚本文件
    pub async fn run_script_file(&mut self, script_path: &PathBuf) -> Result<ExecutionResult> {
        // 解析脚本
        let script = self.parser.parse_file(script_path)?;
        info!("成功解析脚本: {} ({})", script.script_name, script.case_id);
        
        // 执行脚本
        self.run_script(script).await
    }
    
    // 运行脚本内容
    pub async fn run_script_content(&mut self, content: &str) -> Result<ExecutionResult> {
        // 解析脚本
        let script = self.parser.parse(content)?;
        info!("成功解析脚本内容: {} ({})", script.script_name, script.case_id);
        
        // 执行脚本
        self.run_script(script).await
    }
    
    // 运行脚本
    pub async fn run_script(&mut self, script: TksScript) -> Result<ExecutionResult> {
        self.is_running = true;
        self.should_stop = false;
        
        // 初始化解释器
        let mut interpreter = ScriptInterpreter::new(
            self.project_path.clone(), 
            self.device_id.clone()
        )?;
        
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        
        let mut result = ExecutionResult {
            success: true,
            case_id: script.case_id.clone(),
            script_name: script.script_name.clone(),
            start_time: chrono::DateTime::from_timestamp(start_time as i64 / 1000, 0)
                .unwrap_or_default()
                .to_rfc3339(),
            end_time: String::new(),
            steps: Vec::new(),
            error: None,
        };
        
        info!("开始执行脚本: {} (共{}个步骤)", script.script_name, script.steps.len());
        
        // 设置Ctrl+C信号处理
        let stop_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_flag_clone = stop_flag.clone();
        
        tokio::spawn(async move {
            if let Ok(()) = signal::ctrl_c().await {
                stop_flag_clone.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        });
        
        // 执行每个步骤
        for (index, step) in script.steps.iter().enumerate() {
            // 检查是否需要停止
            if self.should_stop || stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                result.success = false;
                result.error = Some("执行被中止".to_string());
                warn!("脚本执行被中止");
                break;
            }
            
            info!("执行步骤 {}/{}: {}", index + 1, script.steps.len(), step.raw);
            
            let step_start = Instant::now();
            let step_result = match interpreter.interpret_step(step).await {
                Ok(()) => StepResult {
                    index,
                    command: step.raw.clone(),
                    success: true,
                    error: None,
                    duration_ms: step_start.elapsed().as_millis() as u64,
                },
                Err(e) => {
                    error!("步骤执行失败: {}", e);
                    result.success = false;
                    result.error = Some(e.to_string());
                    
                    StepResult {
                        index,
                        command: step.raw.clone(),
                        success: false,
                        error: Some(e.to_string()),
                        duration_ms: step_start.elapsed().as_millis() as u64,
                    }
                }
            };
            
            result.steps.push(step_result.clone());
            
            // 如果步骤失败，停止执行
            if !step_result.success {
                break;
            }
            
            // 步骤间短暂延迟
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        
        let end_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        
        result.end_time = chrono::DateTime::from_timestamp(end_time as i64 / 1000, 0)
            .unwrap_or_default()
            .to_rfc3339();
        
        self.is_running = false;
        
        // 保存执行结果
        self.save_result(&result, &script).await?;
        
        if result.success {
            info!("脚本执行完成: {} ({})", script.script_name, 
                  if result.success { "成功" } else { "失败" });
        } else {
            error!("脚本执行失败: {} - {}", script.script_name, 
                   result.error.as_ref().unwrap_or(&"未知错误".to_string()));
        }
        
        Ok(result)
    }
    
    // 停止执行
    pub fn stop(&mut self) {
        info!("收到停止请求");
        self.should_stop = true;
    }
    
    // 检查是否正在运行
    pub fn is_running(&self) -> bool {
        self.is_running
    }
    
    // 保存执行结果
    async fn save_result(&self, result: &ExecutionResult, script: &TksScript) -> Result<()> {
        // 推断case文件夹名称
        let case_folder = if let Some(ref script_path) = script.file_path {
            // 从脚本路径推断case文件夹
            script_path.parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("temp")
        } else {
            "temp"
        };
        
        // 创建result目录
        let result_dir = self.project_path.join("cases").join(case_folder).join("result");
        tokio::fs::create_dir_all(&result_dir).await
            .map_err(|e| TkeError::IoError(e))?;
        
        // 生成结果文件名
        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S").to_string();
        let status = if result.success { "PASS" } else { "FAIL" };
        let script_name = if let Some(ref script_path) = script.file_path {
            script_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown_script")
        } else {
            "unknown_script"
        };
        
        let result_file_name = format!("{}_{}_{}_{}.json", 
                                      case_folder, script_name, timestamp, status);
        let result_path = result_dir.join(result_file_name);
        
        // 保存结果
        let json = serde_json::to_string_pretty(result)
            .map_err(|e| TkeError::JsonError(e))?;
        
        tokio::fs::write(&result_path, json).await
            .map_err(|e| TkeError::IoError(e))?;
        
        info!("执行结果已保存到: {:?}", result_path);
        Ok(())
    }
    
    // 验证脚本
    pub fn validate_script(&self, script: &TksScript) -> Result<()> {
        if script.case_id.is_empty() {
            return Err(TkeError::ScriptParseError("脚本缺少用例ID".to_string()));
        }
        
        if script.steps.is_empty() {
            return Err(TkeError::ScriptParseError("脚本没有定义任何步骤".to_string()));
        }
        
        // 检查启动命令
        let has_launch = script.steps.iter()
            .any(|step| matches!(step.command, crate::TksCommand::Launch));
        
        if !has_launch {
            warn!("脚本没有启动应用的步骤");
        }
        
        Ok(())
    }
    
    // 运行项目中的所有脚本
    pub async fn run_project_scripts(&mut self) -> Result<Vec<ExecutionResult>> {
        let cases_dir = self.project_path.join("cases");
        
        if !cases_dir.exists() {
            return Err(TkeError::InvalidProjectPath(
                "项目目录中没有cases文件夹".to_string()
            ));
        }
        
        let mut results = Vec::new();
        
        // 遍历所有case文件夹
        let mut entries = tokio::fs::read_dir(cases_dir).await
            .map_err(|e| TkeError::IoError(e))?;
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| TkeError::IoError(e))? {
            
            if !entry.file_type().await
                .map_err(|e| TkeError::IoError(e))?.is_dir() {
                continue;
            }
            
            let case_dir = entry.path();
            let script_dir = case_dir.join("script");
            
            if !script_dir.exists() {
                continue;
            }
            
            // 查找.tks文件
            let mut script_entries = tokio::fs::read_dir(script_dir).await
                .map_err(|e| TkeError::IoError(e))?;
            
            while let Some(script_entry) = script_entries.next_entry().await
                .map_err(|e| TkeError::IoError(e))? {
                
                let script_path = script_entry.path();
                if script_path.extension().and_then(|s| s.to_str()) == Some("tks") {
                    info!("运行脚本: {:?}", script_path);
                    
                    match self.run_script_file(&script_path).await {
                        Ok(result) => results.push(result),
                        Err(e) => {
                            error!("脚本执行失败: {:?} - {}", script_path, e);
                            // 继续执行其他脚本
                        }
                    }
                }
            }
        }
        
        info!("项目脚本执行完成，共执行 {} 个脚本", results.len());
        Ok(results)
    }
}