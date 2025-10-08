// TKE (Toolkit Engine) 集成模块

use std::path::{Path, PathBuf};
use std::process::Command;
use serde_json::Value;
use tracing::{info, debug, warn, error};

use crate::core::error::{Result, AiTesterError};
use crate::models::{TestAction, UIElement, DeviceInfo};

/// TKE 执行器
pub struct TkeExecutor {
    tke_path: PathBuf,
    project_path: PathBuf,
    device_id: String,
}

impl TkeExecutor {
    pub fn new(project_path: impl AsRef<Path>, device_id: String) -> Result<Self> {
        // TKE 可执行文件路径（相对于项目根目录）
        let tke_path = project_path.as_ref()
            .parent()
            .ok_or_else(|| AiTesterError::FileSystemError("无法获取项目父目录".to_string()))?
            .join("toolkit-engine")
            .join("target")
            .join("release")
            .join("tke");

        if !tke_path.exists() {
            error!("TKE 可执行文件不存在: {:?}", tke_path);
            return Err(AiTesterError::TkeError(format!("TKE not found at {:?}", tke_path)));
        }

        Ok(Self {
            tke_path,
            project_path: project_path.as_ref().to_path_buf(),
            device_id,
        })
    }

    /// 执行 TKE 命令
    fn execute_tke(&self, args: &[&str]) -> Result<String> {
        let mut cmd = Command::new(&self.tke_path);

        // 添加通用参数
        cmd.arg("--device").arg(&self.device_id)
            .arg("--project").arg(&self.project_path);

        // 添加具体命令参数
        for arg in args {
            cmd.arg(arg);
        }

        debug!("执行 TKE 命令: {:?}", cmd);

        let output = cmd.output()
            .map_err(|e| AiTesterError::TkeError(format!("执行TKE失败: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("TKE 执行失败: {}", stderr);
            return Err(AiTesterError::TkeError(format!("TKE failed: {}", stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    }

    /// 获取设备列表
    pub fn get_devices(&self) -> Result<Vec<String>> {
        info!("获取设备列表");
        let output = self.execute_tke(&["controller", "devices"])?;

        // 解析设备列表（假设输出为JSON格式）
        let devices: Vec<String> = serde_json::from_str(&output)
            .map_err(|e| AiTesterError::ParseError(format!("解析设备列表失败: {}", e)))?;

        Ok(devices)
    }

    /// 截图并获取XML
    pub fn capture_screen(&self) -> Result<(PathBuf, PathBuf)> {
        info!("截图并获取XML");
        self.execute_tke(&["controller", "capture"])?;

        // 返回workarea中的文件路径
        let workarea = self.project_path.join("workarea");
        let screenshot = workarea.join("current_screenshot.png");
        let xml = workarea.join("current_ui_tree.xml");

        if !screenshot.exists() || !xml.exists() {
            return Err(AiTesterError::FileSystemError(
                "截图或XML文件不存在".to_string()
            ));
        }

        Ok((screenshot, xml))
    }

    /// 提取UI元素
    pub fn extract_elements(&self) -> Result<Vec<UIElement>> {
        info!("提取UI元素");
        let output = self.execute_tke(&["fetcher", "extract", "--format", "json"])?;

        let json: Value = serde_json::from_str(&output)
            .map_err(|e| AiTesterError::ParseError(format!("解析UI元素失败: {}", e)))?;

        let elements = json.get("elements")
            .and_then(|v| v.as_array())
            .ok_or_else(|| AiTesterError::ParseError("响应中缺少elements字段".to_string()))?;

        let mut ui_elements = Vec::new();
        for (index, elem) in elements.iter().enumerate() {
            ui_elements.push(UIElement {
                index,
                class: elem.get("class").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                text: elem.get("text").and_then(|v| v.as_str()).map(String::from),
                resource_id: elem.get("resource_id").and_then(|v| v.as_str()).map(String::from),
                content_desc: elem.get("content_desc").and_then(|v| v.as_str()).map(String::from),
                bounds: Self::parse_bounds(elem.get("bounds")),
                clickable: elem.get("clickable").and_then(|v| v.as_bool()).unwrap_or(false),
                scrollable: elem.get("scrollable").and_then(|v| v.as_bool()).unwrap_or(false),
            });
        }

        Ok(ui_elements)
    }

    /// 执行测试操作
    pub fn execute_action(&self, action: &TestAction) -> Result<()> {
        info!("执行操作: {:?}", action);

        match action {
            TestAction::Tap { x, y } => {
                self.execute_tke(&["controller", "tap", &x.to_string(), &y.to_string()])?;
            }
            TestAction::Swipe { x1, y1, x2, y2, duration } => {
                self.execute_tke(&[
                    "controller", "swipe",
                    &x1.to_string(), &y1.to_string(),
                    &x2.to_string(), &y2.to_string(),
                    "--duration", &duration.to_string()
                ])?;
            }
            TestAction::Input { text } => {
                self.execute_tke(&["controller", "input", text])?;
            }
            TestAction::Back => {
                self.execute_tke(&["controller", "key", "back"])?;
            }
            TestAction::Home => {
                self.execute_tke(&["controller", "key", "home"])?;
            }
            TestAction::Wait { seconds } => {
                std::thread::sleep(std::time::Duration::from_secs(*seconds as u64));
            }
            TestAction::Screenshot => {
                self.capture_screen()?;
            }
        }

        Ok(())
    }

    /// 保存测试脚本
    pub fn save_script(&self, test_name: &str, actions: &[TestAction]) -> Result<PathBuf> {
        info!("保存测试脚本: {}", test_name);

        // 构建 .tks 脚本内容
        let mut script = String::new();
        script.push_str(&format!("# AI自动生成的测试脚本: {}\n", test_name));
        script.push_str(&format!("# 生成时间: {}\n\n", chrono::Utc::now()));

        for (i, action) in actions.iter().enumerate() {
            script.push_str(&format!("# Step {}\n", i + 1));
            script.push_str(&self.action_to_tks(action));
            script.push_str("\n");
        }

        // 保存到测试脚本目录
        let scripts_dir = self.project_path.join("test_scripts");
        std::fs::create_dir_all(&scripts_dir)
            .map_err(|e| AiTesterError::FileSystemError(format!("创建脚本目录失败: {}", e)))?;

        let script_path = scripts_dir.join(format!("{}.tks", test_name));
        std::fs::write(&script_path, script)
            .map_err(|e| AiTesterError::FileSystemError(format!("保存脚本失败: {}", e)))?;

        Ok(script_path)
    }

    /// 将测试操作转换为TKS脚本
    fn action_to_tks(&self, action: &TestAction) -> String {
        match action {
            TestAction::Tap { x, y } => format!("tap({}, {})", x, y),
            TestAction::Swipe { x1, y1, x2, y2, duration } => {
                format!("swipe({}, {}, {}, {}, {})", x1, y1, x2, y2, duration)
            }
            TestAction::Input { text } => format!("input(\"{}\")", text),
            TestAction::Back => "key(back)".to_string(),
            TestAction::Home => "key(home)".to_string(),
            TestAction::Wait { seconds } => format!("wait({})", seconds),
            TestAction::Screenshot => "screenshot()".to_string(),
        }
    }

    /// 解析bounds数组
    fn parse_bounds(bounds: Option<&Value>) -> [i32; 4] {
        if let Some(bounds) = bounds {
            if let Some(arr) = bounds.as_array() {
                if arr.len() == 4 {
                    return [
                        arr[0].as_i64().unwrap_or(0) as i32,
                        arr[1].as_i64().unwrap_or(0) as i32,
                        arr[2].as_i64().unwrap_or(0) as i32,
                        arr[3].as_i64().unwrap_or(0) as i32,
                    ];
                }
            }
        }
        [0, 0, 0, 0]
    }
}