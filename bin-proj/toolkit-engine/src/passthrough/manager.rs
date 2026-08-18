// Tools 模块 - 通用外部工具直通
// tke 作为所有测试工具的统一入口/协调器：
//   tke adb <原生adb指令> / tke k6 <原生k6指令> / tke ffmpeg ... 等
// 工具二进制与 tke 放在同一目录（bin/<platform>/），新增工具零代码改动

use crate::{Result, TkeError};
use std::path::PathBuf;

/// 通用工具管理器：按名称解析 tke 同目录下的二进制
pub struct ToolManager;

impl ToolManager {
    /// 解析工具二进制路径
    /// 查找顺序：同目录 <name> → 同目录 <name>.exe → 同目录 tke-<name>
    /// （tke-<name> 支持 `tke opencv` → tke-opencv、`tke scrcpy` → tke-scrcpy 简写）
    pub fn resolve(name: &str) -> Result<PathBuf> {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .ok_or_else(|| TkeError::InvalidArgument("无法获取 tke 所在目录".to_string()))?;

        let candidates = [
            exe_dir.join(name),
            exe_dir.join(format!("{}.exe", name)),
            exe_dir.join(format!("tke-{}", name)),
            exe_dir.join(format!("tke-{}.exe", name)),
        ];

        for candidate in &candidates {
            if candidate.is_file() {
                return Ok(candidate.clone());
            }
        }

        // 指向 doctor 而不是列一串"目录里现在有什么"——缺依赖时人要的是**怎么补**，
        // 不是"还缺哪些"的清单
        Err(TkeError::InvalidArgument(format!(
            "缺少 {}（要和 tke 放在同一目录）。补齐：tke doctor --fix",
            name
        )))
    }


}
