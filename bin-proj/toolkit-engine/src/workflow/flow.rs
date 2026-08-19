// Flow 工作流 - 依次执行一组 .tks 脚本
// web 会话生命周期: 脚本间保留（可测多脚本联动），flow 结束统一销毁
// flow 文件为 TOML:
//   name = "冒烟测试"             # 可省略，默认用文件名
//   scripts = ["a.tks", "b.tks"]  # 按顺序执行，路径相对 flow 文件所在目录
//
// 跨设备/跨平台测试：每项可单独指定设备（不指定则沿用全局 -d）——
//   scripts = [
//     { path = "a-设置开关.tks", device = "phoneA" },   # 在 A 手机上改设置
//     { path = "b-查看生效.tks", device = "phoneB" },   # 到 B 手机上验收
//   ]
// 一个 .tks = 一个设备上的一段流程（两件套自包含，INV-7），跨设备 = 把几段串成 flow。
// 收尾按设备分组清场（每台都要清，只清全局那台会留孤儿）。
//
// 指定 --log 时产物结构:
// <log>/<时间戳>_flow_<名称>/
//   ├── flow.json             整个 flow 的汇总日志
//   └── <脚本名>/              每个脚本一个子目录（log.json + screenshots/ + page/）

use crate::{Result, TkeError, ExecutionResult, Params};
use super::{RunEvent, RunArtifacts, ScriptRunner};
use super::script_runner::validate_script_path;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Flow 定义文件 (TOML)
#[derive(Debug, Deserialize)]
pub struct FlowDef {
    /// flow 名称（缺省用文件名）
    pub name: Option<String>,
    /// 依次执行的 .tks 脚本
    pub scripts: Vec<FlowScript>,
}

/// flow 里的一项脚本。两种写法：
///   "a.tks"                              → 用全局 -d
///   { path = "a.tks", device = "手机A" }  → 指定该段跑在哪个设备/平台上
///
/// **跨设备测试就是这么表达的**：一个 .tks = 一个设备上的一段流程（两件套自包含，INV-7），
/// 跨设备 = 把几段串成 flow。例：手机A 改设置 → 手机B 看是否生效；web 后台下发 → 手机端验收。
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum FlowScript {
    /// 简写：只给路径
    Path(String),
    /// 完整：路径 + 该段的设备
    Detailed {
        path: String,
        /// 设备 ID（Android 序列号 / `web` / iOS UDID）；不给则沿用全局 -d
        device: Option<String>,
    },
}

impl FlowScript {
    pub fn path(&self) -> &str {
        match self {
            Self::Path(p) => p,
            Self::Detailed { path, .. } => path,
        }
    }

    pub fn device(&self) -> Option<&str> {
        match self {
            Self::Path(_) => None,
            Self::Detailed { device, .. } => device.as_deref(),
        }
    }
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
    params: Arc<Params>,
    /// 自愈钩子工厂（AI 辅助驾驶）：透传给每个脚本的 ScriptRunner，见 script_runner::HealerFactory
    healer_factory: Option<super::script_runner::HealerFactory>,
}

impl FlowRunner {
    pub fn new(params: Arc<Params>) -> Self {
        Self { params, healer_factory: None }
    }

    /// 注入自愈钩子工厂（builder 式；AI 辅助驾驶）
    pub fn with_healer_factory(mut self, factory: super::script_runner::HealerFactory) -> Self {
        self.healer_factory = Some(factory);
        self
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

        // 设备必须有着落：要么全局 -d，要么该项自带 device。
        // 不校验的话没设备的那一项会被当成 Android，web 用例只得到一句「adb 缺失」。
        if self.params.device().is_none() {
            let missing: Vec<&str> = def
                .scripts
                .iter()
                .filter(|s| s.device().is_none())
                .map(|s| s.path())
                .collect();
            if !missing.is_empty() {
                return Err(TkeError::InvalidArgument(format!(
                    "这些脚本没有设备可用：{}。给 flow 加全局 -d/--device，或在对应项写 {{ path = \"…\", device = \"…\" }}",
                    missing.join("、")
                )));
            }
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
        // runner 按脚本构造：每项可带自己的 device（跨设备 flow），不带则沿用全局 -d。
        // web 会话按设备 id 分文件存，不同设备各自独立，互不干扰。
        let make_runner = |device: Option<&str>| {
            let params = match device {
                Some(d) => Arc::new(self.params.with_device(Some(d.to_string()))),
                None => self.params.clone(),
            };
            let mut r = ScriptRunner::new(params);
            if let Some(factory) = &self.healer_factory {
                r = r.with_healer_factory(factory.clone());
            }
            r
        };
        let mut results: Vec<ExecutionResult> = Vec::new();
        let mut all_success = true;
        // 每个脚本跑在哪个设备上、在那台上启动过哪些 App —— 收尾要按设备分别清场，
        // 否则跨设备 flow 只会清全局那台，其余设备留下孤儿浏览器/App
        let mut used: Vec<(Option<String>, Vec<String>)> = Vec::new();

        for (index, item) in def.scripts.iter().enumerate() {
            // 相对路径基于 flow 文件所在目录解析
            let script_path = {
                let p = PathBuf::from(item.path());
                if p.is_absolute() { p } else { flow_dir.join(p) }
            };
            let runner = make_runner(item.device());

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
                used.push((
                    item.device().map(String::from).or_else(|| self.params.device()),
                    r.launched_packages.clone(),
                ));
                results.push(r);
            } else {
                // 执行器级错误（如脚本不存在）也记录进汇总
                results.push(ExecutionResult {
                    success: false,
                    case_id: String::new(),
                    script_name: item.path().to_string(),
                    start_time: chrono::Local::now().to_rfc3339(),
                    end_time: chrono::Local::now().to_rfc3339(),
                    steps: Vec::new(),
                    error,
                    script_path: Some(script_path.to_string_lossy().to_string()),
                    run_dir: None,
                    launched_packages: Vec::new(),
                    device: item.device().map(String::from).or_else(|| self.params.device()),
                });
            }

            if !success {
                all_success = false;
            }
        }

        // flow 收尾清场（脚本间状态保留以便联动，整个 flow 结束统一还原）:
        //   web         → 销毁浏览器会话
        //   android/ios → 关闭 flow 期间启动过的所有 App（去重）; ios 再销毁 WDA 会话
        // **按设备分组**：跨设备 flow 每台都要清，只清全局那台会留下孤儿。
        let mut by_device: Vec<(Option<String>, Vec<String>)> = Vec::new();
        for (dev, pkgs) in used {
            match by_device.iter_mut().find(|(d, _)| *d == dev) {
                Some(entry) => {
                    for p in pkgs {
                        if !entry.1.contains(&p) {
                            entry.1.push(p);
                        }
                    }
                }
                None => by_device.push((dev, pkgs)),
            }
        }
        // 一个脚本都没成功跑过时，仍按全局设备清一次（可能已建过会话）
        if by_device.is_empty() {
            by_device.push((self.params.device(), Vec::new()));
        }

        for (device, packages) in by_device {
            let platform = crate::Platform::from_device(device.as_deref());
            let controller = match crate::Controller::new(device.clone()) {
                Ok(c) => c,
                Err(_) => continue,
            };
            match platform {
                crate::Platform::Web => {
                    let _ = controller.stop_app("");
                }
                crate::Platform::Android | crate::Platform::Ios => {
                    for p in &packages {
                        let _ = controller.stop_app(p);
                    }
                    // iOS 真机：App 关完后销毁 WDA 会话。
                    // 模拟器走的是 idb，没有"会话"这回事——空参对它是空操作，无害
                    if platform == crate::Platform::Ios {
                        let _ = controller.stop_app("");
                    }
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// flow 的两种写法都能解析：纯路径（沿用全局 -d）与带设备（跨设备 flow）
    #[test]
    fn flow_def_accepts_both_plain_and_device_forms() {
        let toml_src = r#"
name = "开关下发验证"
scripts = [
  "预置.tks",
  { path = "a-设置开关.tks", device = "phoneA" },
  { path = "b-查看生效.tks", device = "web" },
  { path = "c-无设备.tks" },
]
"#;
        let def: FlowDef = toml::from_str(toml_src).expect("flow 解析失败");
        assert_eq!(def.name.as_deref(), Some("开关下发验证"));
        assert_eq!(def.scripts.len(), 4);

        // 纯字符串 → 无设备，沿用全局
        assert_eq!(def.scripts[0].path(), "预置.tks");
        assert_eq!(def.scripts[0].device(), None);

        // 带设备
        assert_eq!(def.scripts[1].path(), "a-设置开关.tks");
        assert_eq!(def.scripts[1].device(), Some("phoneA"));
        assert_eq!(def.scripts[2].device(), Some("web"));

        // 表形式但省略 device → 同样沿用全局
        assert_eq!(def.scripts[3].path(), "c-无设备.tks");
        assert_eq!(def.scripts[3].device(), None);
    }

    /// 老 flow 文件（纯字符串列表）必须照常可用——不许因为加了设备维度就破坏兼容
    #[test]
    fn flow_def_backward_compatible_with_plain_list() {
        let def: FlowDef = toml::from_str(r#"scripts = ["a.tks", "b.tks"]"#).expect("解析失败");
        assert_eq!(def.name, None);
        assert!(def.scripts.iter().all(|s| s.device().is_none()));
        assert_eq!(def.scripts.iter().map(|s| s.path()).collect::<Vec<_>>(), vec!["a.tks", "b.tks"]);
    }
}
