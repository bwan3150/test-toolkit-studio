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
//   - 截图 **base64 内嵌** → 单个 html 发给同事/贴进工单也能看图，这是人最需要的。
//     任务报告内嵌前先**缩放**（见 EMBED_MAX_WIDTH）：报告容器只有 880px，
//     内嵌更大的像素一个字都不会更清楚，只是撑体积
//   - 页面结构 xml **不内嵌**，只在顶部给文件链接 → xml 动辄几百 KB 且只有排障才逐行看
//
// 批次分隔行只在**有信息量**时才插（换设备 / 中间停了很久）。探索式使用会产生一长串
// "1 步"的批次，每批插一行的话，人看到的全是"AI 分几次调的"——那是工具的实现细节。
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

// ── 全流程汇总 ─────────────────────────────────────────────────────────

/// 报告头部要展示的"任务级"信息。**tke 自己给不出这些**——
/// 要验什么是用户说的，验没验成只有走完全程的调用方 AI 知道。
/// 都为空时报告只如实陈述步骤，不替谁下结论。
#[derive(Default)]
struct TaskMeta {
    task: Option<String>,
    verdict: Option<crate::Verdict>,
    summary: Option<String>,
}

/// 一次检查里的一个批次（一次 `tke steps` 调用留下的产物）
struct Batch {
    dir: PathBuf,
    /// 相对总报告的路径前缀，如 `steps_20260813-191740/` 或 `phone/steps_.../`
    prefix: String,
    result: ExecutionResult,
}

/// 递归找出 `root` 下所有批次（含 log.json 的目录），**按开始时间排序**。
///
/// 为什么递归：跨设备检查会分成 `web/` 与 `phone/` 两个子目录，汇总时按时间交错排开，
/// 正好还原「在平台上做 → 去手机上验」的真实顺序。
fn collect_batches(root: &Path, prefix: &str, depth: usize, out: &mut Vec<Batch>) {
    if depth > 3 {
        return; // 防止意外的深目录树把这里拖死
    }
    let Ok(entries) = std::fs::read_dir(root) else { return };
    let mut dirs: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    dirs.sort();
    for d in dirs {
        let Some(name) = d.file_name().and_then(|n| n.to_str()) else { continue };
        let child_prefix = format!("{}{}/", prefix, name);
        let log = d.join("log.json");
        if log.is_file() {
            if let Some(result) = std::fs::read_to_string(&log)
                .ok()
                .and_then(|t| serde_json::from_str::<ExecutionResult>(&t).ok())
            {
                out.push(Batch { dir: d.clone(), prefix: child_prefix.clone(), result });
                continue; // 批次目录内部不再往下找
            }
        }
        collect_batches(&d, &child_prefix, depth + 1, out);
    }
}

/// 把一个 log 根目录下的**所有批次**汇总成一份全流程报告 `<root>/report.html`。
///
/// 存在理由：AI 做一次检查要调很多次 `tke steps`（看页面→操作→再看→再操作），
/// 每次都留下一个独立目录和独立报告。人要审核时面对的是十几份碎报告，**没法读**。
/// 这份把它们按时间接成一条完整时间线。
///
/// `embed=false`（默认）图片走相对链接：重建快、文件小，报告和产物在同一棵树下照常显示。
/// `embed=true` 内嵌成单文件，适合发给别人。
pub fn write_session_report(root: &Path, embed: bool) -> Result<PathBuf> {
    let mut batches = Vec::new();
    collect_batches(root, "", 0, &mut batches);
    if batches.is_empty() {
        return Err(TkeError::InvalidArgument(format!(
            "{} 下没有找到任何检查记录（该目录里应有 steps_*/log.json）",
            root.display()
        )));
    }
    batches.sort_by(|a, b| a.result.start_time.cmp(&b.result.start_time));

    let html = render_session(&batches, embed);
    let out = root.join("report.html");
    std::fs::write(&out, html).map_err(TkeError::IoError)?;
    Ok(out)
}

/// 生成**任务报告** `<task_dir>/report.html`（Task 布局：`tke steps` 反复调用续写同一目录）。
///
/// 与 `write_session_report` 的区别：那份要去子目录里搜集碎批次、图片按相对路径引用；
/// 这份的产物全在同一层（`screenshots/` `pages/`），批次来自 `log.json` 的 `batches`，
/// 且**默认压缩内嵌**——人拿到的就是一个能直接转发的自包含文件。
///
/// `full_image=true` 用原图内嵌（体积大，逐像素复核时用）。
pub fn write_task_report(task_dir: &Path) -> Result<PathBuf> {
    write_task_report_with(task_dir, false)
}

pub fn write_task_report_with(task_dir: &Path, full_image: bool) -> Result<PathBuf> {
    let log = task_dir.join("log.json");
    let task = crate::TaskLog::load(&log);
    let meta = TaskMeta {
        task: task.task.clone(),
        verdict: task.verdict,
        summary: task.summary.clone(),
    };
    if task.batches.is_empty() {
        return Err(TkeError::InvalidArgument(format!(
            "{} 里没有检查记录（应有 log.json，且含 batches）",
            task_dir.display()
        )));
    }
    // 产物与 html 同层：prefix 为空；每批的 run_dir 都是任务目录本身
    let batches: Vec<Batch> = task
        .batches
        .into_iter()
        .map(|result| Batch { dir: task_dir.to_path_buf(), prefix: String::new(), result })
        .collect();

    let mode = if full_image { ImgMode::Embed } else { ImgMode::EmbedCompressed };
    let html = render_session_with(&batches, mode, false, &meta);
    let out = task_dir.join("report.html");
    std::fs::write(&out, html).map_err(TkeError::IoError)?;
    Ok(out)
}

/// 静默重建总报告——`tke steps` 每批跑完后顺手调用，AI 不必记得收尾。
/// 失败只 warn：证据本体已经落好了，汇总生不出来不该让整次运行失败。
pub fn refresh_session_report(run_dir: &Path) {
    let Some(root) = run_dir.parent() else { return };
    if let Err(e) = write_session_report(root, false) {
        tracing::debug!("全流程报告未更新: {}", e);
    }
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

/// 在页面结构里找**包含该坐标的最小元素**（最内层的那个才是真正被点到的）。
/// 新证据是 `pages/*.json`（元素表），老证据是 XML —— 两种都认。
fn hit_test_any(text: &str, x: i64, y: i64) -> Option<HitElement> {
    if text.trim_start().starts_with('[') {
        return hit_test_json(text, x, y);
    }
    hit_test_xml(text, x, y)
}

/// 元素表 JSON 版：直接反序列化，比抠 XML 属性稳
fn hit_test_json(json: &str, x: i64, y: i64) -> Option<HitElement> {
    let els: Vec<crate::UIElement> = serde_json::from_str(json).ok()?;
    els.iter()
        .filter(|e| {
            let b = &e.bounds;
            x >= b.x1 as i64 && x <= b.x2 as i64 && y >= b.y1 as i64 && y <= b.y2 as i64
        })
        .map(|e| HitElement {
            class: e.class_name.clone(),
            text: e.text.clone().unwrap_or_default(),
            desc: e.content_desc.clone().unwrap_or_default(),
            resource_id: e.resource_id.clone().unwrap_or_default(),
            xpath: e.xpath.clone().unwrap_or_default(),
            bounds: (e.bounds.x1 as i64, e.bounds.y1 as i64, e.bounds.x2 as i64, e.bounds.y2 as i64),
            clickable: e.clickable,
        })
        .min_by_key(|e| e.area())
}

fn hit_test_xml(xml: &str, x: i64, y: i64) -> Option<HitElement> {
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

/// 两份报告（单次 / 全流程）共用的样式——长得不一样会让人以为是两个工具出的
const BASE_CSS: &str = r##"*,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
:root{
  --bg:#f6f6f7; --card:#fff; --border:#e5e5e7; --txt:#1c1c1e; --txt2:#48484a; --txt3:#8e8e93;
  --ok:#1a7f37; --ok-bg:#f0fdf4; --ng:#b62324; --ng-bg:#fff0f0; --warn:#bf8600; --warn-bg:#fffbeb;
  --acc:#5856d6; --acc-bg:#f0f0ff;
  --mono:'SF Mono','Fira Code','Cascadia Code',Consolas,monospace;
}
@media (prefers-color-scheme:dark){
  :root{--bg:#1a1a1c;--card:#242426;--border:#38383a;--txt:#f2f2f7;--txt2:#c7c7cc;--txt3:#8e8e93;
        --ok-bg:#132b18;--ng-bg:#2d1416;--warn-bg:#2b2410;--acc-bg:#1e1e35}
}
body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI','PingFang SC','Hiragino Sans GB',
  'Microsoft YaHei',system-ui,sans-serif;background:var(--bg);color:var(--txt);
  font-size:14px;line-height:1.6;padding:24px 16px 64px}
.wrap{max-width:880px;margin:0 auto}
header{background:var(--card);border:1px solid var(--border);border-radius:12px;
  padding:18px 20px;margin-bottom:18px}
.h-top{display:flex;align-items:center;gap:12px;flex-wrap:wrap}
h1{font-size:17px;font-weight:650;flex:1;min-width:0;word-break:break-all}
.badge{padding:4px 14px;border-radius:99px;font-size:12px;font-weight:700;flex-shrink:0}
.b-ok{background:var(--ok);color:#fff} .b-ng{background:var(--ng);color:#fff}
.b-wa{background:var(--warn);color:#fff}
/* 没人给结论时的中性徽章：只说"跑完了"，不替谁判成败 */
.b-nu{background:var(--bg);color:var(--txt2);border:1px solid var(--border)}
/* 任务 / 结论：报告开头最该先看到的两行 */
.task{margin-top:10px;font-size:13px;line-height:1.5;color:var(--txt);
  display:flex;gap:10px;align-items:baseline}
.t-k{flex-shrink:0;color:var(--txt3);font-size:11px;padding:1px 7px;border-radius:4px;
  background:var(--bg);border:1px solid var(--border)}
/* 任务/结论卡片：报告里最该先看到的东西，给它独立的一块 */
.card{background:var(--card);border:1px solid var(--border);border-radius:10px;
  padding:16px 18px;margin-bottom:14px}
.c-blk+.c-blk{margin-top:14px;padding-top:14px;border-top:1px solid var(--border)}
.c-k{font-size:11px;color:var(--txt3);letter-spacing:.06em;margin-bottom:6px}
.c-v{font-size:13.5px;line-height:1.6;color:var(--txt)}
.t-md{min-width:0}
/* 表格：窄屏/长表横向滚动，别把整页撑破 */
.t-tw{overflow-x:auto;margin:8px 0}
.t-md table{border-collapse:collapse;font-size:12.5px;min-width:100%}
.t-md th,.t-md td{border:1px solid var(--border);padding:5px 10px;text-align:left;
  white-space:nowrap;vertical-align:top}
.t-md th{background:var(--bg);font-weight:650;color:var(--txt2)}
.t-md h4,.t-md h5{margin:10px 0 6px;font-size:13px;font-weight:650}
.t-md h5{font-size:12.5px;color:var(--txt2)}
.t-md p{margin:0 0 6px} .t-md p:last-child{margin-bottom:0}
.t-md ul,.t-md ol{margin:0 0 6px;padding-left:20px} .t-md li{margin:2px 0}
.t-md code{font-family:var(--mono);font-size:12px;padding:1px 5px;border-radius:4px;
  background:var(--bg);border:1px solid var(--border)}
.t-md strong{font-weight:650}
.chips{display:flex;gap:8px;flex-wrap:wrap;margin-top:12px}
.chip{padding:3px 10px;border-radius:6px;font-size:12px;font-family:var(--mono);
  background:var(--bg);border:1px solid var(--border);color:var(--txt2)}
.chip b{color:var(--txt);font-weight:700}
.c-ok b{color:var(--ok)} .c-ng b{color:var(--ng)} .c-warn b{color:var(--warn)}
.c-wa b{color:var(--warn)}
.meta{margin-top:12px;padding-top:12px;border-top:1px solid var(--border);
  display:grid;grid-template-columns:auto 1fr;gap:2px 14px;font-size:12px}
.m-row{display:contents}
.m-k{color:var(--txt3);white-space:nowrap}
.m-v{color:var(--txt2);font-family:var(--mono);word-break:break-all;font-size:11.5px}
.files{margin-top:14px;display:flex;gap:8px;flex-wrap:wrap}
.fbtn{display:inline-flex;align-items:center;gap:6px;padding:6px 12px;border-radius:7px;
  border:1px solid var(--border);background:var(--bg);color:var(--txt2);
  font-size:12px;text-decoration:none;white-space:nowrap;transition:all .12s}
.fbtn:hover{border-color:var(--acc);color:var(--acc);background:var(--acc-bg)}
.fbtn svg{width:13px;height:13px;flex-shrink:0;opacity:.75}
.err-top{margin-top:12px;padding:10px 12px;background:var(--ng-bg);border-radius:8px;
  font-size:12px;color:var(--ng);font-family:var(--mono);white-space:pre-wrap;word-break:break-all}
.step{background:var(--card);border:1px solid var(--border);border-radius:10px;
  overflow:hidden;margin-bottom:14px}
.step.ng{border-color:var(--ng)}
.s-hd{display:flex;align-items:center;gap:10px;padding:10px 14px;border-bottom:1px solid var(--border)}
.s-num{font-family:var(--mono);font-size:11px;color:var(--txt3);flex-shrink:0}
.s-mark{width:18px;height:18px;border-radius:50%;display:flex;align-items:center;
  justify-content:center;font-size:11px;font-weight:700;flex-shrink:0;color:#fff}
.m-ok{background:var(--ok)} .m-ng{background:var(--ng)}
.s-cmd{flex:1;min-width:0;font-family:var(--mono);font-size:12.5px;word-break:break-all}
.s-dur{font-family:var(--mono);font-size:11px;color:var(--txt3);flex-shrink:0}
.tgt{padding:8px 14px;background:var(--acc-bg);border-bottom:1px solid var(--border);font-size:12px}
.tgt summary{cursor:pointer;list-style:none;display:flex;align-items:center;gap:8px}
.tgt summary::-webkit-details-marker{display:none}
.tgt summary::before{content:'▸';color:var(--acc);font-size:10px;flex-shrink:0}
.tgt[open] summary::before{content:'▾'}
.t-src{font-family:var(--mono);font-size:10px;font-weight:700;color:#fff;background:var(--acc);
  padding:1px 6px;border-radius:3px;flex-shrink:0}
.t-lbl{font-weight:600;color:var(--txt);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.t-note{color:var(--txt3);font-size:11px;flex-shrink:0}
.t-body{margin-top:8px;display:grid;grid-template-columns:auto 1fr;gap:2px 12px;
  font-family:var(--mono);font-size:11px}
.t-k{color:var(--txt3)} .t-v{color:var(--txt2);word-break:break-all}
.say{padding:9px 14px;border-bottom:1px solid var(--border);font-size:13px;color:var(--txt);
  display:flex;gap:8px;align-items:baseline}
.say::before{content:'“';color:var(--acc);font-size:20px;line-height:0.6;flex-shrink:0}
.s-err{padding:10px 14px;background:var(--ng-bg);color:var(--ng);font-family:var(--mono);
  font-size:11.5px;white-space:pre-wrap;word-break:break-all;border-bottom:1px solid var(--border)}
.s-heal{padding:7px 14px;background:var(--warn-bg);color:var(--warn);font-size:11.5px;
  border-bottom:1px solid var(--border)}
.s-img{padding:14px;background:#141414;display:flex;justify-content:center}
/* 手机竖屏截图 1080×2412，按 max-width 铺开会有两三屏高——一步都看不完整。
   限到 56vh：一屏内能看见这一步的全貌，要看细节点一下展开成原始尺寸。
   纯 CSS 的 checkbox hack，**不引 JS**：报告要能离线看、内网看、转 PDF 看。 */
.s-img img{max-width:100%;max-height:56vh;width:auto;height:auto;border-radius:6px;display:block;cursor:zoom-in}
.s-zoom{display:none}
.s-zoom:checked + label img{max-height:none;cursor:zoom-out}
.s-foot{padding:6px 14px;font-size:11px;color:var(--txt3);font-family:var(--mono)}
.s-foot a{color:var(--acc);text-decoration:none}
"##;


impl Ctx<'_> {
    /// 这一步的「原始页面」相对路径（`raw_pages/step_003.json` 之类）。
    /// **按前缀扫**：扩展名随驱动而异（web=.html / 安卓与 iOS 真机=.xml / 模拟器=.json），
    /// 写死清单迟早漏一个，而漏了是静默的（P-43 就是这么来的）
    fn raw_page_rel(&self, seq: usize) -> Option<String> {
        let dir = self.run_dir.join("raw_pages");
        let prefix = format!("step_{:03}.", seq);
        std::fs::read_dir(&dir)
            .ok()?
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .find(|n| n.starts_with(&prefix))
            .map(|n| format!("raw_pages/{}", n))
    }
}

/// 图片怎么进报告：内嵌成 data URI，还是留一个相对链接。
///
/// 任务报告(Task 布局)默认 `EmbedCompressed`——**报告的用处就是发给人看**，
/// 一个自包含文件能直接转发/贴进工单，而原图内嵌 20 步就 5MB+、大到没法粘。
/// 旧的碎批次汇总仍默认 `Link`（那棵目录树里图片就在旁边，重建也快）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum ImgMode {
    /// 原图 base64 内嵌（保真，但一步就 ~270KB）
    Embed,
    /// **缩放 + JPEG 后**内嵌：一步 ~60KB，20 步的报告 5.4MB → 1.2MB。
    /// 按钮文字、输入框内容、标注横幅、元素框、点击点都还看得清；
    /// 极小图标细节会糊——要逐像素复核就用 `--full-image` 或直接看 screenshots/ 里的原图。
    EmbedCompressed,
    Link,
}

/// 内嵌图片的目标宽度。报告容器是 880px（见 CSS `.wrap`），图片最多显示到 ~850px——
/// 内嵌比这更大的像素**一个字都不会更清楚**，只是撑体积。960 给高 DPI 屏留了点余量。
const EMBED_MAX_WIDTH: u32 = 960;
/// 内嵌 JPEG 质量。注意：对 UI 截图，**光转 JPEG 几乎不省**（PNG 对大片纯色压得很好，
/// 而 JPEG 在文字锐边上还吃亏）——真正省体积的是上面那个缩放。
const EMBED_JPEG_QUALITY: u8 = 82;

/// 把截图压缩成适合内嵌的 JPEG。失败(解码不了/编码不了)返回 None，
/// 调用方回退到原图内嵌——**宁可报告大，也不能没有证据**。
fn compress_for_embed(bytes: &[u8]) -> Option<Vec<u8>> {
    let img = image::load_from_memory(bytes).ok()?;
    let img = if img.width() > EMBED_MAX_WIDTH {
        let h = (img.height() as f64 * EMBED_MAX_WIDTH as f64 / img.width() as f64).round() as u32;
        img.resize(EMBED_MAX_WIDTH, h.max(1), image::imageops::FilterType::Lanczos3)
    } else {
        img
    };
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, EMBED_JPEG_QUALITY)
        .encode_image(&img.to_rgb8())
        .ok()?;
    Some(out)
}

/// 渲染一份报告所需的上下文
struct Ctx<'a> {
    /// 这个 run 的产物目录（读图、读页面结构都从这里）
    run_dir: &'a Path,
    /// 相对 html 所在位置的前缀（全流程报告在父目录，要带上 `steps_xxx/`；单次报告为空）
    prefix: &'a str,
    img: ImgMode,
}

impl Ctx<'_> {
    /// html 里引用某个产物时该写的路径
    fn href(&self, rel: &str) -> String {
        format!("{}{}", self.prefix, rel)
    }

    fn img_src(&self, rel: &str) -> Option<String> {
        match self.img {
            ImgMode::Link => std::fs::metadata(self.run_dir.join(rel))
                .is_ok()
                .then(|| self.href(rel)),
            ImgMode::Embed => {
                let bytes = std::fs::read(self.run_dir.join(rel)).ok()?;
                Some(format!(
                    "data:image/png;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(bytes)
                ))
            }
            ImgMode::EmbedCompressed => {
                let bytes = std::fs::read(self.run_dir.join(rel)).ok()?;
                match compress_for_embed(&bytes) {
                    Some(jpg) => Some(format!(
                        "data:image/jpeg;base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(jpg)
                    )),
                    // 压不了就用原图——证据不能因为压缩失败而消失
                    None => Some(format!(
                        "data:image/png;base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(bytes)
                    )),
                }
            }
        }
    }
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

/// 两批之间空了多久才值得在报告里标一行。低于这个值就是 AI 正常的"想一下再走下一步"，
/// 标出来只会变成噪音；超过它多半是**人在中间做了什么**（手动登录、去后台改配置）——
/// 那件事对读报告的人很重要，而它在证据里没有任何痕迹，只剩这个时间空档。
const IDLE_GAP_SECS: i64 = 60;

fn gap_secs(prev_end: &str, next_start: &str) -> Option<i64> {
    ms_between(prev_end, next_start).map(|ms| ms / 1000)
}

fn human_gap(secs: i64) -> String {
    if secs >= 3600 {
        format!("{}h{}m", secs / 3600, (secs % 3600) / 60)
    } else if secs >= 60 {
        format!("{} 分钟", secs / 60)
    } else {
        format!("{} 秒", secs)
    }
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

    let ctx = Ctx { run_dir, prefix: "", img: ImgMode::Embed };
    let steps_html: String = r
        .steps
        .iter()
        .enumerate()
        .map(|(i, s)| step_card(&ctx, r, i, s, plat))
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
        ("设备", r.device_label.clone().or_else(|| r.device.clone()).unwrap_or_else(|| "—".into())),
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
{css}
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
        css = BASE_CSS,
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
    ctx: &Ctx,
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
    let Some(xml) = page_before(ctx.run_dir, &r.steps, i) else {
        return format!(
            r#"<div class="tgt"><summary><span class="t-src">坐标</span>
      <span class="t-note">({}, {}) · 无前置页面结构，查不到点中的元素</span></summary></div>"#,
            x, y
        );
    };

    let Some(el) = hit_test_any(&xml, x, y) else {
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

/// 全流程报告：所有批次接成一条时间线
fn render_session(batches: &[Batch], embed: bool) -> String {
    render_session_with(
        batches,
        if embed { ImgMode::Embed } else { ImgMode::Link },
        true,
        &TaskMeta::default(),
    )
}

/// `batch_links=true` 时每批带「单独看这一批」链接（碎批次布局才有单批报告；
/// Task 布局下产物全在一层、没有单批报告，带了就是死链）。
fn render_session_with(batches: &[Batch], img: ImgMode, batch_links: bool, meta: &TaskMeta) -> String {
    let all_steps: usize = batches.iter().map(|b| b.result.steps.len()).sum();
    let passed: usize = batches
        .iter()
        .map(|b| b.result.steps.iter().filter(|s| s.success).count())
        .sum();
    let failed = all_steps - passed;
    let first = &batches[0].result;
    let last = &batches[batches.len() - 1].result;

    // 任务目录：旧的碎批次布局里批次在子目录中（要往上一层才是任务目录）；
    // Task 布局下产物与报告同层，`dir` 就是任务目录本身——照旧取 parent 会拿到
    // `~/.tke/logs`，于是报告标题变成 "logs"、"打开检查目录"也跳错地方。
    let task_dir: &Path = if batches[0].prefix.is_empty() {
        &batches[0].dir
    } else {
        batches[0].dir.parent().unwrap_or(&batches[0].dir)
    };

    // 标题用任务目录名（`~/.tke/logs/<任务简称>/` 里那个简称），比脚本名更像"这次检查"
    let title = task_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&first.script_name)
        .to_string();

    let mut body = String::new();
    let mut seq = 0usize;   // 跨批次连续编号
    let mut prev_dev: Option<String> = None;
    let mut prev_end: Option<String> = None;
    for (i, b) in batches.iter().enumerate() {
        let plat = platform_tag(b.result.device.as_deref());
        let ctx = Ctx { run_dir: &b.dir, prefix: &b.prefix, img };
        // ⚠️ **显示用名字，判断用 ID**，两者不能混：
        // 两台同型号的模拟器 label 一模一样（都叫 `iPhone 17 Pro · iOS 26.0（模拟器）`），
        // 拿 label 判"换没换设备"就会把跨设备那一跳吞掉——而那正是跨设备检查
        // 最需要在报告里还原的东西（单测 task_report_separator_only_when_meaningful 钉着）
        let dev_key = b.result.device.clone().unwrap_or_default();
        let dev = b.result.device_label.clone()
            .or_else(|| b.result.device.clone())
            .unwrap_or_else(|| "—".into());
        let link = if batch_links {
            format!(
                r#"<a class="b-link" href="{href}">单独看这一批</a>"#,
                href = esc(&format!("{}report.html", b.prefix)),
            )
        } else {
            String::new()
        };
        // 分隔行只在**有信息量**时才插。探索式使用(看一步、想一想、再走一步)会产生一长串
        // "1 步"的批次，每批都插一行分隔的话，读报告的人看到的全是"AI 分几次调的"——
        // 那是工具的实现细节，不是这次检查发生了什么。
        // 真正值得标出来的只有两件事：**换设备了**、**中间停了很久**（多半是人在手动登录/操作）。
        // 第一批不算"换设备"——顶部摘要已经写了设备，再插一行是重复
        let dev_changed = prev_dev.is_some() && prev_dev.as_deref() != Some(dev_key.as_str());
        let gap = prev_end
            .as_deref()
            .and_then(|p| gap_secs(p, &b.result.start_time))
            .filter(|s| *s >= IDLE_GAP_SECS);
        if batch_links || dev_changed || gap.is_some() {
            let gap_note = gap
                .map(|s| format!(r#"<span class="b-gap">间隔 {}</span>"#, human_gap(s)))
                .unwrap_or_default();
            body.push_str(&format!(
                r#"<div class="batch">
  <span class="b-no">{n}</span>
  <span class="b-dev">{dev}</span>
  <span class="b-time">{time}</span>
  {gap_note}
  {link}
</div>"#,
                n = i + 1,
                dev = esc(&dev),
                time = hhmmss(&b.result.start_time),
                gap_note = gap_note,
                link = link,
            ));
        }
        prev_dev = Some(dev_key.clone());
        prev_end = Some(if b.result.end_time.is_empty() {
            b.result.start_time.clone()
        } else {
            b.result.end_time.clone()
        });
        for (j, st) in b.result.steps.iter().enumerate() {
            seq += 1;
            body.push_str(&step_card_seq(&ctx, &b.result, j, st, plat, Some(seq)));
            body.push('\n');
        }
    }

    // 设备去重（跨设备检查里这一行能一眼看出涉及了哪几台）。
    // **显示给人看的名字**——`sim:92AA7443-4027-4CAA-A5F6-543EA24FB3F3` 对读报告的人
    // 没有任何意义（下面的分隔行早就用 label 了，这里漏了就成了同一份报告里两套叫法）
    let mut devs: Vec<String> = batches
        .iter()
        .filter_map(|b| b.result.device_label.clone().or_else(|| b.result.device.clone()))
        .collect();
    devs.dedup();
    devs.sort();
    devs.dedup();

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} · 全流程检查报告</title>
<style>{css}
.batch{{display:flex;align-items:center;gap:10px;margin:26px 0 12px;padding:7px 12px;
  background:var(--acc-bg);border-radius:8px;font-size:12px;color:var(--txt2)}}
.b-no{{display:inline-flex;align-items:center;justify-content:center;width:20px;height:20px;
  border-radius:50%;background:var(--acc);color:#fff;font-size:11px;font-weight:700;flex-shrink:0}}
.b-dev{{font-family:var(--mono);font-weight:600;color:var(--txt)}}
.b-time,.b-steps{{font-family:var(--mono);color:var(--txt3)}}
.b-gap{{font-family:var(--mono);color:var(--warn);background:var(--warn-bg);
  padding:1px 7px;border-radius:99px;font-size:11px}}
.b-link{{margin-left:auto;color:var(--acc);text-decoration:none;font-size:11px}}
.b-link:hover{{text-decoration:underline}}
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
    <span class="chip c-ok"><b>{passed}</b> 步成功</span>
    {failed_chip}
    <span class="chip">共 <b>{all_steps}</b> 步 / <b>{nb}</b> 批操作</span>
    <span class="chip">耗时 <b>{dur}</b></span>
  </div>
  <div class="meta">
    <div class="m-row"><span class="m-k">设备</span><span class="m-v">{devs}</span></div>
    <div class="m-row"><span class="m-k">开始</span><span class="m-v">{start}</span></div>
    <div class="m-row"><span class="m-k">结束</span><span class="m-v">{end}</span></div>
    <div class="m-row"><span class="m-k">目录</span><span class="m-v">{dir}</span></div>
  </div>
  <div class="files">
    <a class="fbtn" href="{file_dir}">打开检查目录</a>
  </div>
</header>
{task_card}
{body}
</div>
</body>
</html>"#,
        title = esc(&title),
        css = BASE_CSS,
        // 结论**只认调用方给的**。步骤里有没有"没点中"跟任务成没成是两回事——
        // 定位失败换个方式点中了就没事，而功能是否真的可用只有走完全程的人/AI 知道。
        // 没人给结论时不下判断，只说"已完成"（此前一步没命中就整份标"失败"，是在撒谎）。
        badge_cls = meta.verdict.map(|v| v.badge().0).unwrap_or("b-nu"),
        badge_txt = meta.verdict.map(|v| v.badge().1).unwrap_or("已完成"),
        // 任务与结论**独立成卡片**放在最前面：人打开报告第一眼要看的就是
        // "要验什么"和"结论是什么"，把它们挤在标题行旁边等于藏起来
        task_card = {
            if meta.task.is_none() && meta.summary.is_none() {
                String::new()
            } else {
                let mut s = String::from(r#"<section class="card">"#);
                if let Some(task) = &meta.task {
                    s.push_str(&format!(
                        r#"<div class="c-blk"><div class="c-k">任务</div><div class="c-v">{}</div></div>"#,
                        esc(task)
                    ));
                }
                if let Some(sum) = &meta.summary {
                    // 总结按 **Markdown** 渲染：AI 写表格/列表/多段是常态，塞进一行读不了。
                    // 在 Rust 侧转好（不往报告里塞 JS——它得离线、内网、转 PDF 都能看）
                    s.push_str(&format!(
                        r#"<div class="c-blk"><div class="c-k">结论</div><div class="c-v t-md">{}</div></div>"#,
                        crate::workflow::markdown::to_html(sum)
                    ));
                }
                s.push_str("</section>");
                s
            }
        },
        passed = passed,
        all_steps = all_steps,
        nb = batches.len(),
        failed_chip = if failed > 0 {
            // 步骤级只说"没成"：它可能只是定位没命中，后面换个方式就点中了。
            // 叫"失败"会让人以为整件事砸了——用户实测撞到过（7 步里 1 步没命中，
            // 第 6 步用坐标点回来了，报告顶上却写着"失败"）
            format!(r#"<span class="chip c-wa"><b>{}</b> 步未成</span>"#, failed)
        } else {
            String::new()
        },
        dur = ms_between(&first.start_time, &last.end_time).map(fmt_dur).unwrap_or_else(|| "—".into()),
        devs = esc(&if devs.is_empty() { "—".to_string() } else { devs.join(" · ") }),
        start = hhmmss(&first.start_time),
        end = hhmmss(&last.end_time),
        dir = esc(&task_dir.display().to_string()),
        file_dir = esc(&format!("file://{}", task_dir.display())),
        body = body,
    )
}

fn step_card(ctx: &Ctx, r: &ExecutionResult, i: usize, s: &StepResult, plat: &str) -> String {
    step_card_seq(ctx, r, i, s, plat, None)
}

/// `seq` 给全流程报告用：跨批次连续编号。单批报告传 None，用步骤自己的序号。
fn step_card_seq(
    ctx: &Ctx,
    r: &ExecutionResult,
    i: usize,
    s: &StepResult,
    plat: &str,
    seq: Option<usize>,
) -> String {
    // 内嵌的是缩过的图，默认限高 56vh（手机竖屏图不限高会占两三屏）。
    // 点一下**就地展开**成原始尺寸；要看真正的原图/原始页面，走下面那行链接。
    // id 用步骤序号：同一份报告里几十张图，checkbox 的 id 不能撞
    let img = s
        .screenshot
        .as_deref()
        .and_then(|rel| ctx.img_src(rel).map(|uri| (rel, uri)))
        .map(|(_rel, uri)| {
            let id = seq.unwrap_or(s.index);
            format!(
                r#"<div class="s-img"><input type="checkbox" class="s-zoom" id="z{id}"><label for="z{id}"><img src="{uri}" alt="step" title="点击放大"></label></div>"#,
                id = id,
                uri = uri,
            )
        })
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
  {dlg}
  {errs}
  {err}
  {img}
  {foot}
</div>"#,
        ng = if s.success { "" } else { "ng" },
        num = seq.unwrap_or(s.index + 1),
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
        tgt = target_block(ctx, r, i, s, plat),
        heal = match &s.healed {
            Some(name) => format!(
                r#"<div class="s-heal">⚠ 元素「{}」按原定位没找到，由 AI 依当前页面找回（脚本定位可能该更新了）</div>"#,
                esc(name)
            ),
            None => String::new(),
        },
        // 页面自己报的错：截图和页面结构里都没有它，报告不说就等于没发生过
        errs = if s.errors.is_empty() { String::new() } else {
            s.errors.iter()
                .map(|e| format!(r#"<div class="s-err">⚠ 页面报错：{}</div>"#, esc(e)))
                .collect::<Vec<_>>().join("")
        },
        // 原生对话框：浏览器自己画的，**截图里也拍不到**，报告不说就没人知道它出现过
        dlg = match &s.dialog {
            Some(d) => format!(
                r#"<div class="s-heal">⚠ 弹出原生对话框：「{}」（浏览器绘制，截图与页面结构中均无此内容）</div>"#,
                esc(d)
            ),
            None => String::new(),
        },
        err = match &s.error {
            Some(e) if !e.is_empty() => format!(r#"<div class="s-err">{}</div>"#, esc(e)),
            _ => String::new(),
        },
        img = img,
        // 图片下面这行才是"去看原件"的入口：原图（未缩放）、元素表、**驱动直给的原始页面**。
        // 名字用中文而不是路径——路径长且每步都一样，读的人要的是"点哪个能看到什么"
        foot = {
            let link = |rel: &str, name: &str| {
                format!(r#"<a href="{}" title="{}">{}</a>"#, esc(&ctx.href(rel)), esc(rel), name)
            };
            let mut parts: Vec<String> = Vec::new();
            if let Some(p) = &s.screenshot {
                parts.push(link(p, "原图"));
            }
            if let Some(x) = &s.xml {
                parts.push(link(x, "元素表"));
            }
            // 原始页面（DOM / uiautomator / XCUI / AX 原文）——**按前缀找**，
            // 扩展名随驱动而异（html/xml/json），写死清单迟早漏（P-43）
            if let Some(raw) = seq.and_then(|n| ctx.raw_page_rel(n)) {
                parts.push(link(&raw, "原始页面"));
            }
            if parts.is_empty() {
                String::new()
            } else {
                format!(r#"<div class="s-foot">{}</div>"#, parts.join(" · "))
            }
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
            device_label: Some("Chrome（无头）".into()),
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
            dialog: None,
            errors: vec![],
        }
    }

    fn setup(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tke-report-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("page")).unwrap();
        std::fs::write(d.join("page/step_001.xml"), PAGE).unwrap();
        d
    }

    /// **一步没命中 ≠ 任务失败**（用户实测撞到过：7 步里 1 步定位没中、第 6 步用坐标点
    /// 回来了，报告顶上却写着"失败"）。没人给结论时报告只说"已完成"，不替谁判成败。
    #[test]
    fn one_missed_step_does_not_fail_the_task() {
        let dir = std::env::temp_dir().join(format!("tke-report-verdict-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let write = |t: &crate::TaskLog| {
            std::fs::write(dir.join("log.json"), serde_json::to_string(t).unwrap()).unwrap();
            std::fs::read_to_string(write_task_report(&dir).unwrap()).unwrap()
        };

        let mut ok = mk_result(vec![step(0, "点击 [\"A\"]", None)]);
        ok.device = Some("web".into());
        let mut bad = mk_result(vec![{
            let mut s = step(0, "点击 [\"没有的东西\"]", None);
            s.success = false;
            s.error = Some("元素未找到".into());
            s
        }]);
        bad.success = false;
        bad.device = Some("web".into());

        // ① 没给结论：不许出现"失败"字样的徽章，只说"已完成"
        let html = write(&crate::TaskLog { batches: vec![bad, ok], ..Default::default() });
        assert!(html.contains("已完成"), "没人下结论时该是中性措辞:{}", &html[..400.min(html.len())]);
        assert!(html.contains("步未成"), "步骤级要如实说有一步没成");
        assert!(!html.contains(r#"badge b-ng"#), "不该自作主张判失败");

        // ② 调用方说通过：徽章就是通过，哪怕中间有一步没命中
        let mut t = crate::TaskLog::load(&dir.join("log.json"));
        t.verdict = Some(crate::Verdict::Pass);
        t.task = Some("验证播放器面板".into());
        t.summary = Some("换个方式点中了，功能正常".into());
        let html = write(&t);
        assert!(html.contains("badge b-ok"), "调用方说 pass 就是 pass");
        assert!(html.contains("验证播放器面板"), "报告开头要写明这次验的是什么");
        assert!(html.contains("换个方式点中了"), "结论说明要显示出来");

        // ③ 只有被测对象真有问题才是"有问题"
        let mut t = crate::TaskLog::load(&dir.join("log.json"));
        t.verdict = Some(crate::Verdict::Fail);
        let html = write(&t);
        assert!(html.contains("有问题"), "fail 才判被测对象有问题");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 任务报告的批次分隔行：**只在有信息量时才插**。
    /// 探索式使用(看一步、想一想、再走一步)会产生一长串"1 步"的批次——每批插一行的话，
    /// 人看到的全是"AI 分几次调的"，那是工具的实现细节，不是这次检查发生了什么。
    #[test]
    fn task_report_separator_only_when_meaningful() {
        // ⚠️ label 故意**全都一样**：换设备的判断必须看 `device` 那个 ID，
        // 不能看显示名——两台同型号模拟器的名字一模一样，混用就会把跨设备那一跳吞掉
        let mk = |dev: &str, start: &str, end: &str| {
            let mut r = mk_result(vec![step(0, "点击 [\"X\"]", None)]);
            r.device = Some(dev.into());
            r.device_label = Some("同一个显示名".into());
            r.start_time = start.into();
            r.end_time = end.into();
            r
        };
        let render = |batches: Vec<ExecutionResult>, tag: &str| {
            let dir = std::env::temp_dir()
                .join(format!("tke-report-sep-{}-{}", tag, std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("log.json"),
                serde_json::to_string(&crate::TaskLog { batches, ..Default::default() }).unwrap(),
            )
            .unwrap();
            let html =
                std::fs::read_to_string(write_task_report(&dir).unwrap()).unwrap();
            let _ = std::fs::remove_dir_all(&dir);
            html
        };

        // ① 同设备、连着跑：一行都不该有（探索式的常态）
        let html = render(
            vec![
                mk("web", "2026-08-15T10:00:00+10:00", "2026-08-15T10:00:05+10:00"),
                mk("web", "2026-08-15T10:00:07+10:00", "2026-08-15T10:00:09+10:00"),
                mk("web", "2026-08-15T10:00:11+10:00", "2026-08-15T10:00:13+10:00"),
            ],
            "quiet",
        );
        assert_eq!(
            html.matches(r#"<div class="batch">"#).count(),
            0,
            "同设备连续批次不该插分隔行"
        );

        // ② 换设备：要标出来——这正是跨设备检查要还原的因果
        let html = render(
            vec![
                mk("web", "2026-08-15T10:00:00+10:00", "2026-08-15T10:00:05+10:00"),
                mk("phone-1", "2026-08-15T10:00:07+10:00", "2026-08-15T10:00:09+10:00"),
            ],
            "dev",
        );
        assert_eq!(html.matches(r#"<div class="batch">"#).count(), 1);
assert!(
            html.contains("同一个显示名"),
            "分隔行写的是**给人看的设备名**"
        );

        // ③ 中间停了很久：多半是人在手动登录/改配置，那件事在证据里只剩这个空档
        let html = render(
            vec![
                mk("web", "2026-08-15T10:00:00+10:00", "2026-08-15T10:00:05+10:00"),
                mk("web", "2026-08-15T10:06:05+10:00", "2026-08-15T10:06:09+10:00"),
            ],
            "gap",
        );
        assert_eq!(html.matches(r#"<div class="batch">"#).count(), 1);
        assert!(html.contains("间隔 6 分钟"), "应标出空档时长:{}", html);
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

    /// 造一个批次目录（含 log.json）
    fn mk_batch(root: &std::path::Path, name: &str, start: &str, cmds: &[&str]) {
        let d = root.join(name);
        std::fs::create_dir_all(&d).unwrap();
        let steps: Vec<StepResult> = cmds
            .iter()
            .enumerate()
            .map(|(i, c)| step(i, c, None))
            .collect();
        let mut r = mk_result(steps);
        r.start_time = start.to_string();
        r.end_time = start.to_string();
        std::fs::write(d.join("log.json"), serde_json::to_string(&r).unwrap()).unwrap();
    }

    /// 全流程汇总：多批产物接成一条时间线（一次检查会调很多次 steps，碎报告没法审）
    #[test]
    fn session_report_merges_all_batches() {
        let root = std::env::temp_dir().join(format!("tke-session-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        mk_batch(&root, "steps_20260813-100000", "2026-08-13T10:00:00+10:00", &["点击 [{1, 1}]"]);
        mk_batch(&root, "steps_20260813-100500", "2026-08-13T10:05:00+10:00", &["返回", "等待 [1s]"]);

        let out = write_session_report(&root, false).unwrap();
        let html = std::fs::read_to_string(&out).unwrap();
        assert_eq!(out, root.join("report.html"));
        assert_eq!(html.matches(r#"class="batch""#).count(), 2, "两批都要在");
        assert!(html.contains("共 <b>3</b> 步"), "步数要跨批累计:{}", html);
        assert!(html.contains("steps_20260813-100000/report.html"), "要能跳回单批报告");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **按时间排序**，不是按目录名——跨设备时两个子目录交错，顺序错了就读不出因果
    #[test]
    fn session_report_orders_by_time_across_subdirs() {
        let root = std::env::temp_dir().join(format!("tke-session-ord-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        // phone 的批次时间更早，但目录名排在 web 后面
        mk_batch(&root.join("web"), "steps_b", "2026-08-13T10:05:00+10:00", &["点击 [{2, 2}]"]);
        mk_batch(&root.join("phone"), "steps_a", "2026-08-13T10:00:00+10:00", &["返回"]);

        let html = std::fs::read_to_string(write_session_report(&root, false).unwrap()).unwrap();
        let phone_at = html.find("phone/steps_a").expect("应含 phone 批次");
        let web_at = html.find("web/steps_b").expect("应含 web 批次");
        assert!(phone_at < web_at, "时间早的要排前面(跨子目录也按时间)");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 全流程报告的步骤要**跨批次连续编号**——每批各自从 1 开始的话，
    /// 读起来像好几段互不相干的测试拼在一起（用户实测反馈）
    #[test]
    fn session_report_numbers_steps_continuously() {
        let root = std::env::temp_dir().join(format!("tke-session-seq-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        mk_batch(&root, "steps_a", "2026-08-14T10:00:00+10:00", &["返回", "等待 [1s]"]);
        mk_batch(&root, "steps_b", "2026-08-14T10:05:00+10:00", &["返回", "等待 [1s]"]);

        let html = std::fs::read_to_string(write_session_report(&root, false).unwrap()).unwrap();
        let nums: Vec<&str> = html
            .split(r#"class="s-num">"#)
            .skip(1)
            .filter_map(|x| x.split('<').next())
            .collect();
        assert_eq!(nums, vec!["01", "02", "03", "04"], "四步应连续编号，而不是 01 02 01 02");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 空目录不该产出一份骗人的空报告
    #[test]
    fn session_report_refuses_empty_dir() {
        let root = std::env::temp_dir().join(format!("tke-session-empty-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        assert!(write_session_report(&root, false).is_err(), "没有记录就该报错");
        let _ = std::fs::remove_dir_all(&root);
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
