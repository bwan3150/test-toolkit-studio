// TKE 命令执行器

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;
use tokio::process::Command as TokioCommand;
use tracing::{debug, error, info};

use super::{ActionResult, CaptureResult, OcrResult, UiElementsResult};

/// TKE 执行器
pub struct TkeExecutor {
    /// TKE 可执行文件路径
    tke_path: PathBuf,
    /// 项目路径
    project_path: PathBuf,
    /// 设备 ID
    device_id: Option<String>,
}

impl TkeExecutor {
    /// 创建新的 TKE 执行器
    pub fn new(
        tke_path: impl Into<PathBuf>,
        project_path: impl Into<PathBuf>,
        device_id: Option<String>,
    ) -> Self {
        Self {
            tke_path: tke_path.into(),
            project_path: project_path.into(),
            device_id,
        }
    }

    /// 构建基础命令
    fn build_command(&self) -> Command {
        let mut cmd = Command::new(&self.tke_path);
        cmd.current_dir(&self.project_path);
        if let Some(device_id) = &self.device_id {
            cmd.arg("-d").arg(device_id);
        }
        cmd
    }

    /// 执行命令并返回 JSON 输出
    async fn execute_json<T: serde::de::DeserializeOwned>(
        &self,
        args: &[&str],
    ) -> Result<T> {
        let mut cmd = TokioCommand::new(&self.tke_path);
        cmd.current_dir(&self.project_path);

        if let Some(device_id) = &self.device_id {
            cmd.arg("-d").arg(device_id);
        }

        cmd.args(args);

        debug!("执行 TKE 命令: {:?}", cmd);

        let output = cmd
            .output()
            .await
            .context("执行 TKE 命令失败")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("TKE 命令执行失败: {}", stderr);
            anyhow::bail!("TKE 命令执行失败: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        debug!("TKE 输出: {}", stdout);

        serde_json::from_str(&stdout)
            .context("解析 TKE JSON 输出失败")
    }

    /// 截图并获取 UI 树
    pub async fn capture(&self) -> Result<CaptureResult> {
        info!("执行截图和获取 UI 树");
        self.execute_json(&["controller", "capture"]).await
    }

    /// 执行 OCR
    pub async fn ocr(&self, image_path: &str, online: bool, url: Option<&str>) -> Result<OcrResult> {
        info!("执行 OCR: {}", image_path);

        let mut args = vec!["ocr", "--image", image_path];

        if online {
            args.push("--online");
            if let Some(ocr_url) = url {
                args.push("--url");
                args.push(ocr_url);
            }
        }

        self.execute_json(&args).await
    }

    /// 提取 UI 元素
    pub async fn extract_ui_elements(&self, xml_path: &str) -> Result<UiElementsResult> {
        info!("提取 UI 元素: {}", xml_path);

        let mut cmd = TokioCommand::new(&self.tke_path);
        cmd.current_dir(&self.project_path);
        cmd.args(&["fetcher", "extract-ui-elements"]);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        // 读取 XML 文件
        let xml_content = tokio::fs::read_to_string(xml_path)
            .await
            .context("读取 XML 文件失败")?;

        let mut child = cmd.spawn().context("启动 TKE fetcher 失败")?;

        // 写入 stdin
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(xml_content.as_bytes())
                .await
                .context("写入 stdin 失败")?;
        }

        let output = child.wait_with_output().await.context("等待命令输出失败")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("TKE fetcher 执行失败: {}", stderr);
            anyhow::bail!("TKE fetcher 执行失败: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&stdout).context("解析 UI 元素 JSON 失败")
    }

    /// 点击
    pub async fn tap(&self, x: i32, y: i32) -> Result<ActionResult> {
        info!("点击: ({}, {})", x, y);
        let x_str = x.to_string();
        let y_str = y.to_string();
        self.execute_json(&["controller", "tap", &x_str, &y_str]).await
    }

    /// 滑动
    pub async fn swipe(&self, x1: i32, y1: i32, x2: i32, y2: i32, duration: Option<u32>) -> Result<ActionResult> {
        info!("滑动: ({}, {}) -> ({}, {})", x1, y1, x2, y2);

        let x1_str = x1.to_string();
        let y1_str = y1.to_string();
        let x2_str = x2.to_string();
        let y2_str = y2.to_string();

        let mut args = vec![
            "controller", "swipe",
            x1_str.as_str(), y1_str.as_str(),
            x2_str.as_str(), y2_str.as_str(),
        ];

        let duration_str;
        if let Some(d) = duration {
            duration_str = d.to_string();
            args.push("--duration");
            args.push(&duration_str);
        }

        self.execute_json(&args).await
    }

    /// 输入文字
    pub async fn input(&self, text: &str) -> Result<ActionResult> {
        info!("输入文字: {}", text);
        self.execute_json(&["controller", "input", text]).await
    }

    /// 返回
    pub async fn back(&self) -> Result<ActionResult> {
        info!("返回");
        self.execute_json(&["controller", "back"]).await
    }

    /// 启动 App
    pub async fn launch(&self, package: &str, activity: &str) -> Result<ActionResult> {
        info!("启动 App: {} / {}", package, activity);
        self.execute_json(&["controller", "launch", package, activity]).await
    }

    /// 停止 App
    pub async fn stop(&self, package: &str) -> Result<ActionResult> {
        info!("停止 App: {}", package);
        self.execute_json(&["controller", "stop", package]).await
    }

    /// 清空输入框
    pub async fn clear_input(&self) -> Result<ActionResult> {
        info!("清空输入框");
        self.execute_json(&["controller", "clear-input"]).await
    }
}
