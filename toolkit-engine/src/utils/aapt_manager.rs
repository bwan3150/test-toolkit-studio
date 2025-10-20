// aapt_manager.rs - AAPT 管理模块，处理内置 AAPT 的提取和路径管理

use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;
use tracing::{warn, debug};
use crate::{Result, TkeError};

// 引入构建时生成的 AAPT 二进制数据
include!(concat!(env!("OUT_DIR"), "/embedded_aapt.rs"));

pub struct AaptManager {
    aapt_path: PathBuf,
    is_bundled: bool,
}

impl AaptManager {
    /// 创建新的 AAPT 管理器，自动处理内置或系统 AAPT
    pub fn new() -> Result<Self> {
        if HAS_BUNDLED_AAPT {
            Self::create_with_bundled_aapt_silent()
        } else {
            Self::create_with_system_aapt_silent()
        }
    }

    /// 使用内置 AAPT 创建管理器（静默模式）
    fn create_with_bundled_aapt_silent() -> Result<Self> {
        // 获取临时目录来存放提取的 AAPT
        let temp_dir = std::env::temp_dir().join("tke_aapt");

        // 确保临时目录存在
        if !temp_dir.exists() {
            fs::create_dir_all(&temp_dir)
                .map_err(|e| TkeError::AaptError(format!("无法创建临时目录: {}", e)))?;
        }

        let aapt_path = temp_dir.join(AAPT_BINARY_NAME);

        // 检查是否需要重新提取 AAPT
        let should_extract = if aapt_path.exists() {
            // 检查文件大小是否匹配
            match fs::metadata(&aapt_path) {
                Ok(metadata) => metadata.len() != EMBEDDED_AAPT_BINARY.len() as u64,
                Err(_) => true,
            }
        } else {
            true
        };

        if should_extract {
            // 写入 AAPT 二进制文件
            let mut file = fs::File::create(&aapt_path)
                .map_err(|e| TkeError::AaptError(format!("无法创建 AAPT 文件: {}", e)))?;

            file.write_all(EMBEDDED_AAPT_BINARY)
                .map_err(|e| TkeError::AaptError(format!("无法写入 AAPT 文件: {}", e)))?;

            // 设置执行权限（Unix 系统）
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = file.metadata()
                    .map_err(|e| TkeError::AaptError(format!("无法获取文件权限: {}", e)))?
                    .permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&aapt_path, perms)
                    .map_err(|e| TkeError::AaptError(format!("无法设置执行权限: {}", e)))?;
            }
        }

        Ok(Self {
            aapt_path,
            is_bundled: true,
        })
    }

    /// 使用系统 AAPT 创建管理器（静默模式）
    fn create_with_system_aapt_silent() -> Result<Self> {
        let aapt_path = which::which("aapt")
            .map_err(|_| TkeError::AaptError(
                "系统中未找到 AAPT，请确保 AAPT 已安装并在 PATH 中".to_string()
            ))?;

        Ok(Self {
            aapt_path,
            is_bundled: false,
        })
    }

    /// 获取 AAPT 可执行文件路径
    pub fn aapt_path(&self) -> &Path {
        &self.aapt_path
    }

    /// 检查是否使用的是内置 AAPT
    pub fn is_bundled(&self) -> bool {
        self.is_bundled
    }

    /// 验证 AAPT 是否可用
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

impl Drop for AaptManager {
    fn drop(&mut self) {
        // 如果使用的是内置 AAPT，在程序结束时清理临时文件
        if self.is_bundled {
            if let Some(temp_dir) = self.aapt_path.parent() {
                if temp_dir.file_name() == Some("tke_aapt".as_ref()) {
                    if let Err(e) = fs::remove_dir_all(temp_dir) {
                        warn!("清理临时 AAPT 文件失败: {}", e);
                    } else {
                        debug!("已清理临时 AAPT 文件");
                    }
                }
            }
        }
    }
}
