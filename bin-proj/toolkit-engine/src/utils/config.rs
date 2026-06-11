// 配置文件加载 - --config <tke.toml>
// config 文件等同于自动输入这些 CLI 参数，显式 CLI 参数优先于 config

use crate::{Result, TkeError};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// tke.toml 配置
/// 示例:
///   device = "f64b3b4d"
///   element = "locator/element.json"   # 相对路径基于 config 文件所在目录
///   log = "logs"
#[derive(Debug, Default, Deserialize)]
pub struct TkeConfig {
    /// 目标设备 ID
    pub device: Option<String>,
    /// 元素库路径
    pub element: Option<PathBuf>,
    /// 产物输出目录
    pub log: Option<PathBuf>,
}

impl TkeConfig {
    /// 加载配置文件；其中的相对路径基于 config 文件所在目录解析
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            TkeError::InvalidArgument(format!("读取配置文件失败 {}: {}", path.display(), e))
        })?;

        let mut config: TkeConfig = toml::from_str(&content)
            .map_err(|e| TkeError::InvalidArgument(format!("配置文件解析失败: {}", e)))?;

        // 相对路径基于 config 文件所在目录
        let base = path.parent().unwrap_or(Path::new("."));
        config.element = config.element.map(|p| resolve(base, p));
        config.log = config.log.map(|p| resolve(base, p));

        Ok(config)
    }
}

fn resolve(base: &Path, p: PathBuf) -> PathBuf {
    if p.is_absolute() { p } else { base.join(p) }
}
