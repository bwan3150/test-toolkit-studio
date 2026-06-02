// aapt_manager.rs - AAPT 路径管理
// 查找顺序：tke 同目录 → 系统 PATH
// 不再内嵌 AAPT 二进制，aapt 需与 tke 放在同一目录或安装到系统 PATH

use std::path::{Path, PathBuf};
use crate::{Result, TkeError};

#[cfg(target_os = "windows")]
const AAPT_BINARY_NAME: &str = "aapt.exe";
#[cfg(not(target_os = "windows"))]
const AAPT_BINARY_NAME: &str = "aapt";

pub struct AaptManager {
    aapt_path: PathBuf,
}

impl AaptManager {
    /// 查找 AAPT：优先同目录，其次系统 PATH
    pub fn new() -> Result<Self> {
        // 1. tke 同目录
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let candidate = exe_dir.join(AAPT_BINARY_NAME);
                if candidate.exists() {
                    return Ok(Self { aapt_path: candidate });
                }
            }
        }

        // 2. 系统 PATH
        if let Ok(aapt_path) = which::which(AAPT_BINARY_NAME) {
            return Ok(Self { aapt_path });
        }

        Err(TkeError::AaptError(
            format!("未找到 AAPT：请将 {} 放在与 tke 相同的目录下，或安装到系统 PATH 中", AAPT_BINARY_NAME)
        ))
    }

    pub fn aapt_path(&self) -> &Path {
        &self.aapt_path
    }

    pub fn verify_aapt(&self) -> Result<String> {
        use std::process::Command;

        let output = Command::new(&self.aapt_path)
            .arg("version")
            .output()
            .map_err(|e| TkeError::AaptError(format!("无法执行 AAPT: {}", e)))?;

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            let version_line = version.lines().next().unwrap_or("未知版本");
            Ok(version_line.to_string())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(TkeError::AaptError(format!("AAPT 验证失败: {}", error)))
        }
    }
}
