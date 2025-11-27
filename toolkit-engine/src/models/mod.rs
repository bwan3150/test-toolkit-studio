// 数据模型模块
// 所有数据结构的统一导出

mod point;
mod ui_element;
mod locator;
mod tks_types;
mod device_info;
mod execution;

// 导出所有公开类型
pub use point::{Point, Bounds};
pub use ui_element::UIElement;
pub use locator::{Locator, LocatorStrategy};
pub use tks_types::{TksCommand, TksParam, TksStep, TksScript};
pub use device_info::{DeviceInfo, HardwareInfo, BatteryInfo, NetworkInfo};
pub use execution::{ExecutionResult, StepResult, AppInfo, CurrentFocus};
