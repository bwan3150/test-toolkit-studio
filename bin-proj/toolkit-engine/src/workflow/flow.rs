// Flow 工作流 - 依次执行一组 .tks 脚本
// flow 文件为 JSON: { "name": "冒烟测试", "scripts": ["a.tks", "b.tks", ...] }
// 路径相对于 flow 文件所在目录解析；产物结构:
// runs/<时间戳>_flow_<名称>/
//   ├── flow.json           整个 flow 的汇总日志
//   └── <脚本名>/            每个脚本一个子目录（结构同 script 运行）

use crate::{Result, TkeError, ExecutionResult};
use super::{RunEvent, RunArtifacts, ScriptRunner};
use super::script_runner::validate_script_path;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Flow 定义文件
#[derive(Debug, Deserialize)]
pub struct FlowDef {
    /// flow 名称（缺省用文件名）
    pub name: Option<String>,
    /// 依次执行的 .tks 脚本路径列表
    pub scripts: Vec<String>,
}

/// Flow 执行汇总结果
#[derive(Debug, Serialize)]
pub struct FlowResult {
    pub success: bool,
    pub flow: String,
    pub flow_path: String,
    pub start_time: String,
    pub end_time: String,
    pub total_scripts: usize,
    pub successful_scripts: usize,
    pub run_dir: String,
    pub scripts: Vec<ExecutionResult>,
}

/// Flow 运行器
pub struct FlowRunner {
    project_path: PathBuf,
    device_id: Option<String>,
}

impl FlowRunner {
    pub fn new(project_path: PathBuf, device_id: Option<String>) -> Self {
        Self { project_path, device_id }
    }

    /// 执行 flow 文件
    pub async fn run(
        &self,
        flow_path: &Path,
        on_event: &mut dyn FnMut(&RunEvent),
    ) -> Result<FlowResult> {
        // 1. 读取 flow 定义
        let content = std::fs::read_to_string(flow_path).map_err(TkeError::IoError)?;
        let def: FlowDef = serde_json::from_str(&content)
            .map_err(|e| TkeError::ScriptParseError(format!("flow 文件解析失败: {}", e)))?;

        if def.scripts.is_empty() {
            return Err(TkeError::InvalidArgument("flow 中没有任何脚本".to_string()));
        }

        let flow_name = def.name.clone().unwrap_or_else(|| {
            flow_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("flow")
                .to_string()
        });

        // 2. 创建 flow 产物目录
        let artifacts =
            RunArtifacts::create(&self.project_path, None, &format!("flow_{}", flow_name))?;
        let run_dir_str = artifacts.run_dir.to_string_lossy().to_string();

        let flow_dir = flow_path.parent().unwrap_or(Path::new("."));
        let start_time = chrono::Local::now().to_rfc3339();

        on_event(&RunEvent::FlowStart {
            flow: flow_name.clone(),
            total_scripts: def.scripts.len(),
            run_dir: run_dir_str.clone(),
        });

        // 3. 依次执行每个脚本
        let runner = ScriptRunner::new(self.project_path.clone(), self.device_id.clone());
        let mut results: Vec<ExecutionResult> = Vec::new();
        let mut all_success = true;

        for (index, script_rel) in def.scripts.iter().enumerate() {
            // 相对路径基于 flow 文件所在目录解析
            let script_path = {
                let p = PathBuf::from(script_rel);
                if p.is_absolute() { p } else { flow_dir.join(p) }
            };

            on_event(&RunEvent::ScriptStart {
                index,
                script: script_path.to_string_lossy().to_string(),
            });

            // 校验失败/执行异常不中断 flow，记录后继续下一个脚本
            let exec = match validate_script_path(&script_path) {
                Ok(()) => {
                    runner
                        .run(&script_path, Some(&artifacts.run_dir), on_event)
                        .await
                }
                Err(e) => Err(e),
            };

            let (success, error) = match &exec {
                Ok(r) => (r.success, r.error.clone()),
                Err(e) => (false, Some(e.to_string())),
            };

            on_event(&RunEvent::ScriptEnd {
                index,
                script: script_path.to_string_lossy().to_string(),
                success,
                error: error.clone(),
            });

            if let Ok(r) = exec {
                results.push(r);
            } else {
                // 执行器级错误（如脚本不存在）也记录进汇总
                results.push(ExecutionResult {
                    success: false,
                    case_id: String::new(),
                    script_name: script_rel.clone(),
                    start_time: chrono::Local::now().to_rfc3339(),
                    end_time: chrono::Local::now().to_rfc3339(),
                    steps: Vec::new(),
                    error,
                    script_path: Some(script_path.to_string_lossy().to_string()),
                    run_dir: None,
                });
            }

            if !success {
                all_success = false;
            }
        }

        // 4. 写入 flow 汇总日志
        let flow_result = FlowResult {
            success: all_success,
            flow: flow_name,
            flow_path: flow_path.to_string_lossy().to_string(),
            start_time,
            end_time: chrono::Local::now().to_rfc3339(),
            total_scripts: results.len(),
            successful_scripts: results.iter().filter(|r| r.success).count(),
            run_dir: run_dir_str.clone(),
            scripts: results,
        };

        let log_path = artifacts.run_dir.join("flow.json");
        let json = serde_json::to_string_pretty(&flow_result).map_err(TkeError::JsonError)?;
        std::fs::write(&log_path, json).map_err(TkeError::IoError)?;

        on_event(&RunEvent::FlowEnd {
            success: flow_result.success,
            total_scripts: flow_result.total_scripts,
            successful_scripts: flow_result.successful_scripts,
            run_dir: run_dir_str,
            log: log_path.to_string_lossy().to_string(),
        });

        Ok(flow_result)
    }
}
