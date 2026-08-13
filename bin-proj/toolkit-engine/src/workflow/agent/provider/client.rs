// genai 客户端构建：把 provider/api_key/base_url → genai 适配器/认证/端点选择
// 这一层把 genai 的客户端构建细节隔离，session 只管对话。

use genai::adapter::AdapterKind;
use genai::resolver::{AuthData, AuthResolver, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};

use crate::utils::AiConfig;
use crate::{Result, TkeError};

/// 按 provider/api_key/base_url 构建 genai 客户端，返回 (客户端, 解析后的模型名)
pub(super) fn build_client(cfg: &AiConfig) -> Result<(Client, String)> {
    let provider = cfg
        .provider
        .as_deref()
        .unwrap_or("anthropic")
        .to_lowercase();
    let model = cfg
        .model
        .clone()
        .unwrap_or_else(|| default_model(&provider));

    let mut builder = Client::builder();

    // ① 自定义 API Key（缺省回落到各家标准环境变量）
    if let Some(key) = cfg.api_key.clone() {
        let auth_resolver = AuthResolver::from_resolver_fn(
            move |_model_iden: ModelIden| -> std::result::Result<Option<AuthData>, genai::resolver::Error> {
                Ok(Some(AuthData::from_single(key.clone())))
            },
        );
        builder = builder.with_auth_resolver(auth_resolver);
    }

    // ② 自定义端点：OpenAI 兼容端点（doubao 火山方舟 / qwen 百炼）
    //    强制走 OpenAI 适配器，并改写 endpoint / auth / model
    if let Some(base_url) = cfg.base_url.clone() {
        if !is_openai_compatible(&provider) {
            return Err(TkeError::InvalidArgument(format!(
                "[ai].base_url 仅用于 OpenAI 兼容端点(doubao/qwen)，provider={} 无需配置",
                provider
            )));
        }
        let key = cfg.api_key.clone();
        let model_for_target = model.clone();
        let target_resolver = ServiceTargetResolver::from_resolver_fn(
            move |service_target: ServiceTarget| -> std::result::Result<ServiceTarget, genai::resolver::Error> {
                let endpoint = Endpoint::from_owned(base_url.clone());
                let auth = match &key {
                    Some(k) => AuthData::from_single(k.clone()),
                    None => service_target.auth,
                };
                let model = ModelIden::new(AdapterKind::OpenAI, model_for_target.clone());
                Ok(ServiceTarget { endpoint, auth, model })
            },
        );
        builder = builder.with_service_target_resolver(target_resolver);
    }

    Ok((builder.build(), model))
}

/// provider 是否为"OpenAI 兼容端点"（需配 base_url）
fn is_openai_compatible(provider: &str) -> bool {
    matches!(provider, "doubao" | "qwen")
}

/// 各 provider 的缺省模型（未配 [ai].model 时使用；建议显式配置）
/// 注意：模型名需为对应服务商 API 接受的真实 id。
/// anthropic 用 4-6（adaptive thinking 路径）：开 reasoning 时 genai 发 `thinking:{type:adaptive}`，
/// 不强制把思考块随工具回带；而旧的 4-5 走 legacy budget_tokens 严格模式，配 reasoning + 多轮工具会 400。
fn default_model(provider: &str) -> String {
    match provider {
        "anthropic" => "claude-sonnet-4-6",
        // gpt-5.x 是 reasoning 模型，接受 reasoning_effort；旧的 gpt-4o 非 reasoning，
        // reasoning 默认常开下会 400，故缺省升到 gpt-5.5-mini（mini 省钱）。
        "openai" => "gpt-5.5-mini",
        "gemini" => "gemini-2.5-flash",
        "deepseek" => "deepseek-chat",
        // doubao/qwen 无通用缺省模型，必须由 [ai].model 指定
        _ => "claude-sonnet-4-6",
    }
    .to_string()
}
