// tks 序列化 - AST → 文本行，与 parser 互为逆操作。
// 语法单一来源：命令/方向的中英映射复用 constants（与解析器同一份），避免漂移。
// 生产者（AI harness / 未来录制）构造 TksStep 后调 step_to_source，保证产出可被 run 回放。

use crate::{LocatorStrategy, TksCommand, TksParam, TksStep};

use super::constants::{create_command_map, create_direction_map};

/// 一组步骤 → 完整 .tks 文本（含 `步骤:` 头）
pub fn script_to_source(steps: &[TksStep]) -> String {
    let mut s = String::from("步骤:\n");
    for st in steps {
        s.push_str(&step_to_source(st));
        s.push('\n');
    }
    s
}

/// 单个 TksStep → 一行 .tks 文本
/// 无参命令（返回/隐藏键盘）输出裸命令；有参输出 `命令 [p1, p2]`
pub fn step_to_source(step: &TksStep) -> String {
    let cmd = command_to_cn(&step.command);
    if step.params.is_empty() {
        cmd
    } else {
        let tokens: Vec<String> = step.params.iter().map(param_to_token).collect();
        format!("{} [{}]", cmd, tokens.join(", "))
    }
}

/// 单个参数 → token（parse_parameter 的逆）
fn param_to_token(p: &TksParam) -> String {
    match p {
        TksParam::Coordinate(pt) => format!("{{{}, {}}}", pt.x, pt.y),
        TksParam::Element { name, strategy } => match strategy_str(strategy) {
            Some(s) => format!("{{{}}}&{}", name, s),
            None => format!("{{{}}}", name),
        },
        TksParam::Text(t) => format!("\"{}\"", t),
        TksParam::Number(n) => n.to_string(),
        // Duration 存毫秒：始终带单位，人读无歧义——整秒 `Ns`，否则 `Nms`（均可逆回 Duration）
        TksParam::Duration(ms) => {
            if *ms % 1000 == 0 {
                format!("{}s", ms / 1000)
            } else {
                format!("{}ms", ms)
            }
        }
        TksParam::Direction(d) => direction_to_cn(d),
        TksParam::Boolean(b) => if *b { "存在".to_string() } else { "不存在".to_string() },
    }
}

/// 策略枚举 → `&策略` 的后缀名（与 LocatorStrategy::from_str 对应）；Auto 无后缀
fn strategy_str(s: &LocatorStrategy) -> Option<&'static str> {
    match s {
        LocatorStrategy::Auto => None,
        LocatorStrategy::XPath => Some("xpath"),
        LocatorStrategy::ResourceId => Some("resourceId"),
        LocatorStrategy::Text => Some("text"),
        LocatorStrategy::ContentDesc => Some("contentDesc"),
        LocatorStrategy::ClassName => Some("className"),
        LocatorStrategy::Ocr => Some("ocr"),
        LocatorStrategy::Img => Some("img"),
    }
}

/// TksCommand → 中文命令名（复用解析器命令表的逆查）
fn command_to_cn(cmd: &TksCommand) -> String {
    create_command_map()
        .into_iter()
        .find(|(_, c)| c == cmd)
        .map(|(k, _)| k)
        .unwrap_or_else(|| "?".to_string())
}

/// 英文方向 → 中文（复用解析器方向表的逆查）
fn direction_to_cn(d: &str) -> String {
    create_direction_map()
        .into_iter()
        .find(|(_, e)| e.as_str() == d)
        .map(|(cn, _)| cn)
        .unwrap_or_else(|| d.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ScriptParser;

    /// 往返稳定性：parse → serialize → parse → serialize，两次序列化结果一致
    #[test]
    fn round_trip_stable() {
        let samples = [
            "点击 [{登录按钮}]",
            "点击 [{930, 2294}]",
            "输入 [{用户名}, \"hello\"]",
            "按压 [{菜单}, 1000]",
            "定向滑动 [{540, 960}, 上, 600]",
            "等待 [2s]",
            "等待 [1500ms]",
            "断言 [{首页}, 存在]",
            "断言 [{提示}, 存在, 10s]",   // 重试断言:第三参数=最长等待
            "启动 [\"com.x.app\", \".MainActivity\"]",
            "返回",
            "隐藏键盘",
            "点击 [{登录}&xpath]",
        ];
        let parser = ScriptParser::new();
        for line in samples {
            let src = format!("步骤:\n{}\n", line);
            let script1 = parser.parse(&src).expect("parse1");
            let out1 = script_to_source(&script1.steps);
            let script2 = parser.parse(&out1).expect("parse2");
            let out2 = script_to_source(&script2.steps);
            assert_eq!(out1, out2, "往返不稳定: {}", line);
        }
    }
}
