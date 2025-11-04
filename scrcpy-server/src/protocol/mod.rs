// Protocol 模块 - scrcpy 协议相关
pub mod video_settings;
pub mod control_message;

pub use video_settings::{VideoSettings, Bounds, Crop};
pub use control_message::{ControlMessage, ControlMessageType, TouchAction};
