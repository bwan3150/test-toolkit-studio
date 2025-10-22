// Runner模块 - 脚本执行器

// 子模块
mod parser;
mod interpreter;

// 导出
pub use parser::ScriptParser;
pub use interpreter::ScriptInterpreter;

use crate::{
    Result, TkeError, TksScript,
    ExecutionResult, StepResult
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH, Instant};

pub struct Runner {
    project_path: PathBuf,
    device_id: Option<String>,
    pub parser: ScriptParser,  // 为 Toolkit Studio 开放访问
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
    
    // 运行单行脚本指令
    pub async fn run_single_step(&mut self, line: &str) -> Result<StepResult> {
        // 构造一个最小的脚本来解析单行指令
        let minimal_script = format!("用例: 单步执行\n脚本名: 单步\n\n步骤:\n{}", line);

        // 解析脚本
        let script = self.parser.parse(&minimal_script)?;

        if script.steps.is_empty() {
            return Err(TkeError::ScriptParseError("无效的脚本指令".to_string()));
        }

        // 初始化解释器
        let mut interpreter = ScriptInterpreter::new(
            self.project_path.clone(),
            self.device_id.clone()
        )?;

        let step = &script.steps[0];
        let start_time = Instant::now();

        // 执行单个步骤
        match interpreter.interpret_step(step).await {
            Ok(()) => Ok(StepResult {
                index: 0,
                command: line.to_string(),
                success: true,
                error: None,
                duration_ms: start_time.elapsed().as_millis() as u64,
            }),
            Err(e) => Ok(StepResult {
                index: 0,
                command: line.to_string(),
                success: false,
                error: Some(e.to_string()),
                duration_ms: start_time.elapsed().as_millis() as u64,
            })
        }
    }

    // 运行脚本文件
    pub async fn run_script_file(&mut self, script_path: &PathBuf) -> Result<ExecutionResult> {
        // 解析脚本
        let script = self.parser.parse_file(script_path)?;

        // 执行脚本
        self.run_script(script).await
    }
    
    // 运行脚本内容
    pub async fn run_script_content(&mut self, content: &str) -> Result<ExecutionResult> {
        // 解析脚本
        let script = self.parser.parse(content)?;

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

        // 执行每个步骤
        for (index, step) in script.steps.iter().enumerate() {
            // 检查是否需要停止
            if self.should_stop {
                result.success = false;
                result.error = Some("执行被中止".to_string());
                break;
            }

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

        Ok(result)
    }
    
    // 停止执行
    pub fn stop(&mut self) {
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
                    // 执行脚本，失败也继续执行其他脚本
                    if let Ok(result) = self.run_script_file(&script_path).await {
                        results.push(result);
                    }
                }
            }
        }

        Ok(results)
    }
}