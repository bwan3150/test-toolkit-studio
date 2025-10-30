// Controller模块 - 负责ADB控制

use crate::{Result, TkeError, DeviceInfo, AdbManager};
use std::path::PathBuf;
use std::process::Command;

const DEFAULT_PORT: u16 = 8765;

pub struct Controller {
    device_id: Option<String>,
    adb_manager: AdbManager,
    port: u16,  // autoserver 端口
}

impl Controller {
    pub fn new(device_id: Option<String>) -> Result<Self> {
        // 使用 AdbManager 来获取 ADB (静默模式)
        let adb_manager = AdbManager::new()?;

        // 验证 ADB 可用性
        adb_manager.verify_adb()?;

        Ok(Self {
            device_id,
            adb_manager,
            port: DEFAULT_PORT,
        })
    }

    /// 设置 autoserver 端口（可选，默认 8765）
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// 获取当前使用的端口
    pub fn port(&self) -> u16 {
        self.port
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

        Ok(())
    }

    // 仅获取截图
    pub async fn capture_screenshot_only(&self, output_path: &PathBuf) -> Result<()> {
        self.capture_screenshot(output_path).await
    }

    // 仅获取 UI 树 XML
    pub async fn capture_xml_only(&self, output_path: &PathBuf) -> Result<()> {
        self.capture_ui_tree(output_path).await
    }
    
    // 获取截图（通过 autoserver）
    async fn capture_screenshot(&self, output_path: &PathBuf) -> Result<()> {
        use std::net::TcpStream;
        use std::io::Read;

        // 连接到 autoserver (通过 adb forward 的端口)
        let addr = format!("127.0.0.1:{}", self.port);
        let mut stream = TcpStream::connect(&addr)
            .map_err(|e| TkeError::AdbError(
                format!("无法连接到 autoserver ({}): {}. 请先启动屏幕投影", addr, e)
            ))?;

        // 读取 PNG 数据
        let mut png_data = Vec::new();
        stream.read_to_end(&mut png_data)
            .map_err(|e| TkeError::IoError(e))?;

        // 保存到文件
        std::fs::write(output_path, png_data)
            .map_err(|e| TkeError::IoError(e))?;

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

        Ok(())
    }
    
    // 获取UI XML内容作为字符串
    pub async fn get_ui_xml(&self) -> Result<String> {
        let temp_path = "/sdcard/ui_dump.xml";
        
        // 在设备上dump UI - 忽略输出，因为有些设备会返回非标准输出
        let _dump_output = self.run_adb_command_output(&["shell", "uiautomator", "dump", temp_path])?;
        
        // 等待一下让文件写入完成
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // 读取XML内容
        let xml_content = self.run_adb_command_output(&["shell", "cat", temp_path])?;
        
        // 删除设备上的临时文件（忽略错误）
        let _ = self.run_adb_command(&["shell", "rm", temp_path]);

        Ok(xml_content)
    }

    // 指令2: 转发ADB基础指令

    // 点击坐标
    pub fn tap(&self, x: i32, y: i32) -> Result<()> {
        self.run_adb_command(&["shell", "input", "tap", &x.to_string(), &y.to_string()])?;
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
        Ok(())
    }

    // 长按
    pub fn press(&self, x: i32, y: i32, duration_ms: u32) -> Result<()> {
        // 使用swipe模拟长按
        self.swipe(x, y, x, y, duration_ms)?;
        Ok(())
    }

    // 输入文本
    pub fn input_text(&self, text: &str) -> Result<()> {
        // 保存当前输入法
        let current_ime = self.get_current_ime().unwrap_or_default();

        // 判断文本类型并切换输入法
        if self.is_chinese_text(text) {
            // 需要中文输入法 - 切换到中文输入法
            self.switch_to_chinese_ime()?;
        } else {
            // 需要英文输入法 - 切换到英文输入法（或关闭中文输入法）
            self.switch_to_english_ime()?;
        }

        // 等待输入法切换完成
        std::thread::sleep(std::time::Duration::from_millis(300));

        // 转义特殊字符
        let escaped = text
            .replace("\"", "\\\"")
            .replace("'", "\\'")
            .replace(" ", "%s");

        self.run_adb_command(&["shell", "input", "text", &escaped])?;

        // 恢复原来的输入法
        if !current_ime.is_empty() && !current_ime.contains("not found") {
            std::thread::sleep(std::time::Duration::from_millis(200));
            let _ = self.run_adb_command(&["shell", "ime", "set", &current_ime]);
        }

        // 不隐藏键盘 - 让后续操作（如点击按钮）自然隐藏，避免在某些页面触发返回

        Ok(())
    }

    // 获取当前输入法
    fn get_current_ime(&self) -> Result<String> {
        let output = self.run_adb_command_output(&["shell", "settings", "get", "secure", "default_input_method"])?;
        Ok(output.trim().to_string())
    }

    // 判断文本是否包含中文
    fn is_chinese_text(&self, text: &str) -> bool {
        text.chars().any(|c| {
            // 判断字符是否在中文 Unicode 范围内
            matches!(c, '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' | '\u{20000}'..='\u{2A6DF}')
        })
    }

    // 切换到英文输入法
    fn switch_to_english_ime(&self) -> Result<()> {
        // 获取设备上可用的输入法列表
        let ime_list = self.run_adb_command_output(&["shell", "ime", "list", "-s"]).unwrap_or_default();

        // 优先使用 Appium Unicode IME（最可靠，专门用于自动化测试）
        if ime_list.contains("io.appium.settings/.UnicodeIME") {
            if self.run_adb_command(&["shell", "ime", "set", "io.appium.settings/.UnicodeIME"]).is_ok() {
                return Ok(());
            }
        }

        // 尝试切换到 Google 输入法（支持英文）
        let english_imes = [
            "com.google.android.inputmethod.latin/com.android.inputmethod.latin.LatinIME",
            "com.android.inputmethod.latin/.LatinIME",
            "com.android.inputmethod/.LatinIME",
        ];

        for ime in &english_imes {
            if ime_list.contains(ime) {
                if self.run_adb_command(&["shell", "ime", "set", ime]).is_ok() {
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    // 切换到中文输入法
    fn switch_to_chinese_ime(&self) -> Result<()> {
        // 获取设备上可用的输入法列表
        let ime_list = self.run_adb_command_output(&["shell", "ime", "list", "-s"]).unwrap_or_default();

        // 尝试切换到中文输入法，按优先级尝试
        let chinese_imes = [
            "com.netease.nie.yosemite/.ime.ImeService",  // 网易输入法
            "com.google.android.inputmethod.pinyin/.PinyinIME",  // Google 拼音
            "com.sohu.inputmethod.sogou/.SogouIME",  // 搜狗
            "com.baidu.input/.ImeService",  // 百度
            "com.iflytek.inputmethod/.FlyIME",  // 讯飞
        ];

        for ime in &chinese_imes {
            if ime_list.contains(ime) {
                if self.run_adb_command(&["shell", "ime", "set", ime]).is_ok() {
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    // 按键事件
    pub fn key_event(&self, key_code: &str) -> Result<()> {
        self.run_adb_command(&["shell", "input", "keyevent", key_code])?;
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
        Ok(())
    }

    // 停止应用
    pub fn stop_app(&self, package: &str) -> Result<()> {
        self.run_adb_command(&["shell", "am", "force-stop", package])?;
        Ok(())
    }
    
    // 清理输入框
    pub fn clear_input(&self) -> Result<()> {
        // 简单粗暴的方法：向前删除20次，向后删除20次
        // 这样无论光标在哪个位置都能清空内容

        // 向前删除 20 次 (KEYCODE_DEL = 向前删除，相当于 Backspace)
        for _ in 0..20 {
            self.key_event("KEYCODE_DEL")?;
        }

        // 向后删除 20 次 (KEYCODE_FORWARD_DEL = 向后删除，相当于 Delete)
        for _ in 0..20 {
            self.key_event("KEYCODE_FORWARD_DEL")?;
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