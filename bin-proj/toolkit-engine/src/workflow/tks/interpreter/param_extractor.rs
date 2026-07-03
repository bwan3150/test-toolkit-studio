// 参数提取模块 - 从 TksParam 中提取各类参数

use crate::{Result, TkeError, TksParam};

/// 参数提取器
pub struct ParamExtractor;

impl ParamExtractor {
    /// 提取文本参数
    pub fn extract_text(param: &TksParam) -> Result<String> {
        match param {
            TksParam::Text(s) => Ok(s.clone()),
            _ => Err(TkeError::InvalidArgument("期望文本参数".to_string())),
        }
    }

    /// 提取数字参数
    pub fn extract_number(param: &TksParam) -> Result<i32> {
        match param {
            TksParam::Number(n) => Ok(*n),
            _ => Err(TkeError::InvalidArgument("期望数字参数".to_string())),
        }
    }

    /// 提取持续时间参数
    pub fn extract_duration(param: &TksParam) -> Result<u32> {
        match param {
            TksParam::Duration(ms) => Ok(*ms),
            TksParam::Number(n) => Ok(*n as u32),
            _ => Err(TkeError::InvalidArgument("期望持续时间参数".to_string())),
        }
    }

    /// 提取方向参数
    pub fn extract_direction(param: &TksParam) -> Result<String> {
        match param {
            TksParam::Direction(d) => Ok(d.clone()),
            TksParam::Text(t) => Ok(t.clone()),
            _ => Err(TkeError::InvalidArgument("期望方向参数".to_string())),
        }
    }
}
