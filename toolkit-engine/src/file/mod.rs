// File 模块 - Android 设备文件系统管理核心逻辑

use crate::{Result, TkeError, AdbManager};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize, Deserialize, Debug)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size: Option<String>,
    pub is_dir: bool,
}

#[derive(Debug)]
struct TreeNode {
    name: String,
    is_dir: bool,
    size: Option<String>,
    children: Vec<TreeNode>,
}

pub struct FileManager {
    device_id: Option<String>,
    adb_manager: AdbManager,
}

impl FileManager {
    pub fn new(device_id: Option<String>) -> Result<Self> {
        // 使用 AdbManager 获取 ADB
        let adb_manager = AdbManager::new()?;

        // 验证 ADB 可用性
        adb_manager.verify_adb()?;

        Ok(Self {
            device_id,
            adb_manager,
        })
    }

    // ========== 查询操作 ==========

    /// 以树形结构列出目录内容
    pub fn tree(&self, path: &str, max_depth: usize) -> Result<String> {
        let mut output = String::new();
        output.push_str(path);
        output.push('\n');

        let root = self.build_tree(path, 0, max_depth)?;
        let (dirs, files) = self.count_items(&root);

        self.print_tree(&root.children, "", true, &mut output);

        output.push_str(&format!("\n{} directories, {} files\n", dirs, files));

        Ok(output)
    }

    /// 构建目录树
    fn build_tree(&self, path: &str, current_depth: usize, max_depth: usize) -> Result<TreeNode> {
        let name = path.split('/').last().unwrap_or(path).to_string();

        if current_depth >= max_depth {
            return Ok(TreeNode {
                name,
                is_dir: true,
                size: None,
                children: vec![],
            });
        }

        // 获取目录内容
        let ls_output = self.exec_shell(&format!("ls -lA {}", path))?;
        let mut children = Vec::new();

        for line in ls_output.lines().skip(1) { // 跳过 "total" 行
            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 8 {
                continue;
            }

            let is_dir = parts[0].starts_with('d');
            let file_name = parts[7..].join(" ");

            // 跳过 . 和 ..
            if file_name == "." || file_name == ".." {
                continue;
            }

            let child_path = format!("{}/{}", path.trim_end_matches('/'), file_name);

            if is_dir {
                // 递归处理子目录
                match self.build_tree(&child_path, current_depth + 1, max_depth) {
                    Ok(child) => children.push(child),
                    Err(_) => {
                        // 如果无法访问,添加一个空节点
                        children.push(TreeNode {
                            name: file_name,
                            is_dir: true,
                            size: None,
                            children: vec![],
                        });
                    }
                }
            } else {
                // 文件,获取大小
                let size = if parts.len() > 4 {
                    Some(self.format_size(parts[4]))
                } else {
                    None
                };

                children.push(TreeNode {
                    name: file_name,
                    is_dir: false,
                    size,
                    children: vec![],
                });
            }
        }

        // 排序:目录在前,文件在后,同类按名称排序
        children.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        Ok(TreeNode {
            name,
            is_dir: true,
            size: None,
            children,
        })
    }

    /// 打印树形结构
    fn print_tree(&self, nodes: &[TreeNode], prefix: &str, is_last: bool, output: &mut String) {
        for (i, node) in nodes.iter().enumerate() {
            let is_last_child = i == nodes.len() - 1;

            // 当前项的前缀
            let current_prefix = if is_last_child { "└── " } else { "├── " };

            // 子项的前缀
            let child_prefix = if is_last_child { "    " } else { "│   " };

            // 输出当前项
            output.push_str(prefix);
            output.push_str(current_prefix);
            output.push_str(&node.name);

            if !node.is_dir {
                if let Some(ref size) = node.size {
                    output.push_str(&format!(" ({})", size));
                }
            }

            output.push('\n');

            // 递归打印子项
            if !node.children.is_empty() {
                let new_prefix = format!("{}{}", prefix, child_prefix);
                self.print_tree(&node.children, &new_prefix, is_last_child, output);
            }
        }
    }

    /// 统计目录和文件数量
    fn count_items(&self, node: &TreeNode) -> (usize, usize) {
        let mut dirs = 0;
        let mut files = 0;

        for child in &node.children {
            if child.is_dir {
                dirs += 1;
                let (sub_dirs, sub_files) = self.count_items(child);
                dirs += sub_dirs;
                files += sub_files;
            } else {
                files += 1;
            }
        }

        (dirs, files)
    }

    /// 格式化文件大小为人类可读格式
    fn format_size(&self, size_str: &str) -> String {
        if let Ok(size) = size_str.parse::<u64>() {
            const KB: u64 = 1024;
            const MB: u64 = KB * 1024;
            const GB: u64 = MB * 1024;

            if size >= GB {
                format!("{:.1}G", size as f64 / GB as f64)
            } else if size >= MB {
                format!("{:.1}M", size as f64 / MB as f64)
            } else if size >= KB {
                format!("{:.1}K", size as f64 / KB as f64)
            } else {
                format!("{}B", size)
            }
        } else {
            size_str.to_string()
        }
    }

    /// 搜索文件 (支持模糊搜索,自动添加通配符)
    pub fn find(&self, path: &str, pattern: &str) -> Result<String> {
        // 如果用户没有提供通配符,自动添加 *pattern*
        let search_pattern = if pattern.contains('*') {
            pattern.to_string()
        } else {
            format!("*{}*", pattern)
        };

        let cmd = format!("find {} -name '{}'", path, search_pattern);
        self.exec_shell(&cmd)
    }

    /// 读取文件内容
    pub fn cat(&self, path: &str) -> Result<String> {
        let cmd = format!("cat {}", path);
        self.exec_shell(&cmd)
    }

    /// 获取目录大小(人类可读格式)
    pub fn size(&self, path: &str) -> Result<String> {
        let cmd = format!("du -sh {}", path);
        self.exec_shell(&cmd)
    }

    // ========== 修改操作 ==========

    /// 创建目录(自动创建父目录)
    pub fn mkdir(&self, path: &str) -> Result<()> {
        let cmd = format!("mkdir -p {}", path);
        self.exec_shell(&cmd)?;
        Ok(())
    }

    /// 删除文件或目录(自动递归删除)
    pub fn rm(&self, path: &str) -> Result<()> {
        let cmd = format!("rm -rf {}", path);
        self.exec_shell(&cmd)?;
        Ok(())
    }

    /// 复制文件(自动递归复制)
    pub fn cp(&self, source: &str, dest: &str) -> Result<()> {
        let cmd = format!("cp -r {} {}", source, dest);
        self.exec_shell(&cmd)?;
        Ok(())
    }

    /// 移动/重命名文件
    pub fn mv(&self, source: &str, dest: &str) -> Result<()> {
        let cmd = format!("mv {} {}", source, dest);
        self.exec_shell(&cmd)?;
        Ok(())
    }

    /// 写入文件内容(覆盖模式)
    pub fn write(&self, path: &str, content: &str) -> Result<()> {
        let cmd = format!("echo '{}' > {}", content, path);
        self.exec_shell(&cmd)?;
        Ok(())
    }

    // ========== 文件传输操作 ==========

    /// 从设备下载文件到本地
    pub fn pull(&self, remote: &str, local: &str) -> Result<String> {
        let args = vec!["pull", remote, local];
        self.run_adb_command_output(&args)
    }

    /// 从本地上传文件到设备
    pub fn push(&self, local: &str, remote: &str) -> Result<String> {
        let args = vec!["push", local, remote];
        self.run_adb_command_output(&args)
    }

    // ========== 辅助函数 ==========

    /// 执行 shell 命令
    fn exec_shell(&self, cmd: &str) -> Result<String> {
        let args = vec!["shell", cmd];
        self.run_adb_command_output(&args)
    }

    /// 执行 ADB 命令并获取输出
    fn run_adb_command_output(&self, args: &[&str]) -> Result<String> {
        let mut cmd = Command::new(self.adb_manager.adb_path());

        // 如果指定了设备ID,添加-s参数
        if let Some(ref device_id) = self.device_id {
            cmd.arg("-s").arg(device_id);
        }

        cmd.args(args);

        let output = cmd.output()
            .map_err(|e| TkeError::AdbError(format!("执行ADB命令失败: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TkeError::AdbError(format!("ADB命令执行失败: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
