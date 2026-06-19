// 【提示词】主系统提示词 / 各级角色(subagent)提示词 / 工具提示词，全部可自定义
//
// 优先级（高 → 低）：CLI 直接注入文本 > CLI 指定 .md 文件 > prompts_dir 目录文件 > 内置默认
// 当前为单 agent（角色名 explorer）；roles 结构已为将来多 subagent 预留——
// 加角色时只需在 defaults 增默认、在 <prompts_dir>/agents/<role>.md 放覆盖即可。

pub mod defaults;
pub mod source;

use std::path::PathBuf;

use crate::Result;

/// 提示词来源说明（由 CLI/config 组装传入）
#[derive(Debug, Clone, Default)]
pub struct PromptSpec {
    /// 直接注入的主系统提示词文本（最高优先级）
    pub system_text: Option<String>,
    /// 主系统提示词的 .md 文件路径
    pub system_file: Option<PathBuf>,
    /// 提示词目录（约定 agents/*.md、tools/*.md）
    pub prompts_dir: Option<PathBuf>,
}

/// 解析后的提示词集合（运行时取用）
pub struct PromptSet {
    /// 主系统提示词（角色 explorer）的"原始模板"，含 {device}/{platform} 占位
    system_raw: String,
    /// 提示词目录（用于按需取工具描述覆盖）
    prompts_dir: Option<PathBuf>,
}

impl PromptSet {
    /// 按优先级解析出最终生效的提示词集合
    pub fn resolve(spec: &PromptSpec) -> Result<Self> {
        let system_raw = if let Some(text) = &spec.system_text {
            text.clone()
        } else if let Some(file) = &spec.system_file {
            source::read_file(file)?
        } else if let Some(dir) = &spec.prompts_dir {
            source::role_override(dir, "explorer")
                .unwrap_or_else(|| defaults::DEFAULT_SYSTEM.trim().to_string())
        } else {
            defaults::DEFAULT_SYSTEM.trim().to_string()
        };

        Ok(Self {
            system_raw,
            prompts_dir: spec.prompts_dir.clone(),
        })
    }

    /// 渲染主系统提示词（角色 explorer）：替换 {device} / {platform} 占位
    pub fn system(&self, device: &str, platform: &str) -> String {
        self.role_system("explorer", device, platform)
    }

    /// 渲染某角色的系统提示词：目录覆盖优先，否则内置默认；替换 {device} / {platform} 占位。
    /// explorer 角色复用构造时已解析好的 system_raw（含 CLI 注入文本/文件覆盖）；
    /// 其它角色（如 doctor）走 <prompts_dir>/agents/<role>.md 覆盖 → 内置默认。
    pub fn role_system(&self, role: &str, device: &str, platform: &str) -> String {
        let raw = if role == "explorer" {
            self.system_raw.clone()
        } else {
            self.prompts_dir
                .as_ref()
                .and_then(|d| source::role_override(d, role))
                .unwrap_or_else(|| defaults::default_role_system(role).trim().to_string())
        };
        raw.replace("{device}", device).replace("{platform}", platform)
    }

    /// 取某工具的 description（角色 explorer）：目录覆盖优先，否则内置默认
    pub fn tool_description(&self, name: &str) -> String {
        self.role_tool_description("explorer", name)
    }

    /// 取某角色某工具的 description：目录覆盖优先（explorer→tools/<name>.md，其它→tools/<role>/<name>.md），
    /// 否则内置默认。
    pub fn role_tool_description(&self, role: &str, name: &str) -> String {
        self.prompts_dir
            .as_ref()
            .and_then(|d| source::tool_override_role(d, role, name))
            .unwrap_or_else(|| defaults::default_tool_description_role(role, name).trim().to_string())
    }
}
