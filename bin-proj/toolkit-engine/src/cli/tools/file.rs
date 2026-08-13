// File 命令处理器

use tke::{Result, FileManager, JsonOutput};

/// File 命令枚举
#[derive(clap::Subcommand)]
pub enum FileCommands {
    /// 列出目录详细信息 (包含日期时间,类似 ls -l)
    Ls {
        /// 设备上的路径 (默认: /sdcard/)
        #[arg(default_value = "/sdcard/")]
        path: String,
    },

    /// 以树形结构列出目录内容 (类似 tree 命令)
    Tree {
        /// 设备上的路径 (默认: /sdcard/)
        #[arg(default_value = "/sdcard/")]
        path: String,

        /// 目录深度 (默认: 1)
        #[arg(short = 'L', long, default_value = "1")]
        level: usize,
    },

    /// 搜索文件 (例如: tke file find /sdcard/ "*.apk")
    Find {
        /// 搜索起始路径
        path: String,

        /// 文件名模式 (例如: "*.apk")
        pattern: String,
    },

    /// 读取文件内容
    Cat {
        /// 设备上的文件路径
        path: String,
    },

    /// 创建目录 (自动创建父目录)
    Mkdir {
        /// 设备上的目录路径
        path: String,
    },

    /// 删除文件或目录
    Rm {
        /// 设备上的文件/目录路径
        path: String,
    },

    /// 复制文件或目录
    Cp {
        /// 源路径
        source: String,

        /// 目标路径
        dest: String,
    },

    /// 移动/重命名文件
    Mv {
        /// 源路径
        source: String,

        /// 目标路径
        dest: String,
    },

    /// 写入文件内容
    Write {
        /// 设备上的文件路径
        path: String,

        /// 写入的内容
        content: String,
    },

    /// 从设备下载文件到本地
    Pull {
        /// 设备上的文件路径
        remote: String,

        /// 本地路径
        local: String,
    },

    /// 从本地上传文件到设备
    Push {
        /// 本地文件路径
        local: String,

        /// 设备上的路径
        remote: String,
    },

    /// 获取目录大小 (人类可读格式)
    Size {
        /// 设备上的目录路径
        path: String,
    },
}

/// 处理 File 相关命令
pub async fn handle(action: FileCommands, params: std::sync::Arc<tke::Params>) -> Result<()> {
    let device_id = params.device();
    let file_manager = FileManager::new(device_id)?;

    match action {
        FileCommands::Ls { path } => {
            let output = file_manager.list(&path)?;
            print!("{}", output);
        }
        FileCommands::Tree { path, level } => {
            let output = file_manager.tree(&path, level)?;
            print!("{}", output);
        }
        FileCommands::Find { path, pattern } => {
            let output = file_manager.find(&path, &pattern)?;
            print!("{}", output);
        }
        FileCommands::Cat { path } => {
            let output = file_manager.cat(&path)?;
            print!("{}", output);
        }
        FileCommands::Mkdir { path } => {
            file_manager.mkdir(&path)?;
            JsonOutput::print(serde_json::json!({
                "success": true,
                "message": format!("目录创建成功: {}", path),
                "path": path
            }));
        }
        FileCommands::Rm { path } => {
            file_manager.rm(&path)?;
            JsonOutput::print(serde_json::json!({
                "success": true,
                "message": format!("删除成功: {}", path),
                "path": path
            }));
        }
        FileCommands::Cp { source, dest } => {
            file_manager.cp(&source, &dest)?;
            JsonOutput::print(serde_json::json!({
                "success": true,
                "message": format!("复制成功: {} -> {}", source, dest),
                "source": source,
                "dest": dest
            }));
        }
        FileCommands::Mv { source, dest } => {
            file_manager.mv(&source, &dest)?;
            JsonOutput::print(serde_json::json!({
                "success": true,
                "message": format!("移动成功: {} -> {}", source, dest),
                "source": source,
                "dest": dest
            }));
        }
        FileCommands::Write { path, content } => {
            file_manager.write(&path, &content)?;
            JsonOutput::print(serde_json::json!({
                "success": true,
                "message": format!("写入成功: {}", path),
                "path": path
            }));
        }
        FileCommands::Pull { remote, local } => {
            let output = file_manager.pull(&remote, &local)?;
            JsonOutput::print(serde_json::json!({
                "success": true,
                "message": format!("下载成功: {} -> {}", remote, local),
                "remote": remote,
                "local": local,
                "output": output.trim()
            }));
        }
        FileCommands::Push { local, remote } => {
            let output = file_manager.push(&local, &remote)?;
            JsonOutput::print(serde_json::json!({
                "success": true,
                "message": format!("上传成功: {} -> {}", local, remote),
                "local": local,
                "remote": remote,
                "output": output.trim()
            }));
        }
        FileCommands::Size { path } => {
            let output = file_manager.size(&path)?;
            print!("{}", output);
        }
    }

    Ok(())
}
