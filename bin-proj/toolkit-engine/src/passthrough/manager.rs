// Tools 模块 - 通用外部工具直通
// tke 作为所有测试工具的统一入口/协调器：
//   tke adb <原生adb指令> / tke k6 <原生k6指令> / tke ffmpeg ... 等
// 工具二进制与 tke 放在同一目录（bin/<platform>/），新增工具零代码改动

use crate::{Result, TkeError};
use std::path::PathBuf;
use std::process::Command;

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

        Err(TkeError::InvalidArgument(format!(
            "{} 可执行文件缺失或不完整：请将其放在与 tke 相同的目录下。当前可用: {}",
            name,
            Self::list_available().join(", ")
        )))
    }

    /// 列出 tke 同目录下所有可执行工具（排除 tke 自身）
    pub fn list_available() -> Vec<String> {
        let mut tools = Vec::new();

        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                if let Ok(entries) = std::fs::read_dir(exe_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if !path.is_file() {
                            continue;
                        }
                        let name = match path.file_name().and_then(|n| n.to_str()) {
                            Some(n) => n.trim_end_matches(".exe").to_string(),
                            None => continue,
                        };
                        // 排除 tke 自身和非可执行资源文件（如 .jar/.dll）
                        if name == "tke" || name.contains('.') {
                            continue;
                        }
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            if let Ok(meta) = path.metadata() {
                                if meta.permissions().mode() & 0o111 == 0 {
                                    continue;
                                }
                            }
                        }
                        tools.push(name);
                    }
                }
            }
        }

        tools.sort();
        tools.dedup();
        tools
    }

    /// 直通执行：继承标准输入输出，以工具自身退出码退出（不返回）
    ///
    /// device_id 仅对支持 -s 的工具（adb）注入设备参数
    pub fn passthrough(name: &str, args: Vec<String>, device_id: Option<String>) -> Result<()> {
        let tool_path = Self::resolve(name)?;

        let mut command = Command::new(&tool_path);

        // adb 特例：-d/--device 转为 adb -s <id>
        if name == "adb" {
            if let Some(device) = device_id {
                command.arg("-s").arg(device);
            }
        }

        command.args(&args);

        let status = command.status().map_err(|e| {
            TkeError::InvalidArgument(format!("执行工具 '{}' 失败: {}", name, e))
        })?;

        std::process::exit(status.code().unwrap_or(1));
    }
}
