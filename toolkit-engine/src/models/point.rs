// 坐标和边界框数据结构

use serde::{Deserialize, Serialize};

/// 坐标点
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

/// 边界框
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

    /// 获取中心点坐标
    pub fn center(&self) -> Point {
        Point {
            x: (self.x1 + self.x2) / 2,
            y: (self.y1 + self.y2) / 2,
        }
    }

    /// 获取宽度
    pub fn width(&self) -> i32 {
        self.x2 - self.x1
    }

    /// 获取高度
    pub fn height(&self) -> i32 {
        self.y2 - self.y1
    }

    /// 判断是否可见（宽高都大于0）
    pub fn is_visible(&self) -> bool {
        self.width() > 0 && self.height() > 0
    }
}
