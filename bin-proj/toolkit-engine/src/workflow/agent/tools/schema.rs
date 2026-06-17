// 各工具的 JSON Schema（description 不在此处，由 prompt 模块提供，便于自定义）
//
// 设备动作工具与 .tks 命令一一对齐（launch/click/input/... ），保证每步可落成可回放的 .tks。
// 控制流工具：request_screenshot（要图）/ ask_user（反问）/ finish（结束）。

/// 一个工具的"名字 + 参数 Schema"（description 在 PromptSet 里取）
pub struct ToolSchema {
    pub name: &'static str,
    pub schema: serde_json::Value,
}

/// 全部工具的 name + schema 表
pub fn tool_schemas() -> Vec<ToolSchema> {
    // 针对元素的工具都需要 element_id + name(+desc)，extra 注入各自附加字段
    let el_props = |extra: serde_json::Value| -> serde_json::Value {
        let mut base = serde_json::json!({
            "element_id": { "type": "integer", "description": "页面元素列表中的序号" },
            "name": { "type": "string", "description": "给该元素起的稳定语义名（落库并写进 .tks，如 '登录按钮'）" },
            "desc": { "type": "string", "description": "可选、可留空——系统会在探索结束后据元素的实际作用统一生成 desc，无需你在此猜测。" }
        });
        if let (Some(obj), Some(ex)) = (base.as_object_mut(), extra.as_object()) {
            for (k, v) in ex {
                obj.insert(k.clone(), v.clone());
            }
        }
        base
    };

    let obj = |props: serde_json::Value, required: serde_json::Value| {
        serde_json::json!({ "type": "object", "properties": props, "required": required })
    };

    vec![
        ToolSchema {
            name: "launch",
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "target": { "type": "string", "description": "包名 / URL / bundleId" },
                    "activity": { "type": "string", "description": "可选：Android Activity，如 .MainActivity" }
                },
                "required": ["target"]
            }),
        },
        ToolSchema {
            name: "close",
            schema: serde_json::json!({
                "type": "object",
                "properties": { "target": { "type": "string", "description": "包名 / URL / bundleId" } },
                "required": ["target"]
            }),
        },
        ToolSchema {
            name: "click",
            schema: obj(el_props(serde_json::json!({})), serde_json::json!(["element_id", "name"])),
        },
        ToolSchema {
            name: "input",
            schema: obj(
                el_props(serde_json::json!({ "text": { "type": "string", "description": "要输入的文本" } })),
                serde_json::json!(["element_id", "name", "text"]),
            ),
        },
        ToolSchema {
            name: "long_press",
            schema: obj(
                el_props(serde_json::json!({ "duration_ms": { "type": "integer", "description": "长按毫秒数，默认 1000" } })),
                serde_json::json!(["element_id", "name"]),
            ),
        },
        ToolSchema {
            name: "clear",
            schema: obj(el_props(serde_json::json!({})), serde_json::json!(["element_id", "name"])),
        },
        ToolSchema {
            name: "click_visual",
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "region": {
                        "type": "array",
                        "items": { "type": "integer" },
                        "minItems": 4, "maxItems": 4,
                        "description": "目标在截图中的像素框 [x1,y1,x2,y2]（优先；越贴合目标越好）"
                    },
                    "x": { "type": "integer", "description": "无 region 时给点击点 x（像素）" },
                    "y": { "type": "integer", "description": "无 region 时给点击点 y（像素）" },
                    "name": { "type": "string", "description": "给该目标起的稳定语义名（落库并写进 .tks）" },
                    "desc": { "type": "string", "description": "可选、可留空——系统会在探索结束后据实际作用统一生成 desc" }
                },
                "required": ["name"]
            }),
        },
        ToolSchema {
            name: "swipe_direction",
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "direction": { "type": "string", "enum": ["up", "down", "left", "right"], "description": "滑动方向" },
                    "distance": { "type": "integer", "description": "可选：滑动像素距离" }
                },
                "required": ["direction"]
            }),
        },
        ToolSchema { name: "back", schema: empty_schema() },
        ToolSchema {
            name: "switch",
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "target": { "type": "string", "description": "web：要切到的标签页序号，或用新标签打开的 http(s) URL；移动端：要切到前台的 App 包名" }
                },
                "required": ["target"]
            }),
        },
        ToolSchema { name: "hide_keyboard", schema: empty_schema() },
        ToolSchema {
            name: "wait",
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "ms": { "type": "integer", "description": "固定等待毫秒数" },
                    "element": { "type": "string", "description": "等待出现的元素名（已落库的 {名}）" }
                }
            }),
        },
        ToolSchema {
            name: "request_screenshot",
            schema: serde_json::json!({
                "type": "object",
                "properties": { "reason": { "type": "string", "description": "为什么需要看截图" } },
                "required": ["reason"]
            }),
        },
        ToolSchema {
            name: "ask_user",
            schema: serde_json::json!({
                "type": "object",
                "properties": { "question": { "type": "string", "description": "向用户提出的问题" } },
                "required": ["question"]
            }),
        },
        ToolSchema {
            name: "finish",
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "success": { "type": "boolean", "description": "是否达成测试用例目标" },
                    "reason": { "type": "string", "description": "结束依据（达成/失败的判断理由）" }
                },
                "required": ["success", "reason"]
            }),
        },
    ]
}

fn empty_schema() -> serde_json::Value {
    serde_json::json!({ "type": "object", "properties": {} })
}
