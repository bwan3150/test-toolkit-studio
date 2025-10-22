// adb_manager.rs - ADB 管理模块，处理内置 ADB 的提取和路径管理

use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;
use crate::{Result, TkeError};

// 引入构建时生成的 ADB 二进制数据
include!(concat!(env!("OUT_DIR"), "/embedded_adb.rs"));

pub struct AdbManager {
    adb_path: PathBuf,
    is_bundled: bool,
}

impl AdbManager {
    /// 创建新的 ADB 管理器，自动处理内置或系统 ADB
    pub fn new() -> Result<Self> {
        if HAS_BUNDLED_ADB {
            Self::create_with_bundled_adb_silent()
        } else {
            Self::create_with_system_adb_silent()
        }
    }
    
    /// 使用内置 ADB 创建管理器（静默模式）
    fn create_with_bundled_adb_silent() -> Result<Self> {
        // 获取临时目录来存放提取的 ADB
        let temp_dir = std::env::temp_dir().join("tke_adb");

        // 确保临时目录存在
        if !temp_dir.exists() {
            fs::create_dir_all(&temp_dir)
                .map_err(|e| TkeError::AdbError(format!("无法创建临时目录: {}", e)))?;
        }

        let adb_path = temp_dir.join(ADB_BINARY_NAME);

        // 检查是否需要重新提取 ADB
        let should_extract = if adb_path.exists() {
            // 检查文件大小是否匹配
            match fs::metadata(&adb_path) {
                Ok(metadata) => metadata.len() != EMBEDDED_ADB_BINARY.len() as u64,
                Err(_) => true,
            }
        } else {
            true
        };

        if should_extract {
            // 写入 ADB 二进制文件
            let mut file = fs::File::create(&adb_path)
                .map_err(|e| TkeError::AdbError(format!("无法创建 ADB 文件: {}", e)))?;

            file.write_all(EMBEDDED_ADB_BINARY)
                .map_err(|e| TkeError::AdbError(format!("无法写入 ADB 文件: {}", e)))?;
            
            // 设置执行权限（Unix 系统）
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = file.metadata()
                    .map_err(|e| TkeError::AdbError(format!("无法获取文件权限: {}", e)))?
                    .permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&adb_path, perms)
                    .map_err(|e| TkeError::AdbError(format!("无法设置执行权限: {}", e)))?;
            }
        }

        Ok(Self {
            adb_path,
            is_bundled: true,
        })
    }

    /// 使用系统 ADB 创建管理器（静默模式）
    fn create_with_system_adb_silent() -> Result<Self> {
        let adb_path = which::which("adb")
            .map_err(|_| TkeError::AdbError(
                "系统中未找到 ADB，请确保 ADB 已安装并在 PATH 中".to_string()
            ))?;

        Ok(Self {
            adb_path,
            is_bundled: false,
        })
    }
    
    /// 获取 ADB 可执行文件路径
    pub fn adb_path(&self) -> &Path {
        &self.adb_path
    }
    
    /// 检查是否使用的是内置 ADB
    pub fn is_bundled(&self) -> bool {
        self.is_bundled
    }
    
    /// 验证 ADB 是否可用
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

// Drop 实现已移除
// 原因：多个 TKE 进程可能并发运行，不应该在单个进程结束时删除共享的临时 ADB 文件
// 临时文件会在系统重启时自动清理
//
// impl Drop for AdbManager {
//     fn drop(&mut self) {
//         // 如果使用的是内置 ADB，在程序结束时清理临时文件
//         if self.is_bundled {
//             if let Some(temp_dir) = self.adb_path.parent() {
//                 if temp_dir.file_name() == Some("tke_adb".as_ref()) {
//                     if let Err(e) = fs::remove_dir_all(temp_dir) {
//                         warn!("清理临时 ADB 文件失败: {}", e);
//                     } else {
//                         debug!("已清理临时 ADB 文件");
//                     }
//                 }
//             }
//         }
//     }
// }