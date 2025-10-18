// Parser 命令处理器

use tke::{Result, TkeError, ScriptParser, JsonOutput};
use std::path::PathBuf;

/// Parser 命令枚举
#[derive(clap::Subcommand)]
pub enum ParserCommands {
    /// 解析TKS脚本文件
    Parse {
        /// 脚本文件路径
        script_path: PathBuf,
    },
    /// 验证脚本语法
    Validate {
        /// 脚本文件路径
        script_path: PathBuf,
    },
    /// 获取语法高亮信息
    Highlight {
        /// 脚本文件路径
        script_path: PathBuf,
    },
}

/// 处理 Parser 相关命令
pub async fn handle(action: ParserCommands) -> Result<()> {
    let parser = ScriptParser::new();

    match action {
        ParserCommands::Parse { script_path } => {
            let script = parser.parse_file(&script_path)?;

            // 输出JSON格式的解析结果
            JsonOutput::print(serde_json::json!({
                "success": true,
                "case_id": script.case_id,
                "script_name": script.script_name,
                "details": script.details,
                "steps": script.steps.iter().map(|step| serde_json::json!({
                    "command": step.raw,
                    "line_number": step.line_number,
                    "command_type": step.command,
                    "params": step.params
                })).collect::<Vec<_>>()
            }));
        }
        ParserCommands::Validate { script_path } => {
            let script = parser.parse_file(&script_path)?;

            // 基本验证
            let mut warnings = Vec::new();
            if script.case_id.is_empty() {
                warnings.push("脚本缺少用例ID");
            }
            if script.script_name.is_empty() {
                warnings.push("脚本缺少脚本名");
            }
            if script.steps.is_empty() {
                JsonOutput::print(serde_json::json!({
                    "valid": false,
                    "error": "脚本没有定义任何步骤"
                }));
                return Err(TkeError::ScriptParseError("脚本验证失败".to_string()));
            }

            JsonOutput::print(serde_json::json!({
                "valid": true,
                "case_id": script.case_id,
                "script_name": script.script_name,
                "steps_count": script.steps.len(),
                "warnings": warnings
            }));
        }
        ParserCommands::Highlight { script_path } => {
            let content = std::fs::read_to_string(&script_path)
                .map_err(|e| TkeError::IoError(e))?;
            let highlights = parser.get_syntax_highlights(&content);

            // 输出JSON格式的语法高亮信息
            JsonOutput::print(&highlights);
        }
    }

    Ok(())
}
