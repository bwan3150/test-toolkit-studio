// Controller模块 - 负责ADB控制

use crate::{Result, TkeError, DeviceInfo, AdbManager};
use std::path::PathBuf;
use std::process::Command;
use tracing::{debug, info, error};
use anyhow::Context;

pub struct Controller {
    device_id: Option<String>,
    adb_manager: AdbManager,
}

impl Controller {
    pub fn new(device_id: Option<String>) -> Result<Self> {
        // 使用 AdbManager 来获取 ADB
        let adb_manager = AdbManager::new()?;
        
        // 验证 ADB 可用性
        adb_manager.verify_adb()?;
        
        Ok(Self {
            device_id,
            adb_manager,
        })
    }
    
    // 设置目标设备
    pub fn set_device(&mut self, device_id: Option<String>) {
        self.device_id = device_id;
    }
    
    // 获取连接的设备列表
    pub fn get_devices(&self) -> Result<Vec<String>> {
        let output = Command::new(self.adb_manager.adb_path())
            .arg("devices")
            .output()
            .map_err(|e| TkeError::AdbError(format!("执行adb devices失败: {}", e)))?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let devices: Vec<String> = stdout
            .lines()
            .skip(1)  // 跳过"List of devices attached"
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[1] == "device" {
                    Some(parts[0].to_string())
                } else {
                    None
                }
            })
            .collect();
        
        Ok(devices)
    }
    
    // 指令1: 获取设备截图和XML并保存到项目目录
    pub async fn capture_ui_state(&self, project_path: &PathBuf) -> Result<()> {
        let workarea = project_path.join("workarea");
        
        // 确保workarea目录存在
        std::fs::create_dir_all(&workarea)
            .map_err(|e| TkeError::IoError(e))?;
        
        // 获取截图
        self.capture_screenshot(&workarea.join("current_screenshot.png")).await?;
        
        // 获取UI树
        self.capture_ui_tree(&workarea.join("current_ui_tree.xml")).await?;
        
        info!("UI状态已捕获并保存到workarea");
        Ok(())
    }
    
    // 获取截图
    async fn capture_screenshot(&self, output_path: &PathBuf) -> Result<()> {
        let temp_path = "/sdcard/screenshot.png";
        
        // 在设备上截图
        self.run_adb_command(&["shell", "screencap", "-p", temp_path])?;
        
        // 拉取截图到本地
        self.run_adb_command(&[
            "pull",
            temp_path,
            output_path.to_str().ok_or_else(|| TkeError::InvalidArgument("无效的输出路径".to_string()))?
        ])?;
        
        // 删除设备上的临时文件
        self.run_adb_command(&["shell", "rm", temp_path])?;
        
        debug!("截图已保存到: {:?}", output_path);
        Ok(())
    }
    
    // 获取UI树
    async fn capture_ui_tree(&self, output_path: &PathBuf) -> Result<()> {
        let temp_path = "/sdcard/ui_dump.xml";
        
        // 在设备上dump UI
        self.run_adb_command(&["shell", "uiautomator", "dump", temp_path])?;
        
        // 拉取XML到本地
        self.run_adb_command(&[
            "pull",
            temp_path,
            output_path.to_str().ok_or_else(|| TkeError::InvalidArgument("无效的输出路径".to_string()))?
        ])?;
        
        // 删除设备上的临时文件
        self.run_adb_command(&["shell", "rm", temp_path])?;
        
        debug!("UI树已保存到: {:?}", output_path);
        Ok(())
    }
    
    // 获取UI XML内容作为字符串
    pub async fn get_ui_xml(&self) -> Result<String> {
        let temp_path = "/sdcard/ui_dump.xml";
        
        // 在设备上dump UI - 忽略输出，因为有些设备会返回非标准输出
        let dump_output = self.run_adb_command_output(&["shell", "uiautomator", "dump", temp_path])?;
        debug!("uiautomator dump输出: {}", dump_output);
        
        // 等待一下让文件写入完成
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // 读取XML内容
        let xml_content = self.run_adb_command_output(&["shell", "cat", temp_path])?;
        
        // 删除设备上的临时文件（忽略错误）
        let _ = self.run_adb_command(&["shell", "rm", temp_path]);
        
        debug!("获取UI XML内容，长度: {}", xml_content.len());
        Ok(xml_content)
    }
    
    // 指令2: 转发ADB基础指令
    
    // 点击坐标
    pub fn tap(&self, x: i32, y: i32) -> Result<()> {
        self.run_adb_command(&["shell", "input", "tap", &x.to_string(), &y.to_string()])?;
        debug!("点击坐标: ({}, {})", x, y);
        Ok(())
    }
    
    // 滑动
    pub fn swipe(&self, x1: i32, y1: i32, x2: i32, y2: i32, duration_ms: u32) -> Result<()> {
        self.run_adb_command(&[
            "shell", "input", "swipe",
            &x1.to_string(), &y1.to_string(),
            &x2.to_string(), &y2.to_string(),
            &duration_ms.to_string()
        ])?;
        debug!("滑动: ({}, {}) -> ({}, {}) 持续{}ms", x1, y1, x2, y2, duration_ms);
        Ok(())
    }
    
    // 长按
    pub fn press(&self, x: i32, y: i32, duration_ms: u32) -> Result<()> {
        // 使用swipe模拟长按
        self.swipe(x, y, x, y, duration_ms)?;
        debug!("长按坐标: ({}, {}) 持续{}ms", x, y, duration_ms);
        Ok(())
    }
    
    // 输入文本
    pub fn input_text(&self, text: &str) -> Result<()> {
        // 转义特殊字符
        let escaped = text
            .replace("\"", "\\\"")
            .replace("'", "\\'")
            .replace(" ", "%s");
        
        self.run_adb_command(&["shell", "input", "text", &escaped])?;
        debug!("输入文本: {}", text);
        Ok(())
    }
    
    // 按键事件
    pub fn key_event(&self, key_code: &str) -> Result<()> {
        self.run_adb_command(&["shell", "input", "keyevent", key_code])?;
        debug!("按键事件: {}", key_code);
        Ok(())
    }
    
    // 返回键
    pub fn back(&self) -> Result<()> {
        self.key_event("KEYCODE_BACK")
    }
    
    // 回到主屏
    pub fn home(&self) -> Result<()> {
        self.key_event("KEYCODE_HOME")
    }
    
    // 启动应用
    pub fn launch_app(&self, package: &str, activity: &str) -> Result<()> {
        let component = format!("{}/{}", package, activity);
        self.run_adb_command(&["shell", "am", "start", "-n", &component])?;
        info!("启动应用: {}", component);
        Ok(())
    }
    
    // 停止应用
    pub fn stop_app(&self, package: &str) -> Result<()> {
        self.run_adb_command(&["shell", "am", "force-stop", package])?;
        info!("停止应用: {}", package);
        Ok(())
    }
    
    // 清理输入框
    pub fn clear_input(&self) -> Result<()> {
        // 移到行尾
        self.key_event("KEYCODE_MOVE_END")?;
        // 全选并删除
        for _ in 0..50 {
            self.key_event("KEYCODE_DEL")?;
        }
        Ok(())
    }
    
    // 隐藏键盘
    pub fn hide_keyboard(&self) -> Result<()> {
        // 尝试返回键隐藏键盘
        self.key_event("KEYCODE_BACK")?;
        Ok(())
    }
    
    // 获取设备信息
    pub fn get_device_info(&self) -> Result<DeviceInfo> {
        let model = self.get_device_prop("ro.product.model")?;
        let manufacturer = self.get_device_prop("ro.product.manufacturer")?;
        let android_version = self.get_device_prop("ro.build.version.release")?;
        
        // 获取屏幕尺寸
        let size_output = self.run_adb_command_output(&["shell", "wm", "size"])?;
        let (width, height) = self.parse_screen_size(&size_output);
        
        Ok(DeviceInfo {
            id: self.device_id.clone().unwrap_or_default(),
            model: Some(model),
            manufacturer: Some(manufacturer),
            android_version: Some(android_version),
            screen_width: width,
            screen_height: height,
        })
    }
    
    // 获取设备属性
    fn get_device_prop(&self, prop: &str) -> Result<String> {
        let output = self.run_adb_command_output(&["shell", "getprop", prop])?;
        Ok(output.trim().to_string())
    }
    
    // 解析屏幕尺寸
    fn parse_screen_size(&self, output: &str) -> (u32, u32) {
        // 输出格式: "Physical size: 1080x1920"
        if let Some(pos) = output.find(": ") {
            let size_str = &output[pos + 2..];
            if let Some(x_pos) = size_str.find('x') {
                let width = size_str[..x_pos].trim().parse().unwrap_or(1080);
                let height = size_str[x_pos + 1..].trim().parse().unwrap_or(1920);
                return (width, height);
            }
        }
        (1080, 1920)  // 默认值
    }
    
    // 执行ADB命令
    fn run_adb_command(&self, args: &[&str]) -> Result<()> {
        let mut cmd = Command::new(self.adb_manager.adb_path());
        
        // 如果指定了设备ID，添加-s参数
        if let Some(ref device_id) = self.device_id {
            cmd.arg("-s").arg(device_id);
        }
        
        cmd.args(args);
        
        let output = cmd.output()
            .map_err(|e| TkeError::AdbError(format!("执行ADB命令失败: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TkeError::AdbError(format!("ADB命令执行失败: {}", stderr)));
        }
        
        Ok(())
    }
    
    // 执行ADB命令并获取输出
    fn run_adb_command_output(&self, args: &[&str]) -> Result<String> {
        let mut cmd = Command::new(self.adb_manager.adb_path());
        
        // 如果指定了设备ID，添加-s参数
        if let Some(ref device_id) = self.device_id {
            cmd.arg("-s").arg(device_id);
        }
        
        cmd.args(args);
        
        let output = cmd.output()
            .map_err(|e| TkeError::AdbError(format!("执行ADB命令失败: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TkeError::AdbError(format!("ADB命令执行失败: {}", stderr)));
        }
        
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}