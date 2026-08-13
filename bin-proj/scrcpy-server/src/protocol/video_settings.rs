// 视频设置类 - 从 ws-scrcpy VideoSettings.ts 转换
// 用于配置视频流参数

use byteorder::{BigEndian, WriteBytesExt};
use std::io::Write;

#[derive(Debug, Clone)]
pub struct Bounds {
    pub width: i16,
    pub height: i16,
}

#[derive(Debug, Clone)]
pub struct Crop {
    pub left: i16,
    pub top: i16,
    pub right: i16,
    pub bottom: i16,
}

#[derive(Debug, Clone)]
pub struct VideoSettings {
    pub bitrate: i32,                          // 比特率 (默认 512kbps)
    pub max_fps: i32,                          // 最大帧率
    pub i_frame_interval: i8,                  // I帧间隔 (秒)
    pub bounds: Option<Bounds>,                // 分辨率限制
    pub crop: Option<Crop>,                    // 裁剪
    pub send_frame_meta: bool,                 // 是否发送帧元数据
    pub locked_video_orientation: i8,          // 锁定视频方向
    pub display_id: i32,                       // 显示器ID
    pub codec_options: String,                 // 编解码器选项
    pub encoder_name: String,                  // 编码器名称
}

impl Default for VideoSettings {
    fn default() -> Self {
        Self {
            bitrate: 524288,              // 512kbps
            max_fps: 24,
            i_frame_interval: 5,
            bounds: None,
            crop: None,
            send_frame_meta: false,
            locked_video_orientation: -1,
            display_id: 0,
            codec_options: String::new(),
            encoder_name: String::new(),
        }
    }
}

impl VideoSettings {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置比特率
    pub fn with_bitrate(mut self, bitrate: i32) -> Self {
        self.bitrate = bitrate;
        self
    }

    /// 设置最大帧率
    pub fn with_max_fps(mut self, max_fps: i32) -> Self {
        self.max_fps = max_fps;
        self
    }

    /// 设置 I 帧间隔
    pub fn with_i_frame_interval(mut self, interval: i8) -> Self {
        self.i_frame_interval = interval;
        self
    }

    /// 设置分辨率限制
    pub fn with_bounds(mut self, width: i16, height: i16) -> Self {
        self.bounds = Some(Bounds { width, height });
        self
    }

    /// 转换为二进制 Buffer (Big Endian)
    /// 格式: 35 字节固定 + 可变长度的 codec_options 和 encoder_name
    pub fn to_buffer(&self) -> Vec<u8> {
        let codec_options_bytes = self.codec_options.as_bytes();
        let encoder_name_bytes = self.encoder_name.as_bytes();
        let codec_options_len = codec_options_bytes.len() as i32;
        let encoder_name_len = encoder_name_bytes.len() as i32;

        let total_length = 35 + codec_options_len as usize + encoder_name_len as usize;
        let mut buffer = Vec::with_capacity(total_length);

        // bitrate: 4 字节 (BE)
        buffer.write_i32::<BigEndian>(self.bitrate).unwrap();

        // max_fps: 4 字节 (BE)
        buffer.write_i32::<BigEndian>(self.max_fps).unwrap();

        // i_frame_interval: 1 字节
        buffer.write_i8(self.i_frame_interval).unwrap();

        // bounds: width(2) + height(2) (BE)
        let bounds_width = self.bounds.as_ref().map(|b| b.width).unwrap_or(0);
        let bounds_height = self.bounds.as_ref().map(|b| b.height).unwrap_or(0);
        buffer.write_i16::<BigEndian>(bounds_width).unwrap();
        buffer.write_i16::<BigEndian>(bounds_height).unwrap();

        // crop: left(2) + top(2) + right(2) + bottom(2) (BE)
        let crop_left = self.crop.as_ref().map(|c| c.left).unwrap_or(0);
        let crop_top = self.crop.as_ref().map(|c| c.top).unwrap_or(0);
        let crop_right = self.crop.as_ref().map(|c| c.right).unwrap_or(0);
        let crop_bottom = self.crop.as_ref().map(|c| c.bottom).unwrap_or(0);
        buffer.write_i16::<BigEndian>(crop_left).unwrap();
        buffer.write_i16::<BigEndian>(crop_top).unwrap();
        buffer.write_i16::<BigEndian>(crop_right).unwrap();
        buffer.write_i16::<BigEndian>(crop_bottom).unwrap();

        // send_frame_meta: 1 字节
        buffer.write_i8(if self.send_frame_meta { 1 } else { 0 }).unwrap();

        // locked_video_orientation: 1 字节
        buffer.write_i8(self.locked_video_orientation).unwrap();

        // display_id: 4 字节 (BE)
        buffer.write_i32::<BigEndian>(self.display_id).unwrap();

        // codec_options length + data: 4 字节长度 (BE) + 数据
        buffer.write_i32::<BigEndian>(codec_options_len).unwrap();
        buffer.write_all(codec_options_bytes).unwrap();

        // encoder_name length + data: 4 字节长度 (BE) + 数据
        buffer.write_i32::<BigEndian>(encoder_name_len).unwrap();
        buffer.write_all(encoder_name_bytes).unwrap();

        buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_settings_default() {
        let settings = VideoSettings::default();
        assert_eq!(settings.bitrate, 524288);
        assert_eq!(settings.max_fps, 24);
        assert_eq!(settings.i_frame_interval, 5);
    }

    #[test]
    fn test_video_settings_to_buffer() {
        let settings = VideoSettings::new()
            .with_bitrate(2097152)
            .with_max_fps(30)
            .with_i_frame_interval(5)
            .with_bounds(1920, 1920);

        let buffer = settings.to_buffer();

        // 验证基础长度 (35 字节固定包含了 2 个 4 字节长度字段 + 实际字符串数据)
        // 35 = 4(bitrate) + 4(fps) + 1(i_frame) + 4(bounds) + 8(crop) + 1(send_meta) + 1(orientation) + 4(display_id) + 4(codec_len) + 4(encoder_len)
        // 由于 codec_options 和 encoder_name 为空字符串，总长度 = 35
        assert_eq!(buffer.len(), 35);
    }
}
