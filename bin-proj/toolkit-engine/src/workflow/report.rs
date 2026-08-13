// 【检查报告】把一次运行的 log.json + 截图序列渲染成一个**自包含的 report.html**。
//
// 存在理由：证据目录里躺着 log.json 和一堆 step_00N.png，人要看懂得自己一个个点开、
// 再对着 json 找哪步是哪张图。报告把它们缝在一起——**双击就能看完全程**。
//
// 自包含策略（有意的取舍）：
//   - 截图 **base64 内嵌** → 单个 html 发给同事/贴进工单也能看图，这是人最需要的
//   - 页面结构 xml **不内嵌**，只留相对链接 → xml 动辄几百 KB 且只有 AI/排障才看，
//     全塞进去会让报告大到打不开。留在原目录里，报告在原地打开时链接照常可用
//
// 无外部依赖：CSS 全内联，不引 CDN——离线、内网、断网的 CI 里打开都一样。

use std::path::{Path, PathBuf};

use base64::Engine;

use crate::{ExecutionResult, StepResult};
use crate::{Result, TkeError};

/// 在 `run_dir` 里生成 `report.html`，返回它的路径。
pub fn write_report(run_dir: &Path, result: &ExecutionResult) -> Result<PathBuf> {
    let html = render(run_dir, result);
    let out = run_dir.join("report.html");
    std::fs::write(&out, html).map_err(TkeError::IoError)?;
    Ok(out)
}

/// 图片 → data URI。读不到就返回 None（报告照出，只是少一张图——
/// 证据不全不该让整份报告生不出来）。
fn img_data_uri(run_dir: &Path, rel: &str) -> Option<String> {
    let bytes = std::fs::read(run_dir.join(rel)).ok()?;
    Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// "2026-08-13T15:47:43+10:00" → "15:47:43"（只给人看时分秒，日期在头部已有）
fn hhmmss(rfc3339: &str) -> String {
    rfc3339
        .split('T')
        .nth(1)
        .map(|t| t.split(['+', '.', 'Z']).next().unwrap_or(t).to_string())
        .unwrap_or_else(|| rfc3339.to_string())
}

fn dur_between(start: &str, end: &str) -> String {
    let (Ok(a), Ok(b)) = (
        chrono::DateTime::parse_from_rfc3339(start),
        chrono::DateTime::parse_from_rfc3339(end),
    ) else {
        return "—".into();
    };
    let ms = (b - a).num_milliseconds().max(0);
    if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{}m {}s", ms / 60_000, (ms % 60_000) / 1000)
    }
}

fn render(run_dir: &Path, r: &ExecutionResult) -> String {
    let passed = r.steps.iter().filter(|s| s.success).count();
    let total = r.steps.len();
    let healed = r.steps.iter().filter(|s| s.healed.is_some()).count();

    let steps_html: String = r
        .steps
        .iter()
        .map(|s| step_card(run_dir, s))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} · 检查报告</title>
<style>
*,*::before,*::after{{box-sizing:border-box;margin:0;padding:0}}
:root{{
  --bg:#f6f6f7; --card:#fff; --border:#e5e5e7; --txt:#1c1c1e; --txt2:#48484a; --txt3:#8e8e93;
  --ok:#1a7f37; --ok-bg:#f0fdf4; --ng:#b62324; --ng-bg:#fff0f0; --warn:#bf8600; --warn-bg:#fffbeb;
  --mono:'SF Mono','Fira Code','Cascadia Code',Consolas,monospace;
}}
@media (prefers-color-scheme:dark){{
  :root{{--bg:#1a1a1c;--card:#242426;--border:#38383a;--txt:#f2f2f7;--txt2:#c7c7cc;--txt3:#8e8e93;
        --ok-bg:#132b18;--ng-bg:#2d1416;--warn-bg:#2b2410}}
}}
body{{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI','PingFang SC','Hiragino Sans GB',
  'Microsoft YaHei',system-ui,sans-serif;background:var(--bg);color:var(--txt);
  font-size:14px;line-height:1.6;padding:24px 16px 64px}}
.wrap{{max-width:860px;margin:0 auto}}
header{{background:var(--card);border:1px solid var(--border);border-radius:12px;
  padding:18px 20px;margin-bottom:20px}}
.h-top{{display:flex;align-items:center;gap:12px;flex-wrap:wrap}}
h1{{font-size:17px;font-weight:650;flex:1;min-width:0;word-break:break-all}}
.badge{{padding:4px 14px;border-radius:99px;font-size:12px;font-weight:700;flex-shrink:0}}
.b-ok{{background:var(--ok);color:#fff}} .b-ng{{background:var(--ng);color:#fff}}
.stats{{display:flex;gap:20px;flex-wrap:wrap;margin-top:12px;padding-top:12px;
  border-top:1px solid var(--border);font-size:12px;color:var(--txt3)}}
.stats b{{color:var(--txt2);font-weight:600;font-family:var(--mono)}}
.err-top{{margin-top:12px;padding:10px 12px;background:var(--ng-bg);border-radius:8px;
  font-size:12px;color:var(--ng);font-family:var(--mono);white-space:pre-wrap;word-break:break-all}}
.step{{background:var(--card);border:1px solid var(--border);border-radius:10px;
  overflow:hidden;margin-bottom:14px}}
.step.ng{{border-color:var(--ng)}}
.s-hd{{display:flex;align-items:center;gap:10px;padding:10px 14px;border-bottom:1px solid var(--border)}}
.s-num{{font-family:var(--mono);font-size:11px;color:var(--txt3);flex-shrink:0}}
.s-mark{{width:18px;height:18px;border-radius:50%;display:flex;align-items:center;
  justify-content:center;font-size:11px;font-weight:700;flex-shrink:0;color:#fff}}
.m-ok{{background:var(--ok)}} .m-ng{{background:var(--ng)}}
.s-cmd{{flex:1;min-width:0;font-family:var(--mono);font-size:12.5px;word-break:break-all}}
.s-dur{{font-family:var(--mono);font-size:11px;color:var(--txt3);flex-shrink:0}}
.s-err{{padding:10px 14px;background:var(--ng-bg);color:var(--ng);font-family:var(--mono);
  font-size:11.5px;white-space:pre-wrap;word-break:break-all;border-bottom:1px solid var(--border)}}
.s-heal{{padding:7px 14px;background:var(--warn-bg);color:var(--warn);font-size:11.5px;
  border-bottom:1px solid var(--border)}}
.s-img{{padding:14px;background:#141414;display:flex;justify-content:center}}
.s-img img{{max-width:100%;height:auto;border-radius:6px;display:block}}
.s-foot{{padding:6px 14px;font-size:11px;color:var(--txt3)}}
.s-foot a{{color:inherit}}
footer{{margin-top:28px;font-size:11px;color:var(--txt3);text-align:center;line-height:1.8}}
</style>
</head>
<body>
<div class="wrap">
<header>
  <div class="h-top">
    <h1>{title}</h1>
    <span class="badge {badge_cls}">{badge_txt}</span>
  </div>
  <div class="stats">
    <span><b>{passed}/{total}</b> 步通过</span>
    <span>耗时 <b>{dur}</b></span>
    <span>开始 <b>{start}</b></span>
    {healed_stat}
  </div>
  {top_err}
</header>
{steps}
<footer>
  tke 检查报告 · 截图已内嵌，此文件可单独发送<br>
  页面结构 xml 未内嵌（体积原因），需在原目录 <code>{dir}</code> 下打开才能查看
</footer>
</div>
</body>
</html>"#,
        title = esc(&r.script_name),
        badge_cls = if r.success { "b-ok" } else { "b-ng" },
        badge_txt = if r.success { "通过" } else { "失败" },
        passed = passed,
        total = total,
        dur = dur_between(&r.start_time, &r.end_time),
        start = hhmmss(&r.start_time),
        healed_stat = if healed > 0 {
            format!(r#"<span style="color:var(--warn)"><b>{}</b> 步由 AI 找回定位</span>"#, healed)
        } else {
            String::new()
        },
        top_err = match &r.error {
            Some(e) if !e.is_empty() => format!(r#"<div class="err-top">{}</div>"#, esc(e)),
            _ => String::new(),
        },
        steps = steps_html,
        dir = esc(&run_dir.display().to_string()),
    )
}

fn step_card(run_dir: &Path, s: &StepResult) -> String {
    let img = s
        .screenshot
        .as_deref()
        .and_then(|rel| img_data_uri(run_dir, rel))
        .map(|uri| format!(r#"<div class="s-img"><img src="{}" alt="step"></div>"#, uri))
        .unwrap_or_default();

    format!(
        r#"<div class="step {ng}">
  <div class="s-hd">
    <span class="s-num">{num:02}</span>
    <span class="s-mark {mcls}">{mark}</span>
    <span class="s-cmd">{cmd}</span>
    <span class="s-dur">{dur}ms</span>
  </div>
  {heal}
  {err}
  {img}
  {foot}
</div>"#,
        ng = if s.success { "" } else { "ng" },
        num = s.index + 1,
        mcls = if s.success { "m-ok" } else { "m-ng" },
        mark = if s.success { "✓" } else { "✗" },
        cmd = esc(&s.command),
        dur = s.duration_ms,
        heal = match &s.healed {
            Some(name) => format!(
                r#"<div class="s-heal">⚠ 元素「{}」按原定位没找到，由 AI 依当前页面找回（脚本定位可能该更新了）</div>"#,
                esc(name)
            ),
            None => String::new(),
        },
        err = match &s.error {
            Some(e) if !e.is_empty() => format!(r#"<div class="s-err">{}</div>"#, esc(e)),
            _ => String::new(),
        },
        img = img,
        foot = match &s.xml {
            Some(x) => format!(r#"<div class="s-foot">页面结构：<a href="{0}">{0}</a></div>"#, esc(x)),
            None => String::new(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_step(index: usize, success: bool) -> StepResult {
        StepResult {
            index,
            command: "点击 [{640, 380}]".into(),
            success,
            error: if success { None } else { Some("元素未找到 <script>".into()) },
            duration_ms: 123,
            line: Some(index + 2),
            screenshot: None,
            xml: Some(format!("page/step_{:03}.xml", index + 1)),
            healed: None,
        }
    }

    fn mk_result(success: bool) -> ExecutionResult {
        ExecutionResult {
            success,
            case_id: "c1".into(),
            script_name: "登录检查 & <验证>".into(),
            start_time: "2026-08-13T15:47:43+10:00".into(),
            end_time: "2026-08-13T15:47:53+10:00".into(),
            steps: vec![mk_step(0, true), mk_step(1, success)],
            error: if success { None } else { Some("第 2 步失败".into()) },
            script_path: None,
            run_dir: None,
            launched_packages: vec![],
        }
    }

    /// 报告能生成，关键信息（标题/结论/命令/耗时）都在
    #[test]
    fn report_contains_summary_and_steps() {
        let dir = std::env::temp_dir().join(format!("tke-report-ok-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = write_report(&dir, &mk_result(true)).unwrap();
        let html = std::fs::read_to_string(&out).unwrap();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("通过"), "应有结论徽标");
        assert!(html.contains("2/2"), "应有通过步数统计");
        assert!(html.contains("10.0s"), "应算出耗时:{}", html);
        assert!(html.contains("15:47:43"), "应有开始时刻");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 失败信息必须出现在报告里(INV-9:失败要可见,不能只留个红叉)
    #[test]
    fn report_surfaces_failure() {
        let dir = std::env::temp_dir().join(format!("tke-report-ng-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = write_report(&dir, &mk_result(false)).unwrap();
        let html = std::fs::read_to_string(&out).unwrap();

        assert!(html.contains("失败"));
        assert!(html.contains("第 2 步失败"), "顶部应有整体错误");
        assert!(html.contains("元素未找到"), "步内应有具体报错");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 脚本名/报错里的 HTML 必须转义——否则一个带 < 的报错就能把报告打歪
    #[test]
    fn report_escapes_html() {
        let dir = std::env::temp_dir().join(format!("tke-report-esc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = write_report(&dir, &mk_result(false)).unwrap();
        let html = std::fs::read_to_string(&out).unwrap();

        assert!(html.contains("&lt;script&gt;"), "报错里的标签应被转义");
        assert!(!html.contains("<script>元素"), "不该出现未转义的注入");
        assert!(html.contains("&amp; &lt;验证&gt;"), "标题应被转义:{}", html);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 截图读不到时报告照样生成（证据不全不该让报告生不出来）
    #[test]
    fn report_survives_missing_screenshot() {
        let dir = std::env::temp_dir().join(format!("tke-report-noimg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut res = mk_result(true);
        res.steps[0].screenshot = Some("screenshots/step_001.png".into()); // 文件并不存在
        let out = write_report(&dir, &res).unwrap();
        let html = std::fs::read_to_string(&out).unwrap();
        assert!(!html.contains("data:image/png"), "没有图就不该有 data uri");
        assert!(html.contains("点击"), "但步骤本身要在");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
