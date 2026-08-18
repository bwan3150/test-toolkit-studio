// 二进制定位（原「① 直通」层）
//
// **CLI 直通已删除**（ADR-0015）：`tke adb shell input tap …` 这种用法绕过了
// 证据留存、坐标换算和统一的动作映射——点得中、什么都没留下，报告里一片空白。
// 设备操作一律走 tke 自己的指令，由 tke 转译后再落到二进制上。
//
// 留下的是**内部定位器**，它是必需的：adb / chromedriver / go-ios / tke-opencv
// 都靠它找到自己在哪。
//
//   resolve(name)  解析同目录二进制路径

mod manager;

pub use manager::ToolManager;
