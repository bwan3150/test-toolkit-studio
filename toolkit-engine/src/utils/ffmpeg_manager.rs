// ffmpeg_manager.rs - FFmpeg 管理模块，处理内置 FFmpeg 的提取和路径管理

use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;
use crate::{Result, TkeError};

// 引入构建时生成的 FFmpeg 二进制数据
include!(concat!(env!("OUT_DIR"), "/embedded_ffmpeg.rs"));

pub struct FfmpegManager {
    ffmpeg_path: PathBuf,
    is_bundled: bool,
}

impl FfmpegManager {
    /// 创建新的 FFmpeg 管理器，自动处理内置或系统 FFmpeg
    pub fn new() -> Result<Self> {
        if HAS_BUNDLED_FFMPEG {
            Self::create_with_bundled_ffmpeg_silent()
        } else {
            Self::create_with_system_ffmpeg_silent()
        }
    }

    /// 使用内置 FFmpeg 创建管理器（静默模式）
    fn create_with_bundled_ffmpeg_silent() -> Result<Self> {
        // 获取临时目录来存放提取的 FFmpeg
        let temp_dir = std::env::temp_dir().join("tke_ffmpeg");

        // 确保临时目录存在
        if !temp_dir.exists() {
            fs::create_dir_all(&temp_dir)
                .map_err(|e| TkeError::AdbError(format!("无法创建临时目录: {}", e)))?;
        }

        let ffmpeg_path = temp_dir.join(FFMPEG_BINARY_NAME);

        // 检查是否需要重新提取 FFmpeg
        let should_extract = if ffmpeg_path.exists() {
            // 检查文件大小是否匹配
            match fs::metadata(&ffmpeg_path) {
                Ok(metadata) => metadata.len() != EMBEDDED_FFMPEG_BINARY.len() as u64,
                Err(_) => true,
            }
        } else {
            true
        };

        if should_extract {
            // 写入 FFmpeg 二进制文件
            let mut file = fs::File::create(&ffmpeg_path)
                .map_err(|e| TkeError::AdbError(format!("无法创建 FFmpeg 文件: {}", e)))?;

            file.write_all(EMBEDDED_FFMPEG_BINARY)
                .map_err(|e| TkeError::AdbError(format!("无法写入 FFmpeg 文件: {}", e)))?;

            // 设置执行权限（Unix 系统）
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = file.metadata()
                    .map_err(|e| TkeError::AdbError(format!("无法获取文件权限: {}", e)))?
                    .permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&ffmpeg_path, perms)
                    .map_err(|e| TkeError::AdbError(format!("无法设置执行权限: {}", e)))?;
            }
        }

        Ok(Self {
            ffmpeg_path,
            is_bundled: true,
        })
    }

    /// 使用系统 FFmpeg 创建管理器（静默模式）
    fn create_with_system_ffmpeg_silent() -> Result<Self> {
        let ffmpeg_path = which::which("ffmpeg")
            .map_err(|_| TkeError::AdbError(
                "系统中未找到 FFmpeg，请确保 FFmpeg 已安装并在 PATH 中".to_string()
            ))?;

        Ok(Self {
            ffmpeg_path,
            is_bundled: false,
        })
    }

    /// 获取 FFmpeg 可执行文件路径
    pub fn ffmpeg_path(&self) -> &Path {
        &self.ffmpeg_path
    }

    /// 检查是否使用的是内置 FFmpeg
    pub fn is_bundled(&self) -> bool {
        self.is_bundled
    }

    /// 验证 FFmpeg 是否可用
    pub fn verify_ffmpeg(&self) -> Result<String> {
        use std::process::Command;

        let output = Command::new(&self.ffmpeg_path)
            .arg("-version")
            .output()
            .map_err(|e| TkeError::AdbError(format!("无法执行 FFmpeg: {}", e)))?;

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            let version_line = version.lines().next().unwrap_or("未知版本");
            Ok(version_line.to_string())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(TkeError::AdbError(format!("FFmpeg 验证失败: {}", error)))
        }
    }
}
