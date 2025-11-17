// 数据模型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// 坐标点
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

// 边界框
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bounds {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

impl Bounds {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        Self { x1, y1, x2, y2 }
    }
    
    pub fn center(&self) -> Point {
        Point {
            x: (self.x1 + self.x2) / 2,
            y: (self.y1 + self.y2) / 2,
        }
    }
    
    pub fn width(&self) -> i32 {
        self.x2 - self.x1
    }
    
    pub fn height(&self) -> i32 {
        self.y2 - self.y1
    }
    
    pub fn is_visible(&self) -> bool {
        self.width() > 0 && self.height() > 0
    }
}

// UI元素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIElement {
    pub index: usize,
    pub class_name: String,
    pub bounds: Bounds,
    pub text: Option<String>,
    pub content_desc: Option<String>,
    pub resource_id: Option<String>,
    pub hint: Option<String>,
    pub clickable: bool,
    pub checkable: bool,
    pub checked: bool,
    pub focusable: bool,
    pub focused: bool,
    pub scrollable: bool,
    pub selected: bool,
    pub enabled: bool,
    pub xpath: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_index: Option<usize>,  // 用于前端渲染的z-index,基于元素面积计算
    #[serde(skip)]
    pub parent_index: Option<usize>,  // 父元素的index
    #[serde(skip)]
    pub depth: usize,  // 层级深度（从0开始）
    #[serde(skip)]
    pub sibling_index: usize,  // 在同一父元素下，相同class的第几个（从1开始）
}

impl UIElement {
    pub fn center(&self) -> Point {
        self.bounds.center()
    }
    
    pub fn is_visible(&self) -> bool {
        self.bounds.is_visible()
    }
    
    pub fn matches_text(&self, text: &str) -> bool {
        let text_lower = text.to_lowercase();
        
        if let Some(ref t) = self.text {
            if t.to_lowercase().contains(&text_lower) {
                return true;
            }
        }
        
        if let Some(ref desc) = self.content_desc {
            if desc.to_lowercase().contains(&text_lower) {
                return true;
            }
        }
        
        if let Some(ref hint) = self.hint {
            if hint.to_lowercase().contains(&text_lower) {
                return true;
            }
        }
        
        false
    }
    
    pub fn to_ai_text(&self) -> String {
        let simple_class = self.class_name.split('.').last().unwrap_or(&self.class_name);
        let mut attrs = Vec::new();
        
        if let Some(ref text) = self.text {
            attrs.push(format!("text={}", text));
        }
        if let Some(ref hint) = self.hint {
            attrs.push(format!("hint={}", hint));
        }
        if let Some(ref desc) = self.content_desc {
            attrs.push(format!("content-desc={}", desc));
        }
        if let Some(ref id) = self.resource_id {
            let id_part = id.split(':').last().unwrap_or(id);
            attrs.push(format!("id={}", id_part));
        }
        
        if self.checked {
            attrs.push("checked=true".to_string());
        }
        if self.focused {
            attrs.push("focused=true".to_string());
        }
        if self.selected {
            attrs.push("selected=true".to_string());
        }
        if !self.enabled {
            attrs.push("enabled=false".to_string());
        }
        
        if attrs.is_empty() {
            format!("{}()", simple_class)
        } else {
            format!("{}({})", simple_class, attrs.join(", "))
        }
    }
}

// Locator类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LocatorType {
    Xml,
    Image,
}

// Locator定义 - 按照element.json格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Locator {
    #[serde(rename = "type")]
    pub locator_type: LocatorType,
    
    // XML定位器字段 - 使用element.json的字段名（snake_case）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<Bounds>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_desc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focusable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scrollable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xpath: Option<String>,
    #[serde(rename = "addedAt", skip_serializing_if = "Option::is_none")]
    pub added_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "centerX", skip_serializing_if = "Option::is_none")]
    pub center_x: Option<i32>,
    #[serde(rename = "centerY", skip_serializing_if = "Option::is_none")]
    pub center_y: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_strategy: Option<String>,
    
    // 图像定位器字段
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}


// 硬件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    /// CPU 型号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_model: Option<String>,
    /// CPU 核心数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_cores: Option<u32>,
    /// CPU 架构 (如 arm64-v8a)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_abi: Option<String>,
    /// 总内存 (MB)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_memory_mb: Option<u64>,
    /// 可用内存 (MB)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_memory_mb: Option<u64>,
    /// 总存储空间 (GB)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_storage_gb: Option<f64>,
    /// 可用存储空间 (GB)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_storage_gb: Option<f64>,
}

// 电池信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryInfo {
    /// 电池电量 (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    /// 电池温度 (摄氏度)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// 电池健康状态 (Good, Overheat, Dead, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<String>,
    /// 充电状态 (Charging, Discharging, Not charging, Full, Unknown)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// 是否正在充电
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_charging: Option<bool>,
    /// 电源类型 (AC, USB, Wireless, None)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_source: Option<String>,
}

// 网络信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    /// WiFi 连接状态
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wifi_enabled: Option<bool>,
    /// WiFi SSID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wifi_ssid: Option<String>,
    /// 移动网络类型 (LTE, 5G, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mobile_network_type: Option<String>,
    /// 运营商名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_name: Option<String>,
    /// 运营商代码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_code: Option<String>,
    /// 国家代码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_iso: Option<String>,
}

// 设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub model: Option<String>,
    pub manufacturer: Option<String>,
    pub android_version: Option<String>,
    pub screen_width: u32,
    pub screen_height: u32,
    /// 硬件信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware: Option<HardwareInfo>,
    /// 电池信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery: Option<BatteryInfo>,
    /// 网络信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<NetworkInfo>,
}

// TKS脚本命令类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TksCommand {
    Launch,      // 启动
    Close,       // 关闭
    Click,       // 点击
    Press,       // 按压
    Swipe,       // 滑动
    DirectionalSwipe,  // 定向滑动
    Input,       // 输入
    Clear,       // 清理
    HideKeyboard, // 隐藏键盘
    Back,        // 返回
    Wait,        // 等待
    Assert,      // 断言
}

impl TksCommand {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "启动" => Some(Self::Launch),
            "关闭" => Some(Self::Close),
            "点击" => Some(Self::Click),
            "按压" => Some(Self::Press),
            "滑动" => Some(Self::Swipe),
            "定向滑动" => Some(Self::DirectionalSwipe),
            "输入" => Some(Self::Input),
            "清理" => Some(Self::Clear),
            "隐藏键盘" => Some(Self::HideKeyboard),
            "返回" => Some(Self::Back),
            "等待" => Some(Self::Wait),
            "断言" => Some(Self::Assert),
            _ => None,
        }
    }
}

// TKS脚本参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TksParam {
    Text(String),           // 纯文本
    Number(i32),            // 数字
    Duration(u32),          // 持续时间(毫秒)
    Coordinate(Point),      // 坐标 {x,y}
    /// XML元素
    /// - 基础格式: {元素名} - 全精确匹配
    /// - 指定策略: {元素名}&resourceId, {元素名}&text, {元素名}&className, {元素名}&xpath
    XmlElement {
        name: String,
        strategy: Option<String>,  // 可选的查找策略：resourceId, text, className, xpath
    },
    ImageElement(String),   // 图像元素 @{图片名}
    Direction(String),      // 方向 up/down/left/right
    Boolean(bool),          // 布尔值
}

// TKS脚本步骤
#[derive(Debug, Clone)]
pub struct TksStep {
    pub command: TksCommand,
    pub params: Vec<TksParam>,
    pub raw: String,
    pub line_number: usize,
}

// TKS脚本
#[derive(Debug, Clone)]
pub struct TksScript {
    pub case_id: String,
    pub script_name: String,
    pub details: HashMap<String, String>,
    pub steps: Vec<TksStep>,
    pub file_path: Option<PathBuf>,
}

// 执行结果
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub case_id: String,
    pub script_name: String,
    pub start_time: String,
    pub end_time: String,
    pub steps: Vec<StepResult>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub index: usize,
    pub command: String,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
}

// App 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    /// 包名
    pub package_name: String,
    /// 版本名称 (如 8.8.76.667)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_name: Option<String>,
    /// 版本号 (如 105910845)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_code: Option<i64>,
    /// APK 路径
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apk_path: Option<String>,
    /// 启动 Activity (用于启动应用)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launch_activity: Option<String>,
}

// 当前聚焦的应用信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentFocus {
    /// 包名
    pub package_name: String,
    /// Activity 名称
    pub activity_name: String,
    /// 完整的窗口信息 (原始输出)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_info: Option<String>,
}