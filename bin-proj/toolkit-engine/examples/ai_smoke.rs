// genai 接入连通性自测（仅验证"统一大模型对接层"这一部分，不涉及设备/探索循环）
//
// 用途：在完整 AgentRunner 闭环建好之前，先单独确认 genai + anthropic 能打通，
//       含两条路径：① 纯文本回复  ② 工具调用(tool calling)。
//
// 运行：
//   export ANTHROPIC_API_KEY=sk-ant-...        # 你的 key（也可改用其它 provider 的环境变量）
//   cargo run --example ai_smoke               # 默认 anthropic / claude-sonnet-4-5
//   cargo run --example ai_smoke -- claude-haiku-4-5   # 指定模型
//
// 说明：本例把 key 从 ANTHROPIC_API_KEY 读出后塞进 AiConfig.api_key，
//       因此顺带验证了 provider.rs 里的"自定义 AuthResolver"代码路径。

use tke::{AiConfig, LlmReply, LlmSession, LlmTool};

#[tokio::main]
async fn main() {
    // 模型名：命令行第一个参数，缺省 claude-sonnet-4-5
    let model = std::env::args().nth(1);

    let cfg = AiConfig {
        provider: Some("anthropic".to_string()),
        model: model.clone(),
        // 从环境变量取 key；若未设置则为 None，genai 会自行回落到 ANTHROPIC_API_KEY
        api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
        base_url: None,
        max_rounds: None,
        prompts_dir: None,
    };

    // ===== ① 纯文本回复 =====
    println!("== 测试 ① 纯文本回复 ==");
    {
        let mut sess = match LlmSession::new(&cfg, "你是一个测试助手，请用一句话回答。", vec![]) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("创建会话失败: {e}");
                std::process::exit(1);
            }
        };
        println!("模型: {}", sess.model());
        sess.user("用一句话说明自动化测试的价值。");
        match sess.next().await {
            Ok(LlmReply::Text(t)) => println!("文本回复: {t}"),
            Ok(LlmReply::ToolCalls { calls, .. }) => println!("(意外)收到工具调用: {calls:?}"),
            Err(e) => {
                eprintln!("请求失败: {e}");
                std::process::exit(1);
            }
        }
    }

    // ===== ② 工具调用 =====
    println!("\n== 测试 ② 工具调用 ==");
    {
        // 定义一个简单工具：模型应当用它来"点击某个元素"
        let click_tool = LlmTool::new(
            "click",
            "点击屏幕上的一个元素",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "element_id": { "type": "integer", "description": "元素在列表中的序号" },
                    "name": { "type": "string", "description": "给该元素起的语义名" }
                },
                "required": ["element_id", "name"]
            }),
        );

        let mut sess = match LlmSession::new(
            &cfg,
            "你在测试一个 App。当前页面有这些元素：[0] 文本 '登录' 按钮，[1] 输入框。\
             请只通过调用工具来操作，不要输出文本。",
            vec![click_tool],
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("创建会话失败: {e}");
                std::process::exit(1);
            }
        };
        sess.user("请点击登录按钮。");
        match sess.next().await {
            Ok(LlmReply::ToolCalls { calls, .. }) => {
                for c in &calls {
                    println!(
                        "工具调用: name={} call_id={} args={}",
                        c.name, c.call_id, c.arguments
                    );
                    // 回填一个假结果，验证回传路径不报错
                    sess.tool_result(c.call_id.as_str(), r#"{"success":true}"#);
                }
            }
            Ok(LlmReply::Text(t)) => println!("(模型未调用工具，返回文本): {t}"),
            Err(e) => {
                eprintln!("请求失败: {e}");
                std::process::exit(1);
            }
        }
    }

    println!("\n✓ genai 接入自测完成");
}
