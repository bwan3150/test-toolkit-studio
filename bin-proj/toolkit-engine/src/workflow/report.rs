// 【检查报告】把一次运行的 log.json + 截图序列 + 页面结构渲染成一个**自包含的 report.html**。
//
// 存在理由：证据目录里躺着 log.json 和一堆 step_00N.png，人要看懂得一个个点开、再对着
// json 找哪步是哪张图。报告把它们缝在一起——**双击就能看完全程**。
//
// 「点了什么」是这份报告的核心增量：脚本里写的是 `点击 [{299, 242}]`，光看这行没人知道
// 点的是啥。报告从**执行时的页面结构**反查该坐标命中的元素，把 text / class / resource-id /
// xpath 摆出来（点击展开）——这正是复核时最想知道的那件事。
//
// ⚠️ 反查要用**上一步**的 xml：每步存的是**动作执行后**的页面（点完就跳走了），
//    所以第 N 步点中的东西，得在第 N-1 步的页面里找。第一步没有前置页面，就不显示。
//
// 自包含策略（有意的取舍）：
//   - 截图 **base64 内嵌** → 单个 html 发给同事/贴进工单也能看图，这是人最需要的
//   - 页面结构 xml **不内嵌**，只在顶部给文件链接 → xml 动辄几百 KB 且只有排障才逐行看
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

// ── 页面结构反查 ─────────────────────────────────────────────────────────

/// 从页面结构里查到的一个元素（报告展开区显示）
#[derive(Debug, Default, Clone)]
struct HitElement {
    class: String,
    text: String,
    desc: String,
    resource_id: String,
    xpath: String,
    bounds: (i64, i64, i64, i64),
    clickable: bool,
}

impl HitElement {
    /// 元素面积——命中多个时取最小的那个（最内层 = 真正被点到的）
    fn area(&self) -> i64 {
        (self.bounds.2 - self.bounds.0).max(0) * (self.bounds.3 - self.bounds.1).max(0)
    }
    /// 一句话标签：优先文字，其次无障碍描述、id，最后退回标签名
    fn label(&self) -> String {
        for s in [&self.text, &self.desc, &self.resource_id] {
            let t = s.trim();
            if !t.is_empty() {
                return t.chars().take(60).collect();
            }
        }
        format!("<{}>", self.class)
    }
}

/// 取 xml 标签里的某个属性值（结构文件是 tke 归一化过的扁平 node 列表，
/// 用不着引 xml 解析器——多一个依赖换不来什么）
fn attr(tag: &str, key: &str) -> String {
    let pat = format!("{}=\"", key);
    let Some(start) = tag.find(&pat) else { return String::new() };
    let rest = &tag[start + pat.len()..];
    let end = rest.find('"').unwrap_or(0);
    rest[..end]
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// `[83,80][317,102]` → (83, 80, 317, 102)
fn parse_bounds(s: &str) -> Option<(i64, i64, i64, i64)> {
    let nums: Vec<i64> = s
        .split(|c: char| !c.is_ascii_digit() && c != '-')
        .filter(|t| !t.is_empty())
        .filter_map(|t| t.parse().ok())
        .collect();
    (nums.len() >= 4).then(|| (nums[0], nums[1], nums[2], nums[3]))
}

/// 在页面结构里找**包含该坐标的最小元素**（最内层的那个才是真正被点到的）
fn hit_test(xml: &str, x: i64, y: i64) -> Option<HitElement> {
    let mut best: Option<HitElement> = None;
    for tag in xml.split('<').filter(|t| t.starts_with("node ")) {
        let Some(b) = parse_bounds(&attr(tag, "bounds")) else { continue };
        if x < b.0 || x > b.2 || y < b.1 || y > b.3 {
            continue;
        }
        let el = HitElement {
            class: attr(tag, "class"),
            text: attr(tag, "text"),
            desc: attr(tag, "content-desc"),
            resource_id: attr(tag, "resource-id"),
            xpath: attr(tag, "xpath"),
            bounds: b,
            clickable: attr(tag, "clickable") == "true",
        };
        if best.as_ref().is_none_or(|cur| el.area() < cur.area()) {
            best = Some(el);
        }
    }
    best
}

/// 从命令里取第一个坐标：`点击 [{299, 242}]` → (299, 242)。
/// 花括号里不是纯数字（`点击 [{登录按钮}]`）说明走的是元素库，不是坐标。
fn coord_in(command: &str) -> Option<(i64, i64)> {
    let inner = command.split('{').nth(1)?.split('}').next()?;
    let mut it = inner.split(',');
    let x = it.next()?.trim().parse().ok()?;
    let y = it.next()?.trim().parse().ok()?;
    Some((x, y))
}

/// 从命令里取元素名：`点击 [{登录按钮}]` → "登录按钮"（坐标形式返回 None）
fn element_in(command: &str) -> Option<String> {
    let inner = command.split('{').nth(1)?.split('}').next()?.trim();
    if inner.is_empty() || coord_in(command).is_some() {
        return None;
    }
    Some(inner.to_string())
}

/// 第 i 步动作发生时的页面 = 往前找到的第一份页面结构。
/// **不能用第 i 步自己的 xml**：那是动作执行完之后的页面（点完早跳走了）。
fn page_before(run_dir: &Path, steps: &[StepResult], i: usize) -> Option<String> {
    steps[..i]
        .iter()
        .rev()
        .find_map(|s| s.xml.as_deref())
        .and_then(|rel| std::fs::read_to_string(run_dir.join(rel)).ok())
}

// ── 渲染 ─────────────────────────────────────────────────────────────────

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

fn hhmmss(rfc3339: &str) -> String {
    rfc3339
        .split('T')
        .nth(1)
        .map(|t| t.split(['+', '.', 'Z']).next().unwrap_or(t).to_string())
        .unwrap_or_else(|| rfc3339.to_string())
}

fn ms_between(start: &str, end: &str) -> Option<i64> {
    let (Ok(a), Ok(b)) = (
        chrono::DateTime::parse_from_rfc3339(start),
        chrono::DateTime::parse_from_rfc3339(end),
    ) else {
        return None;
    };
    Some((b - a).num_milliseconds().max(0))
}

fn fmt_dur(ms: i64) -> String {
    if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{}m {}s", ms / 60_000, (ms % 60_000) / 1000)
    }
}

/// 平台标签：给元素信息加个来源前缀（web: / android: / ios:），
/// 让人一眼看出这份结构是从哪种页面里读出来的
fn platform_tag(device: Option<&str>) -> &'static str {
    match device {
        Some("web") => "web",
        Some(d) if d.len() >= 25 && d.contains('-') => "ios", // UDID 形态
        Some(_) => "android",
        None => "device",
    }
}

fn render(run_dir: &Path, r: &ExecutionResult) -> String {
    let passed = r.steps.iter().filter(|s| s.success).count();
    let failed = r.steps.len() - passed;
    let healed = r.steps.iter().filter(|s| s.healed.is_some()).count();
    let plat = platform_tag(r.device.as_deref());

    let steps_html: String = r
        .steps
        .iter()
        .enumerate()
        .map(|(i, s)| step_card(run_dir, r, i, s, plat))
        .collect::<Vec<_>>()
        .join("\n");

    // 相关文件：报告在原目录打开时这些链接可直接点开
    let dir_str = run_dir.display().to_string();
    // 一排按钮而不是一行链接：文案说清"点了会看到什么"，比裸文件名好认
    const IC_LOG: &str = r#"<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M3 1.5h7l3 3v10H3z"/><path d="M5.5 7h5M5.5 10h5"/></svg>"#;
    const IC_IMG: &str = r#"<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="1.5" y="3" width="13" height="10" rx="1.5"/><circle cx="5.5" cy="6.5" r="1"/><path d="M2 11l3.5-3 3 2.5L11 8l3 3"/></svg>"#;
    const IC_XML: &str = r#"<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M5.5 4L2 8l3.5 4M10.5 4L14 8l-3.5 4"/></svg>"#;
    const IC_DIR: &str = r#"<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M1.5 12.5v-9h4l1.5 2h7.5v7z"/></svg>"#;
    let file_links = format!(
        r#"<a class="fbtn" href="log.json">{IC_LOG}查看原始日志</a>
    <a class="fbtn" href="screenshots/">{IC_IMG}查看截图序列</a>
    <a class="fbtn" href="page/">{IC_XML}查看页面 XML</a>
    <a class="fbtn" href="{dir}">{IC_DIR}打开执行目录</a>"#,
        dir = esc(&format!("file://{}", dir_str)),
    );

    let meta_rows = [
        ("设备", r.device.clone().unwrap_or_else(|| "—".into())),
        ("脚本", r.script_path.clone().unwrap_or_else(|| "—".into())),
        ("开始", hhmmss(&r.start_time)),
        ("结束", hhmmss(&r.end_time)),
        ("目录", dir_str.clone()),
    ]
    .iter()
    .map(|(k, v)| {
        format!(
            r#"<div class="m-row"><span class="m-k">{}</span><span class="m-v">{}</span></div>"#,
            k,
            esc(v)
        )
    })
    .collect::<Vec<_>>()
    .join("");

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
  --acc:#5856d6; --acc-bg:#f0f0ff;
  --mono:'SF Mono','Fira Code','Cascadia Code',Consolas,monospace;
}}
@media (prefers-color-scheme:dark){{
  :root{{--bg:#1a1a1c;--card:#242426;--border:#38383a;--txt:#f2f2f7;--txt2:#c7c7cc;--txt3:#8e8e93;
        --ok-bg:#132b18;--ng-bg:#2d1416;--warn-bg:#2b2410;--acc-bg:#1e1e35}}
}}
body{{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI','PingFang SC','Hiragino Sans GB',
  'Microsoft YaHei',system-ui,sans-serif;background:var(--bg);color:var(--txt);
  font-size:14px;line-height:1.6;padding:24px 16px 64px}}
.wrap{{max-width:880px;margin:0 auto}}
header{{background:var(--card);border:1px solid var(--border);border-radius:12px;
  padding:18px 20px;margin-bottom:18px}}
.h-top{{display:flex;align-items:center;gap:12px;flex-wrap:wrap}}
h1{{font-size:17px;font-weight:650;flex:1;min-width:0;word-break:break-all}}
.badge{{padding:4px 14px;border-radius:99px;font-size:12px;font-weight:700;flex-shrink:0}}
.b-ok{{background:var(--ok);color:#fff}} .b-ng{{background:var(--ng);color:#fff}}
.chips{{display:flex;gap:8px;flex-wrap:wrap;margin-top:12px}}
.chip{{padding:3px 10px;border-radius:6px;font-size:12px;font-family:var(--mono);
  background:var(--bg);border:1px solid var(--border);color:var(--txt2)}}
.chip b{{color:var(--txt);font-weight:700}}
.c-ok b{{color:var(--ok)}} .c-ng b{{color:var(--ng)}} .c-warn b{{color:var(--warn)}}
.meta{{margin-top:12px;padding-top:12px;border-top:1px solid var(--border);
  display:grid;grid-template-columns:auto 1fr;gap:2px 14px;font-size:12px}}
.m-row{{display:contents}}
.m-k{{color:var(--txt3);white-space:nowrap}}
.m-v{{color:var(--txt2);font-family:var(--mono);word-break:break-all;font-size:11.5px}}
.files{{margin-top:14px;display:flex;gap:8px;flex-wrap:wrap}}
.fbtn{{display:inline-flex;align-items:center;gap:6px;padding:6px 12px;border-radius:7px;
  border:1px solid var(--border);background:var(--bg);color:var(--txt2);
  font-size:12px;text-decoration:none;white-space:nowrap;transition:all .12s}}
.fbtn:hover{{border-color:var(--acc);color:var(--acc);background:var(--acc-bg)}}
.fbtn svg{{width:13px;height:13px;flex-shrink:0;opacity:.75}}
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
.tgt{{padding:8px 14px;background:var(--acc-bg);border-bottom:1px solid var(--border);font-size:12px}}
.tgt summary{{cursor:pointer;list-style:none;display:flex;align-items:center;gap:8px}}
.tgt summary::-webkit-details-marker{{display:none}}
.tgt summary::before{{content:'▸';color:var(--acc);font-size:10px;flex-shrink:0}}
.tgt[open] summary::before{{content:'▾'}}
.t-src{{font-family:var(--mono);font-size:10px;font-weight:700;color:#fff;background:var(--acc);
  padding:1px 6px;border-radius:3px;flex-shrink:0}}
.t-lbl{{font-weight:600;color:var(--txt);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}}
.t-note{{color:var(--txt3);font-size:11px;flex-shrink:0}}
.t-body{{margin-top:8px;display:grid;grid-template-columns:auto 1fr;gap:2px 12px;
  font-family:var(--mono);font-size:11px}}
.t-k{{color:var(--txt3)}} .t-v{{color:var(--txt2);word-break:break-all}}
.say{{padding:9px 14px;border-bottom:1px solid var(--border);font-size:13px;color:var(--txt);
  display:flex;gap:8px;align-items:baseline}}
.say::before{{content:'“';color:var(--acc);font-size:20px;line-height:0.6;flex-shrink:0}}
.s-err{{padding:10px 14px;background:var(--ng-bg);color:var(--ng);font-family:var(--mono);
  font-size:11.5px;white-space:pre-wrap;word-break:break-all;border-bottom:1px solid var(--border)}}
.s-heal{{padding:7px 14px;background:var(--warn-bg);color:var(--warn);font-size:11.5px;
  border-bottom:1px solid var(--border)}}
.s-img{{padding:14px;background:#141414;display:flex;justify-content:center}}
.s-img img{{max-width:100%;height:auto;border-radius:6px;display:block}}
.s-foot{{padding:6px 14px;font-size:11px;color:var(--txt3);font-family:var(--mono)}}
.s-foot a{{color:var(--acc);text-decoration:none}}
</style>
</head>
<body>
<div class="wrap">
<header>
  <div class="h-top">
    <h1>{title}</h1>
    <span class="badge {badge_cls}">{badge_txt}</span>
  </div>
  <div class="chips">
    <span class="chip c-ok"><b>{passed}</b> 步通过</span>
    {failed_chip}
    {healed_chip}
    <span class="chip">共 <b>{total}</b> 步</span>
    <span class="chip">耗时 <b>{dur}</b></span>
  </div>
  <div class="meta">{meta_rows}</div>
  <div class="files">{file_links}</div>
  {top_err}
</header>
{steps}
</div>
</body>
</html>"#,
        title = esc(&r.script_name),
        badge_cls = if r.success { "b-ok" } else { "b-ng" },
        badge_txt = if r.success { "通过" } else { "失败" },
        passed = passed,
        total = r.steps.len(),
        failed_chip = if failed > 0 {
            format!(r#"<span class="chip c-ng"><b>{}</b> 步失败</span>"#, failed)
        } else {
            String::new()
        },
        healed_chip = if healed > 0 {
            format!(r#"<span class="chip c-warn"><b>{}</b> 步 AI 找回定位</span>"#, healed)
        } else {
            String::new()
        },
        dur = ms_between(&r.start_time, &r.end_time).map(fmt_dur).unwrap_or_else(|| "—".into()),
        meta_rows = meta_rows,
        file_links = file_links,
        top_err = match &r.error {
            Some(e) if !e.is_empty() => format!(r#"<div class="err-top">{}</div>"#, esc(e)),
            _ => String::new(),
        },
        steps = steps_html,
    )
}

/// 「点了什么」区块：坐标 → 从执行时的页面反查命中元素；元素名 → 直接标出走的是元素库
fn target_block(
    run_dir: &Path,
    r: &ExecutionResult,
    i: usize,
    s: &StepResult,
    plat: &str,
) -> String {
    // 走元素库的（harness 产出的脚本）：定位依据在 .tklib 里，报告只标明来源
    if let Some(name) = element_in(&s.command) {
        return format!(
            r#"<details class="tgt"><summary>
      <span class="t-src">元素库</span>
      <span class="t-lbl">{}</span>
      <span class="t-note">定位依据在同名 .tklib（结构 / OCR / 图像三通道）</span>
    </summary></details>"#,
            esc(&name)
        );
    }

    // AI 找回的：定位不是脚本里那条，而是 AI 看着当前页面重新认的
    if let Some(name) = &s.healed {
        return format!(
            r#"<details class="tgt"><summary>
      <span class="t-src" style="background:var(--warn)">AI 识别</span>
      <span class="t-lbl">{}</span>
      <span class="t-note">原定位失效，由 AI 依当前页面找回</span>
    </summary></details>"#,
            esc(name)
        );
    }

    let Some((x, y)) = coord_in(&s.command) else { return String::new() };

    // 坐标点击（skill 的主用法）：从**执行时**的页面反查这个点落在什么元素上
    let Some(xml) = page_before(run_dir, &r.steps, i) else {
        return format!(
            r#"<div class="tgt"><summary><span class="t-src">坐标</span>
      <span class="t-note">({}, {}) · 无前置页面结构，查不到点中的元素</span></summary></div>"#,
            x, y
        );
    };

    let Some(el) = hit_test(&xml, x, y) else {
        return format!(
            r#"<details class="tgt"><summary>
      <span class="t-src" style="background:var(--ng)">坐标</span>
      <span class="t-lbl">({}, {}) 没有命中任何元素</span>
      <span class="t-note">点了个空处——这一步多半没起作用</span>
    </summary></details>"#,
            x, y
        );
    };

    let rows = [
        ("类型", el.class.clone()),
        ("文本", el.text.clone()),
        ("描述", el.desc.clone()),
        ("id", el.resource_id.clone()),
        ("xpath", el.xpath.clone()),
        (
            "范围",
            format!("[{},{}][{},{}]", el.bounds.0, el.bounds.1, el.bounds.2, el.bounds.3),
        ),
        ("可点击", if el.clickable { "是".into() } else { "否".into() }),
    ]
    .iter()
    .filter(|(_, v)| !v.trim().is_empty())
    .map(|(k, v)| format!(r#"<span class="t-k">{}</span><span class="t-v">{}</span>"#, k, esc(v)))
    .collect::<Vec<_>>()
    .join("");

    format!(
        r#"<details class="tgt"><summary>
      <span class="t-src">{plat}</span>
      <span class="t-lbl">{label}</span>
      <span class="t-note">({x}, {y}){warn}</span>
    </summary>
    <div class="t-body">{rows}</div>
  </details>"#,
        plat = plat,
        label = esc(&el.label()),
        x = x,
        y = y,
        warn = if el.clickable { "" } else { " · 该元素不可点击" },
        rows = rows,
    )
}

fn step_card(
    run_dir: &Path,
    r: &ExecutionResult,
    i: usize,
    s: &StepResult,
    plat: &str,
) -> String {
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
  {say}
  {tgt}
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
        // 写指令的人/AI 留下的「这一步在干什么」。没写就不占位置——
        // 与其填一句机器凑的套话,不如什么都不说(套话会让人以为读懂了,其实没有)
        say = match &s.note {
            Some(n) if !n.trim().is_empty() => format!(r#"<div class="say">{}</div>"#, esc(n)),
            _ => String::new(),
        },
        tgt = target_block(run_dir, r, i, s, plat),
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
        foot = match (&s.xml, &s.screenshot) {
            (Some(x), Some(p)) =>
                format!(r#"<div class="s-foot"><a href="{0}">{0}</a> · <a href="{1}">{1}</a></div>"#, esc(x), esc(p)),
            (Some(x), None) => format!(r#"<div class="s-foot"><a href="{0}">{0}</a></div>"#, esc(x)),
            (None, Some(p)) => format!(r#"<div class="s-foot"><a href="{0}">{0}</a></div>"#, esc(p)),
            _ => String::new(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = r#"<?xml version='1.0' encoding='UTF-8'?>
<hierarchy rotation="0">
  <node class="body" resource-id="" content-desc="" text="" xpath="/body" clickable="false" bounds="[0,0][1280,800]" />
  <node class="a" resource-id="more" content-desc="" text="Learn more" xpath="/body/p[2]/a[1]" clickable="true" bounds="[256,231][343,253]" />
  <node class="h1" resource-id="" content-desc="" text="Example &amp; &lt;Domain&gt;" xpath="/body/h1" clickable="false" bounds="[256,122][1024,155]" />
</hierarchy>"#;

    fn mk_result(steps: Vec<StepResult>) -> ExecutionResult {
        ExecutionResult {
            success: steps.iter().all(|s| s.success),
            case_id: "c1".into(),
            script_name: "检查 & <验证>".into(),
            start_time: "2026-08-13T15:47:43+10:00".into(),
            end_time: "2026-08-13T15:47:53+10:00".into(),
            steps,
            error: None,
            script_path: Some("/tmp/x.tks".into()),
            run_dir: None,
            launched_packages: vec![],
            device: Some("web".into()),
        }
    }

    fn step(index: usize, command: &str, xml: Option<&str>) -> StepResult {
        StepResult {
            index,
            command: command.into(),
            success: true,
            error: None,
            duration_ms: 100,
            line: Some(index + 2),
            screenshot: None,
            xml: xml.map(String::from),
            healed: None,
            note: None,
        }
    }

    fn setup(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tke-report-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("page")).unwrap();
        std::fs::write(d.join("page/step_001.xml"), PAGE).unwrap();
        d
    }

    /// 坐标命中：报告要说清点的是哪个元素（这是这份报告存在的主要理由）
    #[test]
    fn resolves_clicked_element_from_previous_page() {
        let dir = setup("hit");
        let r = mk_result(vec![
            step(0, r#"启动 ["https://example.com"]"#, Some("page/step_001.xml")),
            step(1, "点击 [{299, 242}]", Some("page/step_002.xml")),
        ]);
        let html = std::fs::read_to_string(write_report(&dir, &r).unwrap()).unwrap();

        assert!(html.contains("Learn more"), "应反查出点中的元素:{}", html);
        assert!(html.contains("/body/p[2]/a[1]"), "应给出 xpath");
        assert!(html.contains(r#"<span class="t-src">web</span>"#), "应标出来源平台");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 点空：必须明说"没命中任何元素"——这一步多半没起作用，是假成功的高发地
    #[test]
    fn flags_click_that_hits_nothing() {
        let dir = setup("miss");
        let r = mk_result(vec![
            step(0, r#"启动 ["https://example.com"]"#, Some("page/step_001.xml")),
            step(1, "点击 [{99999, 99999}]", None),
        ]);
        let html = std::fs::read_to_string(write_report(&dir, &r).unwrap()).unwrap();
        assert!(html.contains("没有命中任何元素"), "点空必须说出来:{}", html);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 反查必须用**上一步**的页面：用自己那步的（执行后已跳走）会标错元素
    #[test]
    fn uses_page_before_the_action() {
        let dir = setup("before");
        // step_002.xml 是点击后的新页面，里面没有 Learn more
        std::fs::write(
            dir.join("page/step_002.xml"),
            r#"<hierarchy><node class="h1" text="跳转后的新页面" bounds="[0,0][1280,800]" xpath="/h1" clickable="false" /></hierarchy>"#,
        )
        .unwrap();
        let r = mk_result(vec![
            step(0, r#"启动 ["https://example.com"]"#, Some("page/step_001.xml")),
            step(1, "点击 [{299, 242}]", Some("page/step_002.xml")),
        ]);
        let html = std::fs::read_to_string(write_report(&dir, &r).unwrap()).unwrap();
        assert!(html.contains("Learn more"), "应查前一步的页面");
        assert!(!html.contains("跳转后的新页面"), "不该拿本步(执行后)的页面来反查");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 命中多个时取最内层（body 包住一切，但点中的是链接）
    #[test]
    fn picks_innermost_element() {
        let dir = setup("inner");
        let r = mk_result(vec![
            step(0, "启动 [\"x\"]", Some("page/step_001.xml")),
            step(1, "点击 [{299, 242}]", None),
        ]);
        let html = std::fs::read_to_string(write_report(&dir, &r).unwrap()).unwrap();
        assert!(html.contains("Learn more"), "应取最小的那个元素，而不是 body");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 元素库脚本：标明走的是 .tklib，不去反查坐标
    #[test]
    fn marks_element_library_steps() {
        let dir = setup("lib");
        let r = mk_result(vec![step(0, "点击 [{登录按钮}]", None)]);
        let html = std::fs::read_to_string(write_report(&dir, &r).unwrap()).unwrap();
        assert!(html.contains("元素库"));
        assert!(html.contains("登录按钮"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// AI 找回的步骤要标明是 AI 认的，不能让人以为是脚本原本的定位
    #[test]
    fn marks_ai_healed_steps() {
        let dir = setup("heal");
        let mut s = step(0, "点击 [{300, 200}]", None);
        s.healed = Some("提交按钮".into());
        let html = std::fs::read_to_string(write_report(&dir, &mk_result(vec![s])).unwrap()).unwrap();
        assert!(html.contains("AI 识别"), "应标明是 AI 识别的:{}", html);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 元素文本里的 HTML 实体要正确还原并重新转义（xml 里存的是 &amp; &lt;）
    #[test]
    fn decodes_xml_entities_then_escapes_html() {
        let dir = setup("ent");
        let r = mk_result(vec![
            step(0, "启动 [\"x\"]", Some("page/step_001.xml")),
            step(1, "点击 [{600, 140}]", None), // 命中 h1
        ]);
        let html = std::fs::read_to_string(write_report(&dir, &r).unwrap()).unwrap();
        assert!(html.contains("Example &amp; &lt;Domain&gt;"), "应还原后重新转义:{}", html);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 失败信息必须出现（INV-9：失败要可见，不能只留个红叉）
    #[test]
    fn report_surfaces_failure() {
        let dir = setup("fail");
        let mut s = step(0, "点击 [{1, 1}]", None);
        s.success = false;
        s.error = Some("元素未找到 <script>".into());
        let mut r = mk_result(vec![s]);
        r.success = false;
        r.error = Some("第 1 步失败".into());
        let html = std::fs::read_to_string(write_report(&dir, &r).unwrap()).unwrap();
        assert!(html.contains("失败"));
        assert!(html.contains("第 1 步失败"));
        assert!(html.contains("&lt;script&gt;"), "报错里的标签要转义");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
