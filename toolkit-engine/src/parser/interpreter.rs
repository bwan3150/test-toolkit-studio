// ScriptInterpreter模块 - 脚本解释器，将TKS指令转换为可执行的ADB指令

use crate::{Result, TkeError, TksScript, TksStep, TksCommand, TksParam, Point, Controller, Recognizer};
use std::path::PathBuf;
use tracing::{debug, info, error};

pub struct ScriptInterpreter {
    project_path: PathBuf,
    device_id: Option<String>,
    controller: Controller,
    recognizer: Recognizer,
}

impl ScriptInterpreter {
    pub fn new(project_path: PathBuf, device_id: Option<String>) -> Result<Self> {
        let controller = Controller::new(device_id.clone())?;
        let recognizer = Recognizer::new(project_path.clone())?;
        
        Ok(Self {
            project_path,
            device_id,
            controller,
            recognizer,
        })
    }
    
    // 解释并执行单个步骤
    pub async fn interpret_step(&mut self, step: &TksStep) -> Result<()> {
        debug!("执行步骤: {} (行号: {})", step.raw, step.line_number);
        
        match step.command {
            TksCommand::Launch => self.execute_launch(&step.params).await,
            TksCommand::Close => self.execute_close(&step.params),
            TksCommand::Click => self.execute_click(&step.params).await,
            TksCommand::Press => self.execute_press(&step.params).await,
            TksCommand::Swipe => self.execute_swipe(&step.params).await,
            TksCommand::DirectionalSwipe => self.execute_directional_swipe(&step.params).await,
            TksCommand::Input => self.execute_input(&step.params).await,
            TksCommand::Clear => self.execute_clear(&step.params).await,
            TksCommand::HideKeyboard => self.execute_hide_keyboard(),
            TksCommand::Back => self.execute_back(),
            TksCommand::Wait => self.execute_wait(&step.params).await,
            TksCommand::Assert => self.execute_assert(&step.params).await,
        }
    }
    
    // 启动应用
    async fn execute_launch(&mut self, params: &[TksParam]) -> Result<()> {
        if params.len() < 2 {
            return Err(TkeError::InvalidArgument("启动命令需要包名和Activity名".to_string()));
        }
        
        let package = self.extract_text(&params[0])?;
        let activity = self.extract_text(&params[1])?;
        
        self.controller.launch_app(&package, &activity)?;
        
        // 等待应用启动
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
        
        // 刷新UI状态
        self.controller.capture_ui_state(&self.project_path).await?;
        
        Ok(())
    }
    
    // 关闭应用
    fn execute_close(&mut self, params: &[TksParam]) -> Result<()> {
        if params.is_empty() {
            return Err(TkeError::InvalidArgument("关闭命令需要包名".to_string()));
        }
        
        let package = self.extract_text(&params[0])?;
        self.controller.stop_app(&package)
    }
    
    // 点击操作
    async fn execute_click(&mut self, params: &[TksParam]) -> Result<()> {
        if params.is_empty() {
            return Err(TkeError::InvalidArgument("点击命令需要目标参数".to_string()));
        }
        
        let point = self.resolve_target(&params[0]).await?;
        self.controller.tap(point.x, point.y)
    }
    
    // 长按操作
    async fn execute_press(&mut self, params: &[TksParam]) -> Result<()> {
        if params.is_empty() {
            return Err(TkeError::InvalidArgument("按压命令需要目标参数".to_string()));
        }
        
        let point = self.resolve_target(&params[0]).await?;
        let duration = if params.len() > 1 {
            self.extract_duration(&params[1])?
        } else {
            1000 // 默认1秒
        };
        
        self.controller.press(point.x, point.y, duration)
    }
    
    // 滑动操作
    async fn execute_swipe(&mut self, params: &[TksParam]) -> Result<()> {
        if params.len() < 2 {
            return Err(TkeError::InvalidArgument("滑动命令需要起点和终点".to_string()));
        }
        
        let from_point = self.resolve_target(&params[0]).await?;
        let to_point = self.resolve_target(&params[1]).await?;
        let duration = if params.len() > 2 {
            self.extract_duration(&params[2])?
        } else {
            300 // 默认300ms
        };
        
        self.controller.swipe(from_point.x, from_point.y, to_point.x, to_point.y, duration)
    }
    
    // 定向滑动操作
    async fn execute_directional_swipe(&mut self, params: &[TksParam]) -> Result<()> {
        if params.len() < 3 {
            return Err(TkeError::InvalidArgument("定向滑动命令需要起点、方向和距离".to_string()));
        }
        
        let from_point = self.resolve_target(&params[0]).await?;
        let direction = self.extract_direction(&params[1])?;
        let distance = self.extract_number(&params[2])?;
        let duration = if params.len() > 3 {
            self.extract_duration(&params[3])?
        } else {
            300 // 默认300ms
        };
        
        let to_point = match direction.as_str() {
            "up" => Point::new(from_point.x, from_point.y - distance),
            "down" => Point::new(from_point.x, from_point.y + distance),
            "left" => Point::new(from_point.x - distance, from_point.y),
            "right" => Point::new(from_point.x + distance, from_point.y),
            _ => return Err(TkeError::InvalidArgument(format!("无效的方向: {}", direction))),
        };
        
        self.controller.swipe(from_point.x, from_point.y, to_point.x, to_point.y, duration)
    }
    
    // 输入文本
    async fn execute_input(&mut self, params: &[TksParam]) -> Result<()> {
        if params.len() < 2 {
            return Err(TkeError::InvalidArgument("输入命令需要目标和文本".to_string()));
        }
        
        // 先点击输入框
        let point = self.resolve_target(&params[0]).await?;
        self.controller.tap(point.x, point.y)?;
        
        // 等待键盘弹出
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // 输入文本
        let text = self.extract_text(&params[1])?;
        self.controller.input_text(&text)
    }
    
    // 清理输入框
    async fn execute_clear(&mut self, params: &[TksParam]) -> Result<()> {
        if !params.is_empty() {
            // 先点击输入框
            let point = self.resolve_target(&params[0]).await?;
            self.controller.tap(point.x, point.y)?;
            
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        
        self.controller.clear_input()
    }
    
    // 隐藏键盘
    fn execute_hide_keyboard(&mut self) -> Result<()> {
        self.controller.hide_keyboard()
    }
    
    // 返回操作
    fn execute_back(&mut self) -> Result<()> {
        self.controller.back()
    }
    
    // 等待操作
    async fn execute_wait(&mut self, params: &[TksParam]) -> Result<()> {
        if params.is_empty() {
            // 默认等待1秒
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            return Ok(());
        }
        
        match &params[0] {
            TksParam::Duration(ms) => {
                debug!("等待 {} 毫秒", ms);
                tokio::time::sleep(tokio::time::Duration::from_millis(*ms as u64)).await;
            }
            TksParam::Number(num) => {
                // 数字参数：如果小于等于3600，当作秒数；否则当作毫秒数
                if *num <= 3600 {
                    debug!("等待 {} 秒", num);
                    tokio::time::sleep(tokio::time::Duration::from_secs(*num as u64)).await;
                } else {
                    debug!("等待 {} 毫秒", num);
                    tokio::time::sleep(tokio::time::Duration::from_millis(*num as u64)).await;
                }
            }
            TksParam::XmlElement(element_name) | TksParam::ImageElement(element_name) => {
                debug!("等待元素出现: {}", element_name);
                // 等待元素出现，最多等待30秒
                let timeout = tokio::time::Duration::from_secs(30);
                let start = tokio::time::Instant::now();
                
                while start.elapsed() < timeout {
                    // 刷新UI状态
                    if let Err(e) = self.controller.capture_ui_state(&self.project_path).await {
                        debug!("刷新UI状态失败: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        continue;
                    }
                    
                    // 尝试查找元素
                    let result = match &params[0] {
                        TksParam::XmlElement(name) => {
                            debug!("查找XML元素: {}", name);
                            self.recognizer.find_xml_element(name)
                        }
                        TksParam::ImageElement(name) => {
                            debug!("查找图像元素: {}", name);
                            self.recognizer.find_image_element(name)
                        }
                        _ => unreachable!(),
                    };
                    
                    if result.is_ok() {
                        info!("找到元素: {}", element_name);
                        return Ok(()); // 找到元素，结束等待
                    } else {
                        debug!("未找到元素: {}, 错误: {}", element_name, result.unwrap_err());
                    }
                    
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
                
                return Err(TkeError::ScriptExecuteError(format!("等待元素超时: {}", element_name)));
            }
            TksParam::Text(text) => {
                // 支持文本参数的等待，与JS版本保持一致
                if let Ok(seconds) = text.parse::<u64>() {
                    debug!("等待 {} 秒 (文本解析)", seconds);
                    tokio::time::sleep(tokio::time::Duration::from_secs(seconds)).await;
                } else {
                    return Err(TkeError::InvalidArgument(format!("无法解析等待参数: {}", text)));
                }
            }
            _ => {
                return Err(TkeError::InvalidArgument("等待命令参数无效".to_string()));
            }
        }
        
        Ok(())
    }
    
    // 断言操作
    async fn execute_assert(&mut self, params: &[TksParam]) -> Result<()> {
        if params.len() < 2 {
            return Err(TkeError::InvalidArgument("断言命令需要目标和条件".to_string()));
        }
        
        // 刷新UI状态
        self.controller.capture_ui_state(&self.project_path).await?;
        
        let element_exists = match &params[0] {
            TksParam::XmlElement(name) => {
                self.recognizer.find_xml_element(name).is_ok()
            }
            TksParam::ImageElement(name) => {
                self.recognizer.find_image_element(name).is_ok()
            }
            _ => return Err(TkeError::InvalidArgument("断言目标必须是元素".to_string())),
        };
        
        let expected = match &params[1] {
            TksParam::Boolean(b) => *b,
            TksParam::Text(t) => t == "存在",
            _ => return Err(TkeError::InvalidArgument("断言条件无效".to_string())),
        };
        
        if element_exists != expected {
            let element_name = match &params[0] {
                TksParam::XmlElement(name) | TksParam::ImageElement(name) => name,
                _ => "未知元素",
            };
            
            return Err(TkeError::ScriptExecuteError(
                format!("断言失败: 元素 '{}' {}，但期望{}", 
                       element_name,
                       if element_exists { "存在" } else { "不存在" },
                       if expected { "存在" } else { "不存在" })
            ));
        }
        
        Ok(())
    }
    
    // 辅助方法：解析目标位置
    async fn resolve_target(&mut self, param: &TksParam) -> Result<Point> {
        match param {
            TksParam::Coordinate(point) => {
                debug!("使用坐标: ({}, {})", point.x, point.y);
                Ok(*point)
            }
            TksParam::XmlElement(name) => {
                debug!("查找XML元素: {}", name);
                // 刷新UI状态
                if let Err(e) = self.controller.capture_ui_state(&self.project_path).await {
                    error!("刷新UI状态失败: {}", e);
                    return Err(e);
                }
                
                match self.recognizer.find_xml_element(name) {
                    Ok(point) => {
                        info!("找到XML元素 '{}' 位置: ({}, {})", name, point.x, point.y);
                        Ok(point)
                    }
                    Err(e) => {
                        error!("查找XML元素 '{}' 失败: {}", name, e);
                        Err(e)
                    }
                }
            }
            TksParam::ImageElement(name) => {
                debug!("查找图像元素: {}", name);
                // 刷新UI状态  
                if let Err(e) = self.controller.capture_ui_state(&self.project_path).await {
                    error!("刷新UI状态失败: {}", e);
                    return Err(e);
                }
                
                match self.recognizer.find_image_element(name) {
                    Ok(point) => {
                        info!("找到图像元素 '{}' 位置: ({}, {})", name, point.x, point.y);
                        Ok(point)
                    }
                    Err(e) => {
                        error!("查找图像元素 '{}' 失败: {}", name, e);
                        Err(e)
                    }
                }
            }
            TksParam::Text(text) => {
                debug!("查找文本元素: {}", text);
                // 刷新UI状态
                if let Err(e) = self.controller.capture_ui_state(&self.project_path).await {
                    error!("刷新UI状态失败: {}", e);
                    return Err(e);
                }
                
                match self.recognizer.find_element_by_text(text) {
                    Ok(point) => {
                        info!("找到文本元素 '{}' 位置: ({}, {})", text, point.x, point.y);
                        Ok(point)
                    }
                    Err(e) => {
                        error!("查找文本元素 '{}' 失败: {}", text, e);
                        Err(e)
                    }
                }
            }
            _ => {
                error!("无效的目标类型: {:?}", param);
                Err(TkeError::InvalidArgument("无效的目标类型".to_string()))
            }
        }
    }
    
    // 提取文本参数
    fn extract_text(&self, param: &TksParam) -> Result<String> {
        match param {
            TksParam::Text(s) => Ok(s.clone()),
            _ => Err(TkeError::InvalidArgument("期望文本参数".to_string())),
        }
    }
    
    // 提取数字参数
    fn extract_number(&self, param: &TksParam) -> Result<i32> {
        match param {
            TksParam::Number(n) => Ok(*n),
            _ => Err(TkeError::InvalidArgument("期望数字参数".to_string())),
        }
    }
    
    // 提取持续时间参数
    fn extract_duration(&self, param: &TksParam) -> Result<u32> {
        match param {
            TksParam::Duration(ms) => Ok(*ms),
            TksParam::Number(n) => Ok(*n as u32),
            _ => Err(TkeError::InvalidArgument("期望持续时间参数".to_string())),
        }
    }
    
    // 提取方向参数
    fn extract_direction(&self, param: &TksParam) -> Result<String> {
        match param {
            TksParam::Direction(d) => Ok(d.clone()),
            TksParam::Text(t) => Ok(t.clone()),
            _ => Err(TkeError::InvalidArgument("期望方向参数".to_string())),
        }
    }
}