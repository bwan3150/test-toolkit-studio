// Flow 工作流 - 依次执行一组 .tks 脚本
// web 会话生命周期: 脚本间保留（可测多脚本联动），flow 结束统一销毁
// flow 文件为 TOML:
//   name = "冒烟测试"           # 可省略，默认用文件名
//   scripts = ["a.tks", "b.tks"]  # 按顺序执行，路径相对 flow 文件所在目录
//
// 指定 --log 时产物结构:
// <log>/<时间戳>_flow_<名称>/
//   ├── flow.json             整个 flow 的汇总日志
//   └── <脚本名>/              每个脚本一个子目录（log.json + screenshots/ + page/）

use crate::{Result, TkeError, ExecutionResult};
use super::{RunEvent, RunArtifacts, ScriptRunner};
use super::script_runner::validate_script_path;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Flow 定义文件 (TOML)
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
    device_id: Option<String>,
    element_path: Option<PathBuf>,
}

impl FlowRunner {
    pub fn new(device_id: Option<String>, element_path: Option<PathBuf>) -> Self {
        Self { device_id, element_path }
    }

    /// 执行 flow 文件
    /// log_root: 产物根目录；None 时不保存任何产物
    pub async fn run(
        &self,
        flow_path: &Path,
        log_root: Option<&Path>,
        on_event: &mut dyn FnMut(&RunEvent),
    ) -> Result<FlowResult> {
        // 1. 读取 flow 定义 (TOML)
        let content = std::fs::read_to_string(flow_path).map_err(TkeError::IoError)?;
        let def: FlowDef = toml::from_str(&content)
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

        // 2. flow 产物目录（仅 --log 时创建）
        let artifacts = match log_root {
            Some(root) => Some(RunArtifacts::create(root, &format!("flow_{}", flow_name))?),
            None => None,
        };
        let run_dir_str = artifacts
            .as_ref()
            .map(|a| a.run_dir.to_string_lossy().to_string())
            .unwrap_or_default();

        let flow_dir = flow_path.parent().unwrap_or(Path::new("."));
        let start_time = chrono::Local::now().to_rfc3339();

        on_event(&RunEvent::FlowStart {
            flow: flow_name.clone(),
            total_scripts: def.scripts.len(),
            run_dir: run_dir_str.clone(),
        });

        // 3. 依次执行每个脚本
        let runner = ScriptRunner::new(self.device_id.clone(), self.element_path.clone());
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
            // web 会话由各脚本的 关闭 指令控制（不关则下一个脚本直接复用，可测联动）
            let exec = match validate_script_path(&script_path) {
                Ok(()) => {
                    runner
                        .run(
                            &script_path,
                            artifacts.as_ref().map(|a| a.run_dir.as_path()),
                            on_event,
                        )
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
                    launched_packages: Vec::new(),
                });
            }

            if !success {
                all_success = false;
            }
        }

        // flow 收尾清场（脚本间状态保留以便联动，整个 flow 结束统一还原）:
        //   web     → 销毁浏览器会话
        //   android → 关闭 flow 期间启动过的所有 App（去重）
        match crate::Platform::from_device(self.device_id.as_deref()) {
            crate::Platform::Web => {
                if let Ok(controller) = crate::Controller::new(self.device_id.clone()) {
                    let _ = controller.stop_app("");
                }
            }
            crate::Platform::Android => {
                let mut packages: Vec<String> = Vec::new();
                for r in &results {
                    for p in &r.launched_packages {
                        if !packages.contains(p) {
                            packages.push(p.clone());
                        }
                    }
                }
                if !packages.is_empty() {
                    if let Ok(controller) = crate::Controller::new(self.device_id.clone()) {
                        for p in &packages {
                            let _ = controller.stop_app(p);
                        }
                    }
                }
            }
            crate::Platform::Ios => {}
        }

        // 4. 写入 flow 汇总日志（仅 --log 时）
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

        let log_path = if let Some(a) = &artifacts {
            let path = a.run_dir.join("flow.json");
            let json = serde_json::to_string_pretty(&flow_result).map_err(TkeError::JsonError)?;
            std::fs::write(&path, json).map_err(TkeError::IoError)?;
            path.to_string_lossy().to_string()
        } else {
            String::new()
        };

        on_event(&RunEvent::FlowEnd {
            success: flow_result.success,
            total_scripts: flow_result.total_scripts,
            successful_scripts: flow_result.successful_scripts,
            run_dir: run_dir_str,
            log: log_path,
        });

        Ok(flow_result)
    }
}
