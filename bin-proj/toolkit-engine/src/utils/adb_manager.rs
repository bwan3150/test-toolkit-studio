// adb_manager.rs - ADB 路径管理
// 查找顺序：tke 同目录 → 系统 PATH
// 不再内嵌 ADB 二进制，adb 需与 tke 放在同一目录或安装到系统 PATH

use std::path::{Path, PathBuf};
use crate::{Result, TkeError};

#[cfg(target_os = "windows")]
const ADB_BINARY_NAME: &str = "adb.exe";
#[cfg(not(target_os = "windows"))]
const ADB_BINARY_NAME: &str = "adb";

pub struct AdbManager {
    adb_path: PathBuf,
}

impl AdbManager {
    /// 查找 ADB：优先同目录，其次系统 PATH
    /// 只在 tke 同目录查找，不回退到系统 PATH（确保使用指定版本）
    pub fn new() -> Result<Self> {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let candidate = exe_dir.join(ADB_BINARY_NAME);
                if candidate.exists() {
                    return Ok(Self { adb_path: candidate });
                }
            }
        }

        Err(TkeError::AdbError(
            format!("未找到 ADB：请将 {} 放在与 tke 相同的目录下", ADB_BINARY_NAME)
        ))
    }

    pub fn adb_path(&self) -> &Path {
        &self.adb_path
    }

    pub fn verify_adb(&self) -> Result<String> {
        use std::process::Command;

        let output = Command::new(&self.adb_path)
            .arg("version")
            .output()
            .map_err(|e| TkeError::AdbError(format!("无法执行 ADB: {}", e)))?;

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            let version_line = version.lines().next().unwrap_or("未知版本");
            Ok(version_line.to_string())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(TkeError::AdbError(format!("ADB 验证失败: {}", error)))
        }
    }
}
