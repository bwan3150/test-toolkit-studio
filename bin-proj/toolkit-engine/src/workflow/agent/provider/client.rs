// genai 客户端构建：把 provider/api_key/base_url → genai 适配器/认证/端点选择
// 这一层把 genai 的客户端构建细节隔离，session 只管对话。

use genai::adapter::AdapterKind;
use genai::resolver::{AuthData, AuthResolver, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};

use crate::utils::AiConfig;
use crate::Result;

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

    // ② 自定义端点：把请求打到别的地址去，**保留各家原生适配器**。
    //
    // 两种用法：
    //   a) OpenAI 兼容端点（doubao 火山方舟 / qwen 百炼）——genai 不认识这两个名字，
    //      强制走 OpenAI 适配器；
    //   b) **网关/代理**（如测试管理平台的 AI 网关）——provider 照实写，只换端点。
    //      这条是 ADR-0023 的关键：平台做原生透传，用户的 key 留在平台不下发到节点，
    //      节点拿到的是一个短期任务令牌。**必须保留原生适配器**，否则 anthropic 的
    //      思考块会丢（历史坑：genai 丢思考块 → anthropic 必须 4.6 + adaptive）。
    //      早先这里对非 doubao/qwen 直接报错，等于把网关这条路堵死了。
    if let Some(base_url) = cfg.base_url.clone() {
        let kind = adapter_kind(&provider);
        let key = cfg.api_key.clone();
        let model_for_target = model.clone();
        let target_resolver = ServiceTargetResolver::from_resolver_fn(
            move |service_target: ServiceTarget| -> std::result::Result<ServiceTarget, genai::resolver::Error> {
                let endpoint = Endpoint::from_owned(base_url.clone());
                let auth = match &key {
                    Some(k) => AuthData::from_single(k.clone()),
                    None => service_target.auth,
                };
                let model = ModelIden::new(kind, model_for_target.clone());
                Ok(ServiceTarget { endpoint, auth, model })
            },
        );
        builder = builder.with_service_target_resolver(target_resolver);
    }

    Ok((builder.build(), model))
}

/// provider → genai 适配器。配了 `base_url` 时用它决定"换了端点还按谁的协议说话"。
///
/// **认不出来的一律当 OpenAI 兼容**：那是事实标准，第三方网关九成走它；
/// 猜成别的只会给出更难懂的报错。
fn adapter_kind(provider: &str) -> AdapterKind {
    match provider {
        "anthropic" | "claude" => AdapterKind::Anthropic,
        "gemini" | "google" => AdapterKind::Gemini,
        "deepseek" => AdapterKind::DeepSeek,
        "xai" | "grok" => AdapterKind::Xai,
        "groq" => AdapterKind::Groq,
        "ollama" => AdapterKind::Ollama,
        // doubao 火山方舟 / qwen 百炼 genai 不认识名字，但它们就是 OpenAI 兼容端点
        _ => AdapterKind::OpenAI,
    }
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

#[cfg(test)]
mod adapter_tests {
    use super::*;

    /// 配了 `base_url` 时**保留原生适配器** —— 这是 AI 网关那条路的前提。
    /// 换成 OpenAI 兼容协议会丢掉 anthropic 的思考块（历史坑），
    /// 而思考块正是 harness 质量的来源之一
    #[test]
    fn 网关端点保留各家原生协议() {
        assert_eq!(adapter_kind("anthropic"), AdapterKind::Anthropic);
        assert_eq!(adapter_kind("gemini"), AdapterKind::Gemini);
        assert_eq!(adapter_kind("deepseek"), AdapterKind::DeepSeek);
        // 这两个 genai 不认识名字，但它们本来就是 OpenAI 兼容端点
        assert_eq!(adapter_kind("doubao"), AdapterKind::OpenAI);
        assert_eq!(adapter_kind("qwen"), AdapterKind::OpenAI);
        // 认不出来的当 OpenAI 兼容：事实标准，第三方网关九成走它
        assert_eq!(adapter_kind("某个没见过的网关"), AdapterKind::OpenAI);
    }

    /// 早先 `base_url` 对非 doubao/qwen 直接报错，等于把网关这条路堵死了。
    /// 现在任何 provider 都能配端点——**这条测的是"不再报错"**
    #[test]
    fn 任何供应商都能改端点() {
        for provider in ["anthropic", "openai", "gemini", "doubao", "自建网关"] {
            let cfg = AiConfig {
                provider: Some(provider.into()),
                model: Some("m".into()),
                api_key: Some("任务令牌".into()),
                base_url: Some("https://platform.example/api/v1/ai/proxy".into()),
                ..Default::default()
            };
            assert!(build_client(&cfg).is_ok(), "{provider} 配 base_url 不该被拒");
        }
    }
}
