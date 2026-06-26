// 【提示词】primary(编排官)系统提示词 / 各 worker 角色提示词 / 工具提示词，全部可自定义
//
// 多 agent、全角色对称：primary = orchestrator（CLI --system-prompt 覆盖它）；
// explorer/doctor/reflector/supervisor/asserter/optimizer 都是被调度的 worker，
// 只能经 <prompts_dir>/agents/<role>.md 覆盖。
// 优先级（高 → 低）：[仅 primary] CLI 注入文本 > CLI .md 文件 > <prompts_dir>/agents/<role>.md > 内置默认。
// 加角色时只需在 defaults 增默认、在 <prompts_dir>/agents/<role>.md 放覆盖即可。

pub mod defaults;
pub mod source;

use std::path::PathBuf;

use crate::Result;

/// 提示词来源说明（由 CLI/config 组装传入）
#[derive(Debug, Clone, Default)]
pub struct PromptSpec {
    /// 直接注入的 primary(编排官) 系统提示词文本（最高优先级；只覆盖 primary，不影响 worker）
    pub system_text: Option<String>,
    /// primary(编排官) 系统提示词的 .md 文件路径（worker 角色请用 prompts_dir）
    pub system_file: Option<PathBuf>,
    /// 提示词目录（约定 agents/<role>.md、tools/...、messages/...；覆盖任意角色）
    pub prompts_dir: Option<PathBuf>,
}

/// primary 角色：CLI --system-prompt / --system-prompt-file 覆盖的就是它（编排官）。
/// 其余角色（explorer/doctor/…）都是被它调度的 worker，只能经 <prompts_dir>/agents/<role>.md 覆盖。
const PRIMARY_ROLE: &str = "orchestrator";

/// 解析后的提示词集合（运行时取用）
pub struct PromptSet {
    /// CLI 注入文本 / 文件覆盖的「primary（编排官）系统提示词」原始模板，含 {device}/{platform} 占位；
    /// None = primary 也走 目录覆盖 > 内置默认（与其它角色同一套解析）。
    primary_override: Option<String>,
    /// 提示词目录（按角色惰性取系统提示/工具描述/消息覆盖）
    prompts_dir: Option<PathBuf>,
}

impl PromptSet {
    /// 按优先级解析出最终生效的提示词集合。
    /// CLI 文本/文件只决定 primary(编排官) 的覆盖；各角色的目录覆盖 + 内置默认在取用时按角色惰性解析。
    pub fn resolve(spec: &PromptSpec) -> Result<Self> {
        let primary_override = if let Some(text) = &spec.system_text {
            Some(text.clone())
        } else if let Some(file) = &spec.system_file {
            Some(source::read_file(file)?)
        } else {
            None
        };

        Ok(Self {
            primary_override,
            prompts_dir: spec.prompts_dir.clone(),
        })
    }

    /// 渲染 primary（编排官）系统提示词：替换 {device}/{platform} 占位。
    /// = role_system(PRIMARY_ROLE)；CLI --system-prompt / --system-prompt-file 覆盖的就是它。
    pub fn system(&self, device: &str, platform: &str) -> String {
        self.role_system(PRIMARY_ROLE, device, platform)
    }

    /// 渲染某角色的系统提示词（全角色对称）：
    /// primary 角色优先用 CLI 注入文本/文件(primary_override)；其余一律 目录覆盖 > 内置默认。
    /// 占位 {device}/{platform} 最后统一替换。
    pub fn role_system(&self, role: &str, device: &str, platform: &str) -> String {
        let dir_or_builtin = || {
            self.prompts_dir
                .as_ref()
                .and_then(|d| source::role_override(d, role))
                .unwrap_or_else(|| defaults::default_role_system(role).trim().to_string())
        };
        let raw = if role == PRIMARY_ROLE {
            self.primary_override.clone().unwrap_or_else(dir_or_builtin)
        } else {
            dir_or_builtin()
        };
        raw.replace("{device}", device).replace("{platform}", platform)
    }

    /// 取某角色某工具的 description：目录覆盖优先（explorer→tools/<name>.md，其它→tools/<role>/<name>.md），
    /// 否则内置默认。
    pub fn role_tool_description(&self, role: &str, name: &str) -> String {
        self.prompts_dir
            .as_ref()
            .and_then(|d| source::tool_override_role(d, role, name))
            .unwrap_or_else(|| defaults::default_tool_description_role(role, name).trim().to_string())
    }

    /// 取某角色某**运行时消息模板**（含 {占位符}）：目录覆盖优先（<dir>/messages/<role>/<name>.md），
    /// 否则内置默认。返回的模板已 trim（结构性空白由调用点用 `render` 填充时控制）。
    pub fn message(&self, role: &str, name: &str) -> String {
        self.prompts_dir
            .as_ref()
            .and_then(|d| source::message_override(d, role, name))
            .unwrap_or_else(|| defaults::default_message(role, name).trim().to_string())
    }
}

/// 渲染消息模板：把 `{key}` 依次替换为给定值（沿用 {device}/{platform} 那套，无外部模板引擎）。
/// 只替换显式给出的 key，模板里其它 `{...}`（如 JSON 示例 `{"goal_marker":"..."}`）原样保留。
pub fn render(tmpl: &str, vars: &[(&str, &str)]) -> String {
    let mut s = tmpl.to_string();
    for (k, v) in vars {
        s = s.replace(&format!("{{{}}}", k), v);
    }
    s
}
