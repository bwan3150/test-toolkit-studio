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
    // 元素工具只需 element_id（选哪个元素）；**不必起名**——系统按元素特征自动命名、
    // 落临时库，定稿时再统一起正式名。extra 注入各自附加字段（如 input 的 text）。
    let el_props = |extra: serde_json::Value| -> serde_json::Value {
        let mut base = serde_json::json!({
            "element_id": { "type": "integer", "description": "页面元素列表中的序号；选你要操作的那个元素" }
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
            schema: obj(el_props(serde_json::json!({})), serde_json::json!(["element_id"])),
        },
        ToolSchema {
            name: "input",
            schema: obj(
                el_props(serde_json::json!({ "text": { "type": "string", "description": "要输入的文本" } })),
                serde_json::json!(["element_id", "text"]),
            ),
        },
        ToolSchema {
            name: "long_press",
            schema: obj(
                el_props(serde_json::json!({ "duration_ms": { "type": "integer", "description": "长按毫秒数，默认 1000" } })),
                serde_json::json!(["element_id"]),
            ),
        },
        ToolSchema {
            name: "clear",
            schema: obj(el_props(serde_json::json!({})), serde_json::json!(["element_id"])),
        },
        ToolSchema {
            name: "assert",
            schema: obj(
                el_props(serde_json::json!({
                    "exist": { "type": "boolean", "description": "断言该元素存在(true，默认)还是不存在(false)" }
                })),
                serde_json::json!(["element_id"]),
            ),
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
                    "y": { "type": "integer", "description": "无 region 时给点击点 y（像素）" }
                },
                "required": []
            }),
        },
        ToolSchema {
            name: "swipe_direction",
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "direction": { "type": "string", "enum": ["up", "down", "left", "right"], "description": "手势方向。up=向上滑→看页面更下方内容(往下翻)；down=向下滑→看更上方内容(回顶部)。要往下找内容用 up！" },
                    "amount": { "type": "string", "enum": ["full", "half", "quarter"], "description": "滑动幅度=屏幕比例（推荐，免算像素）：full≈整屏、half≈半屏、quarter≈四分之一屏。默认 half。" },
                    "distance": { "type": "integer", "description": "可选：直接给滑动像素距离（一般用 amount 即可，不用这个）" }
                },
                "required": ["direction"]
            }),
        },
        ToolSchema {
            name: "swipe_to_find",
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "target": { "type": "string", "description": "要滚动找到的目标文字（页面上会出现的可见文字，如产品名/型号/标题，如 'T01 Pro' 或 'DATA SHEET'）" },
                    "direction": { "type": "string", "enum": ["up", "down", "left", "right"], "description": "滚动方向。目标在更下方就用 up（向上滑=看下方内容）。" }
                },
                "required": ["target", "direction"]
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
                    "duration": { "type": "string", "description": "固定等待时长，**必须带单位**：如 5s（5秒）、1000ms（1000毫秒，即1秒）" },
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
            name: "rename_element",
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "old_name": { "type": "string", "description": "当前已知元素的（错误）名字" },
                    "new_name": { "type": "string", "description": "改成的正确名字（名词、指代该元素本身）" }
                },
                "required": ["old_name", "new_name"]
            }),
        },
        ToolSchema {
            name: "finish",
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "success": { "type": "boolean", "description": "是否达成测试用例目标" },
                    "reason": { "type": "string", "description": "结束依据（达成/失败的判断理由）" },
                    "script_name": { "type": "string", "description": "给本次生成的回放脚本起一个简短的文件名（不含扩展名）：小写英文或拼音、用连字符分隔、概括本次测试做了什么，如 login-flow、product-pdf-view、cart-checkout。" }
                },
                "required": ["success", "reason"]
            }),
        },
    ]
}

fn empty_schema() -> serde_json::Value {
    serde_json::json!({ "type": "object", "properties": {} })
}
